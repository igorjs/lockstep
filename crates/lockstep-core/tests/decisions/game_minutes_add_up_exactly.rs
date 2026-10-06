// SPDX-License-Identifier: Apache-2.0
//! Decision: each step also reports its game minutes as exact 16.16 fixed point, the difference of
//! two rounded-down positions, so the minutes of any number of steps add up to exactly the minutes
//! between the first and last position.
//! Alternative rejected: a fixed per-step amount (1,440 / steps per day) rounded once, which loses
//! a fraction every step and drifts by minutes over a day; or the float minutes, which differ in
//! their last bits from platform to platform.
//! Would change if: a game day of steps, at any multiplier, adds up to anything but exactly 1,440
//! minutes (the number to beat is zero raw units off).

use lockstep_core::math::Fixed32;
use lockstep_core::{Clock, ClockConfiguration};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn configuration() -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes: 120.0,
        sunrise_minute: 360,
        sunset_minute: 1_080,
        starting_minute: 8 * 60,
        starting_day: 1,
    }
}

#[test]
fn a_day_of_steps_adds_up_to_exactly_1440_minutes_at_any_multiplier() {
    for multiplier in [1.0, 0.37, 7.0, 20.0] {
        let mut clock = Clock::new(configuration(), 1.0 / 30.0);
        clock.set_multiplier(multiplier);
        let (start_day, start_position) = (clock.day(), clock.position_units());
        let mut total: i64 = 0;
        let mut events = Vec::new();
        let mut steps = 0u64;
        while (clock.day(), clock.position_units()) < (start_day + 1, start_position) {
            total += clock.advance_exactly(&mut events).1.raw() as i64;
            steps += 1;
        }
        // The last step can pass the starting position; measure to exactly where it stopped.
        let overshoot = clock.position_units() - start_position;
        let units_per_step = (multiplier as f64 * 65_536.0).round() as u64;
        assert!(
            overshoot < units_per_step.max(1),
            "{multiplier}: overshoot {overshoot}"
        );
        let expected = Fixed32::from_int(1_440).raw() as i64;
        let overshoot_minutes = total - expected;
        assert!(
            (0..=units_per_step as i64).contains(&overshoot_minutes),
            "{multiplier}: {steps} steps gave {total} raw units against {expected}"
        );
        if overshoot == 0 {
            assert_eq!(total, expected, "{multiplier}: an exact day");
        }
    }
}

#[test]
fn a_whole_number_of_steps_per_day_gives_exactly_1440_minutes() {
    // 216,000 steps make the 120 minute day, at multiplier 1.0.
    let mut clock = Clock::new(configuration(), 1.0 / 30.0);
    let mut events = Vec::new();
    let total: i64 = (0..216_000)
        .map(|_| clock.advance_exactly(&mut events).1.raw() as i64)
        .sum();
    assert_eq!(total, Fixed32::from_int(1_440).raw() as i64);
}
