// SPDX-License-Identifier: Apache-2.0
//! Decision: a day takes at most 2^32 steps. A configuration asking for more (a damaged
//! recording, a zero step length) gets exactly 2^32, so the clock's 64-bit arithmetic cannot
//! overflow, instead of an error that every caller would have to handle.
//! Alternative rejected: refusing the configuration with an error, which would change the
//! signature of `Clock::new` and `Runner::new` for a case no real session reaches; and checked
//! arithmetic throughout the clock, which costs on every step to guard a configuration.
//! Would change if: a design needs a day longer than 2^32 steps (over two years at 60 steps a
//! second), or any configuration below the cap changes its position.

use lockstep_core::{Clock, ClockConfiguration};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn noon(day_length_real_minutes: f32) -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes,
        sunrise_minute: 360,
        sunset_minute: 1080,
        starting_minute: 720,
        starting_day: 0,
    }
}

#[test]
fn a_long_day_below_the_cap_is_exact() {
    // 1,048,576 real minutes at two steps a second is 125,829,120 steps, exact in a float.
    let clock = Clock::new(noon(1_048_576.0), 0.5);
    assert_eq!(clock.position_units(), 125_829_120 * 65_536 / 2);
}

#[test]
fn a_day_past_the_cap_has_exactly_the_cap() {
    let expected = (1u64 << 32) * 65_536 / 2;
    assert_eq!(
        Clock::new(noon(f32::MAX), 1.0 / 30.0).position_units(),
        expected
    );
    assert_eq!(Clock::new(noon(10.0), 0.0).position_units(), expected);
}
