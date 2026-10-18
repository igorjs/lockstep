// SPDX-License-Identifier: Apache-2.0
//! Waves: a large arrival spaced into pulses, paced by game minutes and held back by a budget.

use lockstep_core::math::Fixed32;
use lockstep_core::Message;
use serde::{Deserialize, Serialize};

/// How a wave is paced, as data. A pulse of `pulse_size` arrivals falls due every
/// `interval_minutes` game minutes, the first after `start_after_minutes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveRules {
    /// Arrivals in one pulse. At least one.
    pub pulse_size: u32,
    /// Game minutes between pulses. At least one.
    pub interval_minutes: u32,
    pub start_after_minutes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum WaveEvent {
    /// A pulse fell due. The last pulse may be smaller than the rest.
    Pulse { index: u32, arrivals: u32 },
    /// Arrivals were admitted this step.
    Admitted { arrivals: u32 },
    /// The budget kept arrivals waiting. Sent once per hold, when it starts.
    Held { waiting: u32 },
    /// Every arrival of the wave has been admitted.
    Finished,
}

/// One wave in progress: where it is in its total, and the game time since it began.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Waves {
    total: u32,
    admitted: u32,
    pulses_due: u32,
    /// Raw 16.16 game minutes since the wave began, so no pulse time drifts.
    elapsed: i64,
    holding: bool,
    finished: bool,
}

impl Waves {
    pub fn new(total: u32) -> Self {
        Waves {
            total,
            admitted: 0,
            pulses_due: 0,
            elapsed: 0,
            holding: false,
            finished: total == 0,
        }
    }

    pub fn total(&self) -> u32 {
        self.total
    }

    pub fn admitted(&self) -> u32 {
        self.admitted
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Arrivals that have fallen due and are not yet admitted.
    pub fn waiting(&self, rules: &WaveRules) -> u32 {
        self.due(rules).saturating_sub(self.admitted)
    }

    fn due(&self, rules: &WaveRules) -> u32 {
        let size = rules.pulse_size.max(1) as u64;
        (self.pulses_due as u64 * size).min(self.total as u64) as u32
    }

    /// Lets `minutes` of game time pass and admits as many due arrivals as `headroom` allows
    /// (see `Director::headroom`). Returns the arrivals admitted. The same game time gives the
    /// same pulses at any step rate; the budget only delays arrivals, never drops them.
    pub fn advance(
        &mut self,
        rules: &WaveRules,
        minutes: Fixed32,
        headroom: u32,
        events: &mut Vec<WaveEvent>,
    ) -> u32 {
        if self.finished {
            return 0;
        }
        self.elapsed += minutes.raw().max(0) as i64;
        let size = rules.pulse_size.max(1);
        let pulses_in_total = self.total.div_ceil(size);
        let start = rules.start_after_minutes as i64 * 65_536;
        if self.elapsed >= start {
            let interval = rules.interval_minutes.max(1) as i64 * 65_536;
            let reached = ((self.elapsed - start) / interval + 1).min(pulses_in_total as i64);
            while (self.pulses_due as i64) < reached {
                let index = self.pulses_due;
                self.pulses_due += 1;
                let arrivals = self.due(rules) - (index * size).min(self.total);
                events.push(WaveEvent::Pulse { index, arrivals });
            }
        }
        let waiting = self.due(rules).saturating_sub(self.admitted);
        let admitting = waiting.min(headroom);
        if admitting > 0 {
            self.admitted += admitting;
            events.push(WaveEvent::Admitted {
                arrivals: admitting,
            });
        }
        let still_waiting = waiting - admitting;
        if still_waiting > 0 && !self.holding {
            events.push(WaveEvent::Held {
                waiting: still_waiting,
            });
        }
        self.holding = still_waiting > 0;
        if self.admitted == self.total {
            self.finished = true;
            events.push(WaveEvent::Finished);
        }
        admitting
    }
}
