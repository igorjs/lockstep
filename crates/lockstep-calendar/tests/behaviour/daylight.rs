// SPDX-License-Identifier: Apache-2.0
use crate::common::{calendar, daylight_minutes};
use lockstep_core::{Clock, ClockConfiguration};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_middle_of_a_season_holds_its_exact_daylight() {
    let calendar = calendar();
    assert_eq!(calendar.daylight(1), (330, 1110));
    assert_eq!(calendar.daylight(4), (300, 1200));
    assert_eq!(calendar.daylight(7), (360, 1080));
    assert_eq!(calendar.daylight(10), (480, 960));
}

#[test]
fn daylight_changes_in_a_straight_line_between_middles() {
    let calendar = calendar();
    // Summer to autumn: the sunrise goes 300, 320, 340, 360 over days 4 to 7.
    let sunrises: Vec<u32> = (4..=7).map(|day| calendar.daylight(day).0).collect();
    assert_eq!(sunrises, vec![300, 320, 340, 360]);
    // The sunset goes 1200, 1160, 1120, 1080.
    let sunsets: Vec<u32> = (4..=7).map(|day| calendar.daylight(day).1).collect();
    assert_eq!(sunsets, vec![1200, 1160, 1120, 1080]);
}

/// The gate: winter shortens days. Midwinter has the least daylight, midsummer the most, and a
/// day in autumn falls between them.
#[test]
fn winter_shortens_the_day() {
    let calendar = calendar();
    let lengths: Vec<u32> = (0..12)
        .map(|day| daylight_minutes(&calendar, day))
        .collect();
    assert_eq!(lengths[10], 480);
    assert_eq!(lengths[4], 900);
    assert_eq!(*lengths.iter().min().unwrap(), lengths[10]);
    assert_eq!(*lengths.iter().max().unwrap(), lengths[4]);
    assert!(lengths[10] < lengths[8] && lengths[8] < lengths[4]);
}

#[test]
fn the_year_wraps_from_winter_into_spring() {
    let calendar = calendar();
    assert_eq!(calendar.daylight(12), calendar.daylight(0));
    assert_eq!(calendar.daylight(24 + 11), calendar.daylight(11));
    // Between the middle of winter (day 10) and spring (day 13) the sunrise comes earlier.
    let sunrises: Vec<u32> = (10..=13).map(|day| calendar.daylight(day).0).collect();
    assert_eq!(sunrises, vec![480, 430, 380, 330]);
}

#[test]
fn applying_daylight_moves_the_clocks_night() {
    let calendar = calendar();
    let clock_at = |day: u32| {
        let mut clock = Clock::new(
            ClockConfiguration {
                day_length_real_minutes: 24.0,
                sunrise_minute: 360,
                sunset_minute: 1_080,
                // 07:00.
                starting_minute: 420,
                starting_day: day,
            },
            1.0 / 30.0,
        );
        calendar.apply_daylight(day, &mut clock);
        clock
    };
    // Midsummer's sun is up at 07:00; midwinter's does not rise until 08:00.
    assert!(!clock_at(4).is_night());
    assert!(clock_at(10).is_night());
}
