// SPDX-License-Identifier: Apache-2.0
use crate::common::calendar;
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn degrees(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

#[test]
fn the_mean_of_a_season_holds_at_its_middle() {
    let calendar = calendar();
    let means: Vec<Fixed32> = [1, 4, 7, 10]
        .into_iter()
        .map(|day| calendar.mean_temperature(day))
        .collect();
    assert_eq!(
        means,
        vec![degrees(12), degrees(24), degrees(12), degrees(0)]
    );
}

#[test]
fn a_day_is_coldest_at_three_and_warmest_at_fifteen() {
    let calendar = calendar();
    // Midsummer: mean 24, swing 4.
    assert_eq!(calendar.temperature(4, 180), degrees(20));
    assert_eq!(calendar.temperature(4, 900), degrees(28));
    assert_eq!(calendar.temperature(4, 540), degrees(24));
    assert_eq!(calendar.temperature(4, 1_260), degrees(24));
    // Straight lines between: halfway from the coldest to the mean is a degree under it.
    assert_eq!(calendar.temperature(4, 360), degrees(22));
}

#[test]
fn midwinter_is_colder_than_midsummer_at_every_hour() {
    let calendar = calendar();
    for minute in (0..1_440).step_by(30) {
        assert!(calendar.temperature(10, minute) < calendar.temperature(4, minute));
    }
}

#[test]
fn a_minute_past_the_end_of_the_day_wraps() {
    let calendar = calendar();
    assert_eq!(
        calendar.temperature(4, 1_440 + 180),
        calendar.temperature(4, 180)
    );
}
