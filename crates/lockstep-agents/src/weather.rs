// SPDX-License-Identifier: Apache-2.0
use crate::noise::Wind;
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{Message, Streams};
use serde::{Deserialize, Serialize};

/// Raw 16.16 units of one game minute times 60: one game second.
const RAW_SECOND: i64 = 65_536;
/// 4 degrees a minute is 728 turn units; 12 a second stays under it.
const DRIFT_PER_SECOND: i32 = 12;
/// The quarter and half turns that bound a front's swing.
const QUARTER_TURN: i32 = 16_384;
const HALF_TURN: i32 = 32_768;

/// How the weather behaves. Draws come from the `"weather"` stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherRules {
    /// The strength the wind drifts toward, metres a second.
    pub mean_strength: Fixed32,
    /// Fronts a game day, on average.
    pub fronts_per_day: u32,
}

/// A gust: extra strength for a while.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gust {
    /// 50 to 100.
    pub extra_percent: u32,
    pub seconds_left: u32,
}

/// A front swinging the wind round over some minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Front {
    pub from: Turn,
    /// Signed turn units, a quarter to a half turn either way.
    pub swing: i32,
    pub seconds: u32,
    pub seconds_done: u32,
}

/// The wind over time: the direction drifts at most 4 degrees a game minute, the strength drifts
/// toward the mean, gusts add 50 to 100 percent for 30 to 90 game seconds once or twice an hour,
/// and fronts swing the direction 90 to 180 degrees over 8 to 12 game minutes. It advances one
/// whole game second at a time, so the same game minutes give the same weather at any step rate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Weather {
    pub direction: Turn,
    /// Before any gust.
    pub strength: Fixed32,
    pub gust: Option<Gust>,
    pub front: Option<Front>,
    /// Raw 16.16 game minutes times 60 not yet a whole second.
    carried: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum WeatherEvent {
    GustStarted,
    GustEnded,
    FrontStarted { swing: i32 },
    FrontPassed,
}

impl Weather {
    pub fn new(direction: Turn, strength: Fixed32) -> Self {
        Weather {
            direction,
            strength,
            gust: None,
            front: None,
            carried: 0,
        }
    }

    /// The wind now, gusts included.
    pub fn wind(&self) -> Wind {
        let extra = self.gust.map_or(0, |gust| gust.extra_percent) as i64;
        let strength = self.strength.raw() as i64 * (100 + extra) / 100;
        Wind {
            direction: self.direction,
            strength_metres_per_second: Fixed32::from_raw(strength.min(i32::MAX as i64) as i32),
        }
    }

    /// Advances by `minutes` of game time, one whole game second at a time.
    pub fn advance(
        &mut self,
        minutes: Fixed32,
        rules: &WeatherRules,
        streams: &mut Streams,
        events: &mut Vec<WeatherEvent>,
    ) {
        self.carried += minutes.raw().max(0) as i64 * 60;
        while self.carried >= RAW_SECOND {
            self.carried -= RAW_SECOND;
            self.second(rules, streams, events);
        }
    }

    fn second(
        &mut self,
        rules: &WeatherRules,
        streams: &mut Streams,
        events: &mut Vec<WeatherEvent>,
    ) {
        // Direction: a front swings it on a straight schedule; otherwise it drifts.
        match &mut self.front {
            Some(front) => {
                front.seconds_done += 1;
                let swung = front.swing as i64 * front.seconds_done as i64 / front.seconds as i64;
                self.direction = front.from.wrapping_add(swung as i16 as u16);
                if front.seconds_done >= front.seconds {
                    self.direction = front.from.wrapping_add(front.swing as u16);
                    self.front = None;
                    events.push(WeatherEvent::FrontPassed);
                }
            }
            None => {
                let drift = streams.range("weather", -DRIFT_PER_SECOND, DRIFT_PER_SECOND + 1);
                self.direction = self.direction.wrapping_add(drift as u16);
                if streams.range("weather", 0, 86_400) < rules.fronts_per_day as i32 {
                    let size = streams.range("weather", QUARTER_TURN, HALF_TURN + 1);
                    let swing = if streams.range("weather", 0, 2) == 0 {
                        size
                    } else {
                        -size
                    };
                    let seconds = streams.range("weather", 8 * 60, 12 * 60 + 1) as u32;
                    self.front = Some(Front {
                        from: self.direction,
                        swing,
                        seconds,
                        seconds_done: 0,
                    });
                    events.push(WeatherEvent::FrontStarted { swing });
                }
            }
        }
        // Strength: a thirtieth of the way to the mean each game minute, a second's share now.
        let gap = rules.mean_strength.raw() as i64 - self.strength.raw() as i64;
        self.strength = Fixed32::from_raw((self.strength.raw() as i64 + gap / 1_800) as i32);
        // Gusts: once or twice an hour on average, 1.5 in 3,600 seconds.
        match &mut self.gust {
            Some(gust) => {
                gust.seconds_left -= 1;
                if gust.seconds_left == 0 {
                    self.gust = None;
                    events.push(WeatherEvent::GustEnded);
                }
            }
            None => {
                if streams.range("weather", 0, 7_200) < 3 {
                    self.gust = Some(Gust {
                        extra_percent: streams.range("weather", 50, 101) as u32,
                        seconds_left: streams.range("weather", 30, 91) as u32,
                    });
                    events.push(WeatherEvent::GustStarted);
                }
            }
        }
    }
}
