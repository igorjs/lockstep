// SPDX-License-Identifier: Apache-2.0
//! Decision: a pulse falls due by the game minutes since the wave began, counted from the total
//! and not step by step, so the same game time gives the same pulses at any step rate.
//! Alternative rejected: a pulse every so many steps, which ties the arrivals to the frame rate.
//! Would change if: a wave stepped in halves and in quarters of a minute differs in when pulses
//! fall due or how many arrivals each admits.

use lockstep_agents::{WaveEvent, WaveRules, Waves};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const RULES: WaveRules = WaveRules {
    pulse_size: 12,
    interval_minutes: 5,
    start_after_minutes: 2,
};

/// Pulses with the whole game minute each fell due in.
fn timeline(steps_per_minute: i32) -> Vec<(u32, u32, i32)> {
    let mut waves = Waves::new(100);
    let mut timeline = Vec::new();
    for step in 1..=60 * steps_per_minute {
        let mut events = Vec::new();
        waves.advance(
            &RULES,
            Fixed32::from_ratio(1, steps_per_minute),
            u32::MAX,
            &mut events,
        );
        for event in events {
            if let WaveEvent::Pulse { index, arrivals } = event {
                timeline.push((index, arrivals, step / steps_per_minute));
            }
        }
    }
    timeline
}

#[test]
fn pulses_fall_due_at_the_same_minutes_at_any_step_rate() {
    let halves = timeline(2);
    assert_eq!(halves, timeline(4));
    assert_eq!(halves, timeline(1));
    assert_eq!(halves.len(), 9);
    assert_eq!(halves[0], (0, 12, 2));
    assert_eq!(halves[1], (1, 12, 7));
    assert_eq!(halves[8], (8, 4, 42));
}
