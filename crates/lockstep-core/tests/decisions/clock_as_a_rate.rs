// SPDX-License-Identifier: Apache-2.0
//! Decision: the game clock is a rate (game minutes per step times a multiplier).
//! Alternative rejected: ticks as game minutes, one minute per step.
//! Would change if: a rest at twenty times needs more than 600 steps of real time per night
//! (the number to beat is 130 real seconds from ten at night to sunrise).

use crate::common::{day_clock, STEP_SECONDS};
use lockstep_core::Clock;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_multiplier_scales_game_time_without_changing_the_step_rate() {
    let mut normal = Clock::new(day_clock(), STEP_SECONDS);
    let mut resting = Clock::new(day_clock(), STEP_SECONDS);
    resting.set_multiplier(20.0);
    let mut events = Vec::new();
    let normal_elapsed = normal.advance(&mut events);
    let resting_elapsed = resting.advance(&mut events);
    assert_eq!(resting_elapsed, normal_elapsed * 20.0);
}

#[test]
fn a_one_hundred_twenty_minute_day_is_one_game_day_over_two_hundred_sixteen_thousand_steps() {
    let clock = Clock::new(day_clock(), STEP_SECONDS);
    let mut probe = clock.clone();
    let mut events = Vec::new();
    let per_step = probe.advance(&mut events);
    assert!(
        (per_step * 216_000.0 - 1440.0).abs() < 0.01,
        "one step is {per_step} game minutes"
    );
}
