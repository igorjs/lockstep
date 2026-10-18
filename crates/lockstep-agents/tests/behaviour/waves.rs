// SPDX-License-Identifier: Apache-2.0
use lockstep_agents::{Director, WaveEvent, WaveRules, Waves};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const RULES: WaveRules = WaveRules {
    pulse_size: 10,
    interval_minutes: 5,
    start_after_minutes: 3,
};

fn run(
    total: u32,
    minutes_per_step: Fixed32,
    steps: u32,
    headroom: u32,
) -> (Waves, Vec<WaveEvent>) {
    let mut waves = Waves::new(total);
    let mut events = Vec::new();
    for _ in 0..steps {
        waves.advance(&RULES, minutes_per_step, headroom, &mut events);
    }
    (waves, events)
}

#[test]
fn nothing_arrives_before_the_start_delay() {
    let (waves, events) = run(25, Fixed32::from_ratio(1, 2), 5, 100);
    assert_eq!(waves.admitted(), 0);
    assert!(events.is_empty());
}

#[test]
fn the_last_pulse_is_the_remainder_and_the_wave_finishes_once() {
    let (waves, events) = run(25, Fixed32::from_ratio(1, 2), 100, 100);
    let sizes: Vec<u32> = events
        .iter()
        .filter_map(|event| match event {
            WaveEvent::Pulse { arrivals, .. } => Some(*arrivals),
            _ => None,
        })
        .collect();
    assert_eq!(sizes, vec![10, 10, 5]);
    assert!(waves.is_finished());
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == WaveEvent::Finished)
            .count(),
        1
    );
}

#[test]
fn a_zero_total_is_finished_at_once() {
    let mut events = Vec::new();
    let mut waves = Waves::new(0);
    assert!(waves.is_finished());
    assert_eq!(
        waves.advance(&RULES, Fixed32::from_int(60), 9, &mut events),
        0
    );
    assert!(events.is_empty());
}

#[test]
fn the_director_headroom_is_the_budget_less_the_load() {
    let director = Director { alert_budget: 8 };
    assert_eq!(director.headroom(3), 5);
    assert_eq!(director.headroom(8), 0);
    assert_eq!(director.headroom(20), 0);
}

#[test]
fn a_hold_is_reported_once_and_arrivals_are_admitted_as_room_appears() {
    let mut waves = Waves::new(30);
    let mut events = Vec::new();
    waves.advance(&RULES, Fixed32::from_int(30), 4, &mut events);
    waves.advance(&RULES, Fixed32::from_int(1), 0, &mut events);
    assert_eq!(waves.admitted(), 4);
    assert_eq!(waves.waiting(&RULES), 26);
    let holds = events
        .iter()
        .filter(|event| matches!(event, WaveEvent::Held { .. }))
        .count();
    assert_eq!(holds, 1);
    waves.advance(&RULES, Fixed32::from_int(1), 100, &mut events);
    assert!(waves.is_finished());
}
