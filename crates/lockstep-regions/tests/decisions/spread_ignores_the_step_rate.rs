// SPDX-License-Identifier: Apache-2.0
//! Decision: the spread tries once per whole interval of simulated minutes, carrying any part of an
//! interval to the next step, and draws only then, so the same simulated time gives the same spread at
//! any step rate.
//! Alternative rejected: a chance per step, which spreads faster at 60 steps a second than at 30.
//! Would change if: a day of spread with a 40 percent chance at 30 and at 60 steps a second ends
//! in different conditions or different events.

use crate::common::{calm, id, regions};
use lockstep_core::{Clock, ClockConfiguration, Streams};
use lockstep_regions::{Spread, SpreadEvent, SpreadRules};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn day_at(steps_per_second: u32) -> (Spread, Vec<SpreadEvent>) {
    let regions = regions();
    let rules = SpreadRules {
        base_permille: 400,
        interval_minutes: 45,
        active_intervals: 4,
        wind_percent_per_metre: 0,
    };
    let mut spread = Spread::new(&regions);
    let mut events = Vec::new();
    spread.start(id(&regions, "a"), &rules, &mut events);
    let mut streams = Streams::new(11);
    let mut clock = Clock::new(
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 0,
            starting_day: 0,
        },
        1.0 / steps_per_second as f32,
    );
    for _ in 0..(1_440 * steps_per_second) {
        let (_, minutes) = clock.advance_exactly(&mut Vec::new());
        spread.advance(
            &regions,
            &rules,
            calm(),
            100,
            minutes,
            &mut streams,
            &mut events,
        );
    }
    (spread, events)
}

#[test]
fn a_day_of_spread_is_the_same_at_30_and_60_steps_a_second() {
    let slow = day_at(30);
    assert_eq!(slow, day_at(60));
    // A day holds 32 intervals of 45 minutes: enough for it to start, spread and burn out.
    assert!(slow.0.spent_count() > 1);
}
