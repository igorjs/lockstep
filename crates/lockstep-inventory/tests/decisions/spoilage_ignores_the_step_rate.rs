// SPDX-License-Identifier: Apache-2.0
//! Decision: spoilage adds up exposure as exact integers, raw 16.16 game minutes times a whole
//! percent, and compares the total with the limit. The same game minutes spoil the same whether
//! they come in 30 or 60 steps a second.
//! Alternative rejected: lowering a freshness fraction each step by `minutes / limit`, which
//! rounds once per step and drifts with the step rate.
//! Would change if: a day at 30 and at 60 steps a second leave different exposure or spoil on
//! different game minutes (the number to beat is zero differences).

use crate::common::{whole, Bay};
use lockstep_core::math::Fixed32;
use lockstep_core::{Clock, ClockConfiguration};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Runs a ration on the rack through one and a half game days at a step rate. Returns its
/// exposure after each whole game hour and the game minute it spoiled on.
fn run(steps_per_second: u32) -> (Vec<u64>, Option<i64>) {
    let mut bay = Bay::new();
    let ration = bay.stock("ration", 1, bay.rack);
    // A game day lasts 24 real minutes: one game minute a real second.
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
    let (mut total, mut hourly, mut spoiled_at) = (Fixed32::ZERO, Vec::new(), None);
    let mut events = Vec::new();
    for step in 1..=(36 * 60 * steps_per_second) {
        let (_, minutes) = clock.advance_exactly(&mut Vec::new());
        total += minutes;
        bay.inventory
            .spoil(minutes, 18, &bay.catalogue, &mut events);
        if spoiled_at.is_none() && !events.is_empty() {
            spoiled_at = Some(total.raw() as i64);
        }
        if step % (60 * steps_per_second) == 0 {
            hourly.push(bay.inventory.item(ration).unwrap().exposure());
        }
    }
    (hourly, spoiled_at)
}

#[test]
fn a_day_and_a_half_at_30_and_60_steps_a_second_spoils_identically() {
    let (slow, slow_spoiled) = run(30);
    let (fast, fast_spoiled) = run(60);
    assert_eq!(slow, fast);
    assert_eq!(slow_spoiled, fast_spoiled);
    // Exactly 1,440 minutes at 100 percent: spoiled on the step that reaches minute 1,440.
    assert_eq!(slow_spoiled, Some(whole(1_440).raw() as i64));
    // A spoiled item stops counting.
    assert_eq!(*slow.last().unwrap(), 1_440 * 65_536 * 100);
    assert_eq!(slow[11], 720 * 65_536 * 100);
}
