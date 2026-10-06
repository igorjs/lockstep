// SPDX-License-Identifier: Apache-2.0
//! Decision: the weather advances one whole game second at a time, drawing from the `"weather"`
//! stream, so the same game minutes give the same weather at any step rate, and one game day
//! from seed 42 hashes to the committed value natively and under WebAssembly.
//! Alternative rejected: drawing once per step, which ties the weather to the frame rate.
//! Would change if: a day at 30 and at 60 steps a second differ, or the hash changes without a
//! commit naming the weather rule that changed.

use crate::common::weather_day;
use lockstep_core::hash_of;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn one_day_from_seed_42_matches_the_committed_hash_at_any_step_rate() {
    let (slow, slow_events) = weather_day(42, 30);
    let (fast, fast_events) = weather_day(42, 60);
    assert_eq!(slow, fast);
    assert_eq!(slow_events, fast_events);
    let committed = include_str!("../fixtures/weather.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&slow)), committed);
}
