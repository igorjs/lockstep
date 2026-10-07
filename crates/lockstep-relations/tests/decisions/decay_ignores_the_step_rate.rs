// SPDX-License-Identifier: Apache-2.0
//! Decision: a standing drifts toward its rest by the relation's decay a day, computed afresh
//! each tick from its value at the last change and the exact game minutes since, so the same
//! game time decays it the same at any step rate.
//! Alternative rejected: taking a little off each step, which rounds once per step and drifts
//! with the frame rate.
//! Would change if: trust raised to 55 and left for a game day, decaying 10 a day, is not
//! exactly 45 at both 30 and 60 steps a second, or falls below trusted on different minutes.

use crate::common::{kinds, three, whole};
use lockstep_core::math::Fixed32;
use lockstep_core::{Clock, ClockConfiguration};
use lockstep_relations::{RelationEvent, Relations, Target};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn run(steps_per_second: u32) -> (Fixed32, Vec<u64>) {
    let kinds = kinds();
    let trust = kinds.relation_id("trust").unwrap();
    let (a, b, _) = three();
    let mut relations = Relations::new();
    let mut events = Vec::new();
    relations.change(&kinds, trust, a, Target::Entity(b), whole(55), &mut events);
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
    let mut crossed_at = Vec::new();
    // A game day lasts 24 real minutes: 1,440 seconds of steps.
    for step in 1..=(1_440 * steps_per_second as u64) {
        let (_, minutes) = clock.advance_exactly(&mut Vec::new());
        events.clear();
        relations.tick(&kinds, minutes, &mut events);
        if events
            .iter()
            .any(|event| matches!(event, RelationEvent::Crossed { upward: false, .. }))
        {
            // The game minute it crossed on.
            crossed_at.push(step * 60 / (60 * steps_per_second as u64));
        }
    }
    (
        relations.get(&kinds, trust, a, Target::Entity(b)),
        crossed_at,
    )
}

#[test]
fn a_day_of_decay_is_the_same_at_30_and_60_steps_a_second() {
    let slow = run(30);
    assert_eq!(slow, run(60));
    assert_eq!(slow.0, whole(45));
    // 55 falls below trusted (50) just after half a day: on game minute 720 at both rates.
    assert_eq!(slow.1, vec![720]);
}
