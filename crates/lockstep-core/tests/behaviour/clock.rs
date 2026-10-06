// SPDX-License-Identifier: Apache-2.0
use crate::common::{day_clock, probe_runner, STEP_SECONDS};
use lockstep_core::{Clock, ClockEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn one_real_day_of_steps_advances_one_day_and_emits_one_new_day() {
    let mut runner = probe_runner(1);
    let steps_per_day = 120 * 60 * 30;
    let mut new_days = 0;
    for _ in 0..steps_per_day {
        let advanced = runner.step_once(&[]);
        new_days += advanced
            .clock_events
            .iter()
            .filter(|event| matches!(event, ClockEvent::NewDay(_)))
            .count();
    }
    assert_eq!(new_days, 1);
    assert_eq!(runner.clock().day(), 1);
    assert_eq!(
        runner.clock().minute_of_day(),
        0.0,
        "a full day returns exactly to the start"
    );
    assert_eq!(runner.clock().position_units(), 0);
}

#[test]
fn resting_at_twenty_times_from_ten_at_night_reaches_sunrise_in_about_two_minutes() {
    let mut configuration = day_clock();
    configuration.starting_minute = 22 * 60;
    let mut clock = Clock::new(configuration, STEP_SECONDS);
    clock.set_multiplier(20.0);

    let mut steps = 0u32;
    let mut events = Vec::new();
    while !events.contains(&ClockEvent::Sunrise) {
        clock.advance(&mut events);
        steps += 1;
        assert!(steps < 30 * 200, "sunrise never arrived");
    }
    let real_seconds = steps as f32 * STEP_SECONDS;
    assert!(
        (110.0..=130.0).contains(&real_seconds),
        "took {real_seconds} real seconds"
    );
}

#[test]
fn sunset_and_night_are_reported() {
    let mut configuration = day_clock();
    configuration.starting_minute = 17 * 60 + 59;
    let mut clock = Clock::new(configuration, STEP_SECONDS);
    clock.set_multiplier(60.0);
    assert!(!clock.is_night());
    let mut events = Vec::new();
    for _ in 0..30 {
        clock.advance(&mut events);
    }
    assert!(events.contains(&ClockEvent::Sunset));
    assert!(clock.is_night());
}

#[test]
fn the_multiplier_never_goes_negative() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(-3.0);
    assert_eq!(clock.multiplier(), 0.0);
}

#[test]
fn a_multiplier_is_stored_in_sixty_five_thousand_five_hundred_thirty_sixths() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(20.0);
    assert_eq!(clock.multiplier(), 20.0);
    clock.set_multiplier(0.5);
    assert_eq!(clock.multiplier(), 0.5);
    clock.set_multiplier(f32::NAN);
    assert_eq!(clock.multiplier(), 0.0);
    clock.set_multiplier(f32::INFINITY);
    assert_eq!(clock.multiplier(), 1_000_000.0);
}

#[test]
fn a_stopped_clock_does_not_move() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(0.0);
    let mut events = Vec::new();
    for _ in 0..1000 {
        assert_eq!(clock.advance(&mut events), 0.0);
    }
    assert_eq!(clock.position_units(), 0);
    assert!(events.is_empty());
}

#[test]
fn a_huge_multiplier_crosses_several_days_in_one_step_and_reports_each() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(1_000_000.0);
    let mut events = Vec::new();
    clock.advance(&mut events);
    let new_days = events
        .iter()
        .filter(|event| matches!(event, ClockEvent::NewDay(_)))
        .count();
    assert_eq!(new_days as u32, clock.day());
    assert!(new_days >= 2);
}

#[test]
fn a_clock_survives_a_save_round_trip_exactly() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(3.25);
    let mut events = Vec::new();
    for _ in 0..12_345 {
        clock.advance(&mut events);
    }
    let bytes = bincode::serialize(&clock).unwrap();
    let restored: Clock = bincode::deserialize(&bytes).unwrap();
    assert_eq!(restored, clock);
}
