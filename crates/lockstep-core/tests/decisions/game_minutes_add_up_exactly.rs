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
    // 216,000 steps make this day, so one step is 65,536 × 216,000 position units per day.
    let units_per_day: u64 = 216_000 * 65_536;
    for multiplier in [1.0, 0.37, 7.0, 20.0] {
        let mut clock = Clock::new(configuration(), 1.0 / 30.0);
        clock.set_multiplier(multiplier);
        let (start_day, start_position) = (clock.day(), clock.position_units());
        let mut total: i64 = 0;
        let mut events = Vec::new();
        while (clock.day(), clock.position_units()) < (start_day + 1, start_position) {
            total += clock.advance_exactly(&mut events).1.raw() as i64;
        }
        // The last step may pass the starting position. The start is a whole minute, so the
        // exact answer is a day plus the minutes of that overshoot, rounded down.
        let overshoot = clock.position_units() - start_position;
        let expected = Fixed32::from_int(1_440).raw() as i64
            + (overshoot as u128 * 1_440 * 65_536 / units_per_day as u128) as i64;
        assert_eq!(total, expected, "multiplier {multiplier}");
    }
}

#[test]
fn a_huge_multiplier_on_a_short_day_still_counts_every_minute() {
    // A ten minute day at a million times would cover 80,000 game minutes a step, past what
    // `Fixed32` holds; the multiplier is capped so a step covers at most 16,384.
    let mut clock = Clock::new(
        ClockConfiguration {
            day_length_real_minutes: 10.0,
            ..configuration()
        },
        1.0 / 30.0,
    );
    clock.set_multiplier(1_000_000.0);
    let mut events = Vec::new();
    let (start_day, start_position) = (clock.day(), clock.position_units());
    let mut total: i64 = 0;
    for _ in 0..10 {
        let minutes = clock.advance_exactly(&mut events).1;
        assert!(minutes > Fixed32::ZERO && minutes <= Fixed32::from_int(16_384));
        total += minutes.raw() as i64;
    }
    let units_per_day: u128 = 18_000 * 65_536;
    let travelled = (clock.day() - start_day) as u128 * units_per_day
        + clock.position_units() as u128
        - start_position as u128;
    assert_eq!(
        total as u128,
        travelled * 1_440 * 65_536 / units_per_day,
        "ten steps cover exactly the minutes the clock moved"
    );
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
