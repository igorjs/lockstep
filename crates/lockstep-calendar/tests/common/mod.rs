// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_calendar::Calendar;

/// A twelve day year of four seasons of three days. The middle day of each season is day 1, 4, 7
/// and 10, and holds the season's values exactly.
pub const CALENDAR: &str = r#"{
  "days_per_year": 12,
  "daily_swing_celsius": 4,
  "seasons": [
    { "name": "spring", "sunrise_minute": 330, "sunset_minute": 1110, "mean_celsius": 12 },
    { "name": "summer", "sunrise_minute": 300, "sunset_minute": 1200, "mean_celsius": 24 },
    { "name": "autumn", "sunrise_minute": 360, "sunset_minute": 1080, "mean_celsius": 12 },
    { "name": "winter", "sunrise_minute": 480, "sunset_minute": 960, "mean_celsius": 0 }
  ]
}"#;

pub fn calendar() -> Calendar {
    Calendar::from_json(CALENDAR).expect("the test calendar is valid")
}

/// The minutes of daylight on a day.
pub fn daylight_minutes(calendar: &Calendar, day: u32) -> u32 {
    let (sunrise, sunset) = calendar.daylight(day);
    sunset - sunrise
}
