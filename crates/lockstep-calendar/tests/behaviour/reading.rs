// SPDX-License-Identifier: Apache-2.0
use crate::common::{calendar, CALENDAR};
use lockstep_calendar::{Calendar, CalendarError, SeasonId};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn seasons_are_read_in_order_and_found_by_name() {
    let calendar = calendar();
    assert_eq!(calendar.seasons.len(), 4);
    assert_eq!(calendar.season_id("winter"), Some(SeasonId(3)));
    assert_eq!(calendar.season_id("monsoon"), None);
    assert_eq!(calendar.season_named(SeasonId(1)).unwrap().name, "summer");
}

#[test]
fn each_refusal_names_its_reason() {
    let with = |from: &str, to: &str| Calendar::from_json(&CALENDAR.replace(from, to));
    assert!(matches!(
        Calendar::from_json("{"),
        Err(CalendarError::Json(_))
    ));
    assert_eq!(
        Calendar::from_json(r#"{"days_per_year":4,"daily_swing_celsius":0,"seasons":[]}"#),
        Err(CalendarError::NoSeasons)
    );
    assert_eq!(
        with("\"days_per_year\": 12", "\"days_per_year\": 3"),
        Err(CalendarError::TooFewDays)
    );
    assert_eq!(
        with("\"autumn\"", "\"spring\""),
        Err(CalendarError::DuplicateName("spring".into()))
    );
    assert_eq!(
        with("\"sunrise_minute\": 480", "\"sunrise_minute\": 960"),
        Err(CalendarError::Daylight("winter".into()))
    );
    assert_eq!(
        with("\"sunset_minute\": 1200", "\"sunset_minute\": 1500"),
        Err(CalendarError::Daylight("summer".into()))
    );
}

#[test]
fn a_year_is_cut_into_equal_seasons_and_repeats() {
    let calendar = calendar();
    let seasons: Vec<u16> = (0..14).map(|day| calendar.season(day).0).collect();
    assert_eq!(seasons, vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 0, 0]);
}
