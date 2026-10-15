// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::Fixed32;
use lockstep_core::{Clock, Message};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MINUTES_PER_DAY: i64 = 1_440;
/// The coldest minute of a day; half a day on is the warmest.
const COLDEST_MINUTE: i64 = 180;
const HALF_DAY: i64 = 720;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct SeasonId(pub u16);

/// One season. Its values hold at the middle of the season; between middles they change in a
/// straight line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    pub name: String,
    pub sunrise_minute: u32,
    pub sunset_minute: u32,
    /// The average temperature of the day in degrees Celsius.
    pub mean_celsius: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarError {
    Json(String),
    NoSeasons,
    TooMany,
    DuplicateName(String),
    /// The year has fewer days than seasons, so a season would have no day.
    TooFewDays,
    /// Sunrise is not before sunset, or sunset is past the end of the day.
    Daylight(String),
}

impl std::fmt::Display for CalendarError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CalendarError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    pub days_per_year: u32,
    /// How far the temperature swings either side of the day's mean.
    pub daily_swing_celsius: i32,
    pub seasons: Vec<Season>,
}

impl Calendar {
    /// Reads the data form documented in `docs/lockstep-calendar.md`.
    pub fn from_json(text: &str) -> Result<Self, CalendarError> {
        let calendar: Calendar =
            serde_json::from_str(text).map_err(|error| CalendarError::Json(error.to_string()))?;
        calendar.check()?;
        Ok(calendar)
    }

    fn check(&self) -> Result<(), CalendarError> {
        if self.seasons.is_empty() {
            return Err(CalendarError::NoSeasons);
        }
        if self.seasons.len() > u16::MAX as usize {
            return Err(CalendarError::TooMany);
        }
        if (self.days_per_year as usize) < self.seasons.len() {
            return Err(CalendarError::TooFewDays);
        }
        let mut seen = BTreeSet::new();
        for season in &self.seasons {
            if !seen.insert(season.name.as_str()) {
                return Err(CalendarError::DuplicateName(season.name.clone()));
            }
            if season.sunrise_minute >= season.sunset_minute
                || season.sunset_minute > MINUTES_PER_DAY as u32
            {
                return Err(CalendarError::Daylight(season.name.clone()));
            }
        }
        Ok(())
    }

    pub fn season_id(&self, name: &str) -> Option<SeasonId> {
        self.seasons
            .iter()
            .position(|season| season.name == name)
            .map(|index| SeasonId(index as u16))
    }

    pub fn season_named(&self, id: SeasonId) -> Option<&Season> {
        self.seasons.get(id.0 as usize)
    }

    /// The season a game day falls in: the year is cut into equal parts, rounding each cut down.
    pub fn season(&self, day: u32) -> SeasonId {
        let day_of_year = (day % self.days_per_year) as u64;
        SeasonId((day_of_year * self.seasons.len() as u64 / self.days_per_year as u64) as u16)
    }

    /// The two seasons around the middle of a day, and how far between them it is, in
    /// 65,536ths.
    fn blend(&self, day: u32) -> (usize, usize, i64) {
        let count = self.seasons.len() as i64;
        let day_of_year = (day % self.days_per_year) as i64;
        // The middle of the day in 65,536ths of a season; season `i` is centred on (2i+1)/2.
        let position = (2 * day_of_year + 1) * count * 32_768 / self.days_per_year as i64;
        let shifted = (position - 32_768).rem_euclid(count * 65_536);
        let index = (shifted >> 16) as usize;
        (index, (index + 1) % self.seasons.len(), shifted & 0xffff)
    }

    /// A value of the day, in 65,536ths: straight between the neighbouring seasons' values.
    fn mix(&self, day: u32, value: impl Fn(&Season) -> i64) -> i64 {
        let (from, to, fraction) = self.blend(day);
        let (low, high) = (value(&self.seasons[from]), value(&self.seasons[to]));
        low * 65_536 + (high - low) * fraction
    }

    /// The sunrise and sunset minutes of a game day, whole minutes.
    pub fn daylight(&self, day: u32) -> (u32, u32) {
        let round = |scaled: i64| ((scaled + 32_768) >> 16) as u32;
        (
            round(self.mix(day, |season| season.sunrise_minute as i64)),
            round(self.mix(day, |season| season.sunset_minute as i64)),
        )
    }

    /// The average degrees of a game day.
    pub fn mean_temperature(&self, day: u32) -> Fixed32 {
        Fixed32::from_raw(self.mix(day, |season| season.mean_celsius as i64) as i32)
    }

    /// The degrees at a minute of a game day: the day's mean plus a swing that is lowest at
    /// 03:00 and highest at 15:00, rising and falling in straight lines.
    pub fn temperature(&self, day: u32, minute: u32) -> Fixed32 {
        let since_coldest = (minute as i64 - COLDEST_MINUTE).rem_euclid(MINUTES_PER_DAY);
        // From -720 at the coldest, to 720 at the warmest, and back.
        let triangle = if since_coldest <= HALF_DAY {
            since_coldest * 2 - HALF_DAY
        } else {
            (MINUTES_PER_DAY - since_coldest) * 2 - HALF_DAY
        };
        let swing = self.daily_swing_celsius as i64 * 65_536 * triangle / HALF_DAY;
        Fixed32::from_raw((self.mean_temperature(day).raw() as i64 + swing) as i32)
    }

    /// Hands a day's daylight to the clock. The host calls it when the clock reaches a new day.
    pub fn apply_daylight(&self, day: u32, clock: &mut Clock) {
        let (sunrise, sunset) = self.daylight(day);
        clock.set_daylight(sunrise, sunset);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum CalendarEvent {
    SeasonChanged { season: SeasonId },
}

/// The last season seen, so a change is reported once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Almanac {
    season: SeasonId,
}

impl Almanac {
    /// Starts in the season of `day`, reporting nothing for it.
    pub fn new(calendar: &Calendar, day: u32) -> Self {
        Almanac {
            season: calendar.season(day),
        }
    }

    pub fn season(&self) -> SeasonId {
        self.season
    }

    /// Reports a change when `day` is in another season than the last day seen.
    pub fn observe(&mut self, calendar: &Calendar, day: u32) -> Option<CalendarEvent> {
        let season = calendar.season(day);
        if season == self.season {
            return None;
        }
        self.season = season;
        Some(CalendarEvent::SeasonChanged { season })
    }
}
