// SPDX-License-Identifier: Apache-2.0
use crate::common::calendar;
use lockstep_calendar::{Almanac, CalendarEvent, SeasonId};
use lockstep_core::{decode, encode};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_season_change_is_reported_once_on_the_day_it_starts() {
    let calendar = calendar();
    let mut almanac = Almanac::new(&calendar, 0);
    let events: Vec<(u32, CalendarEvent)> = (0..13)
        .filter_map(|day| almanac.observe(&calendar, day).map(|event| (day, event)))
        .collect();
    let change = |day, season| {
        (
            day,
            CalendarEvent::SeasonChanged {
                season: SeasonId(season),
            },
        )
    };
    assert_eq!(
        events,
        vec![change(3, 1), change(6, 2), change(9, 3), change(12, 0)]
    );
}

#[test]
fn an_almanac_started_mid_year_reports_nothing_for_its_own_season() {
    let calendar = calendar();
    let mut almanac = Almanac::new(&calendar, 10);
    assert_eq!(almanac.season(), SeasonId(3));
    assert_eq!(almanac.observe(&calendar, 10), None);
    assert_eq!(almanac.observe(&calendar, 11), None);
}

#[test]
fn a_saved_almanac_continues_exactly() {
    let calendar = calendar();
    let mut almanac = Almanac::new(&calendar, 0);
    almanac.observe(&calendar, 4);
    let mut restored: Almanac = decode(&encode(&almanac)).unwrap();
    assert_eq!(restored, almanac);
    assert_eq!(
        restored.observe(&calendar, 6),
        almanac.observe(&calendar, 6)
    );
}
