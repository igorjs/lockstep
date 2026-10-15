// SPDX-License-Identifier: Apache-2.0
use crate::graph::{RegionId, Regions};
use lockstep_core::math::{unit, Fixed32, Turn};
use lockstep_core::{Message, Streams};
use serde::{Deserialize, Serialize};

/// How a condition spreads. Draws come from the `"spread"` stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadRules {
    /// The chance in a thousand, each interval, that an active region starts one clear neighbour
    /// in still air, at full pace, in a fully receptive region.
    pub base_permille: u32,
    /// Game minutes between tries. At least one.
    pub interval_minutes: u32,
    /// Intervals a region stays active before it is spent.
    pub active_intervals: u32,
    /// Percent added to the chance for each metre a second of wind blowing straight from the
    /// active region toward its neighbour, and taken off for wind blowing the other way.
    pub wind_percent_per_metre: u32,
}

/// The wind, as the spread needs it. `lockstep-agents` weather makes one with `wind()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breeze {
    /// The way the wind blows toward, 0 east, counter-clockwise.
    pub direction: Turn,
    pub strength: Fixed32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    Clear,
    /// Still spreading, for this many more intervals.
    Active {
        intervals_left: u32,
    },
    /// Over: it neither spreads nor takes hold again.
    Spent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum SpreadEvent {
    /// A region took hold, from a neighbour or from outside.
    Started {
        region: RegionId,
        from: Option<RegionId>,
    },
    Spent {
        region: RegionId,
    },
}

/// The condition of every region, and the part of an interval not yet passed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Spread {
    conditions: Vec<Condition>,
    /// Raw 16.16 game minutes not yet a whole interval.
    carried: i64,
}

impl Spread {
    pub fn new(regions: &Regions) -> Self {
        Spread {
            conditions: vec![Condition::Clear; regions.len()],
            carried: 0,
        }
    }

    pub fn condition(&self, region: RegionId) -> Option<Condition> {
        self.conditions.get(region.0 as usize).copied()
    }

    pub fn is_active(&self, region: RegionId) -> bool {
        matches!(self.condition(region), Some(Condition::Active { .. }))
    }

    pub fn active_count(&self) -> usize {
        self.conditions
            .iter()
            .filter(|condition| matches!(condition, Condition::Active { .. }))
            .count()
    }

    pub fn spent_count(&self) -> usize {
        self.conditions
            .iter()
            .filter(|condition| matches!(condition, Condition::Spent))
            .count()
    }

    /// Starts a clear region from outside, for the full active time. Returns whether it started:
    /// an unknown, active or spent region is left alone.
    pub fn start(
        &mut self,
        region: RegionId,
        rules: &SpreadRules,
        events: &mut Vec<SpreadEvent>,
    ) -> bool {
        let Some(condition) = self.conditions.get_mut(region.0 as usize) else {
            return false;
        };
        if *condition != Condition::Clear {
            return false;
        }
        *condition = Condition::Active {
            intervals_left: rules.active_intervals,
        };
        events.push(SpreadEvent::Started { region, from: None });
        true
    }

    /// Lets `minutes` of game time pass, an interval at a time. In each interval every region
    /// that was active when it began tries each clear neighbour, in order, once; one that takes
    /// hold becomes active for the next interval. `pace_percent` (0 to 100) scales every chance,
    /// so a season can slow or stop the spread. Then each region active at the start of the
    /// interval has one interval less left and becomes spent at none.
    #[allow(clippy::too_many_arguments)]
    pub fn advance(
        &mut self,
        regions: &Regions,
        rules: &SpreadRules,
        breeze: Breeze,
        pace_percent: u32,
        minutes: Fixed32,
        streams: &mut Streams,
        events: &mut Vec<SpreadEvent>,
    ) {
        // A spread made for other regions has nothing to spread across.
        if regions.len() != self.conditions.len() {
            return;
        }
        let interval = rules.interval_minutes.max(1) as i64 * 65_536;
        self.carried += minutes.raw().max(0) as i64;
        while self.carried >= interval {
            self.carried -= interval;
            self.interval(
                regions,
                rules,
                breeze,
                pace_percent.min(100),
                streams,
                events,
            );
        }
    }

    fn interval(
        &mut self,
        regions: &Regions,
        rules: &SpreadRules,
        breeze: Breeze,
        pace_percent: u32,
        streams: &mut Streams,
        events: &mut Vec<SpreadEvent>,
    ) {
        let before = self.conditions.clone();
        for from in regions.ids() {
            if !matches!(before[from.0 as usize], Condition::Active { .. }) {
                continue;
            }
            for &to in regions.neighbours(from) {
                if self.conditions[to.0 as usize] != Condition::Clear {
                    continue;
                }
                let chance = chance_permille(regions, rules, breeze, pace_percent, from, to);
                if streams.range("spread", 0, 1_000) < chance {
                    self.conditions[to.0 as usize] = Condition::Active {
                        intervals_left: rules.active_intervals,
                    };
                    events.push(SpreadEvent::Started {
                        region: to,
                        from: Some(from),
                    });
                }
            }
        }
        for region in regions.ids() {
            if !matches!(before[region.0 as usize], Condition::Active { .. }) {
                continue;
            }
            if let Condition::Active { intervals_left } = &mut self.conditions[region.0 as usize] {
                *intervals_left = intervals_left.saturating_sub(1);
                if *intervals_left == 0 {
                    self.conditions[region.0 as usize] = Condition::Spent;
                    events.push(SpreadEvent::Spent { region });
                }
            }
        }
    }
}

/// The chance in a thousand that `from` starts `to` this interval.
fn chance_permille(
    regions: &Regions,
    rules: &SpreadRules,
    breeze: Breeze,
    pace_percent: u32,
    from: RegionId,
    to: RegionId,
) -> i32 {
    let (source, target) = (&regions.region(from), &regions.region(to));
    let (Some(source), Some(target)) = (source, target) else {
        return 0;
    };
    // How much of the wind blows from source toward target: -1 to 1 in 16.16, times the strength
    // in 16.16, is 32 fractional bits.
    let along = (target.anchor - source.anchor)
        .normalised()
        .dot(unit(breeze.direction));
    let wind_percent =
        (along.raw() as i64 * breeze.strength.raw() as i64 * rules.wind_percent_per_metre as i64)
            >> 32;
    let chance = rules.base_permille as i64
        * (100 + wind_percent).max(0)
        * target.receptive_percent as i64
        * pace_percent as i64
        / 1_000_000;
    chance.clamp(0, 1_000) as i32
}
