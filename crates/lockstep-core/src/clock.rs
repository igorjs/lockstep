// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

/// One step at multiplier 1.0 advances this many units. A multiplier is stored as a whole number
/// of 1/65,536ths, so game time is an exact integer and never drifts.
const UNITS_PER_STEP: u64 = 65_536;
const MAXIMUM_MULTIPLIER: f32 = 1_000_000.0;
const MINUTES_PER_DAY: u64 = 1440;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClockConfiguration {
    pub day_length_real_minutes: f32,
    pub sunrise_minute: u32,
    pub sunset_minute: u32,
    pub starting_minute: u32,
    pub starting_day: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClockEvent {
    Sunrise,
    Sunset,
    NewDay(u32),
}

/// The game clock is a rate: one step advances the multiplier times a fixed amount of game time.
///
/// Time of day is kept as an exact integer position inside the day, so a 120 minute day is
/// exactly 216,000 steps at multiplier 1.0 and the position never drifts. Floats appear only
/// at the edges: the multiplier a host sets and the minute of day it reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clock {
    configuration: ClockConfiguration,
    /// Units in one game day: steps per day times `UNITS_PER_STEP`.
    units_per_day: u64,
    /// Position inside the current day, in `[0, units_per_day)`.
    position: u64,
    day: u32,
    game_minutes_per_step: f32,
    /// The multiplier in 1/65,536ths.
    multiplier_units: u64,
}

impl Clock {
    pub fn new(configuration: ClockConfiguration, step_seconds: f32) -> Self {
        let steps_per_day =
            ((configuration.day_length_real_minutes * 60.0 / step_seconds).round() as u64).max(1);
        let units_per_day = steps_per_day * UNITS_PER_STEP;
        Self {
            configuration,
            units_per_day,
            position: (configuration.starting_minute as u64 % MINUTES_PER_DAY) * units_per_day
                / MINUTES_PER_DAY,
            day: configuration.starting_day,
            game_minutes_per_step: MINUTES_PER_DAY as f32 / steps_per_day as f32,
            multiplier_units: UNITS_PER_STEP,
        }
    }

    /// Negative and not-a-number values become zero. Values are rounded to 1/65,536 and capped.
    pub fn set_multiplier(&mut self, multiplier: f32) {
        let clamped = multiplier.clamp(0.0, MAXIMUM_MULTIPLIER);
        self.multiplier_units = (clamped * UNITS_PER_STEP as f32).round() as u64;
    }

    /// The calendar package calls this at each new day to set seasonal daylight.
    pub fn set_daylight(&mut self, sunrise_minute: u32, sunset_minute: u32) {
        self.configuration.sunrise_minute = sunrise_minute;
        self.configuration.sunset_minute = sunset_minute;
    }

    pub fn multiplier(&self) -> f32 {
        self.multiplier_units as f32 / UNITS_PER_STEP as f32
    }

    pub fn minute_of_day(&self) -> f32 {
        (self.position as f64 * MINUTES_PER_DAY as f64 / self.units_per_day as f64) as f32
    }

    pub fn day(&self) -> u32 {
        self.day
    }

    /// The exact position inside the day, for hashing and saves. Prefer this to
    /// `minute_of_day` wherever equality matters.
    pub fn position_units(&self) -> u64 {
        self.position
    }

    pub fn is_night(&self) -> bool {
        let minute = self.position * MINUTES_PER_DAY / self.units_per_day;
        minute < self.configuration.sunrise_minute as u64
            || minute >= self.configuration.sunset_minute as u64
    }

    fn minute_to_units(&self, minute: u32) -> u64 {
        (minute as u64).min(MINUTES_PER_DAY) * self.units_per_day / MINUTES_PER_DAY
    }

    /// Advance one step. Returns elapsed game minutes and pushes any boundary crossed.
    pub fn advance(&mut self, events: &mut Vec<ClockEvent>) -> f32 {
        let elapsed = self.game_minutes_per_step * self.multiplier();
        let before = self.position;
        let mut after = before + self.multiplier_units;
        let sunrise = self.minute_to_units(self.configuration.sunrise_minute);
        let sunset = self.minute_to_units(self.configuration.sunset_minute);
        if before < sunset && after >= sunset {
            events.push(ClockEvent::Sunset);
        }
        while after >= self.units_per_day {
            after -= self.units_per_day;
            self.day += 1;
            events.push(ClockEvent::NewDay(self.day));
            if after >= sunrise {
                events.push(ClockEvent::Sunrise);
            }
        }
        if before < sunrise && after >= sunrise && after > before {
            events.push(ClockEvent::Sunrise);
        }
        self.position = after;
        elapsed
    }
}
