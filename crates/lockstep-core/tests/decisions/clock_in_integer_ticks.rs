//! Decision: game time is an exact integer position inside the day, advanced by an integer
//! multiplier in 1/65,536ths. Floats appear only where a host reads or sets a value.
//! Alternative rejected: accumulating minutes in a 32-bit float on every step, which drifted
//! 1.37 game minutes over one 120 minute day and made a day about 215,800 steps.
//! Would change if: any day, at any multiplier that divides a day evenly, is not an exact whole
//! number of steps (the number to beat is zero drift over three days).

use crate::common::{day_clock, STEP_SECONDS};
use lockstep_core::{Clock, ClockEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn run_steps(clock: &mut Clock, steps: u64) -> Vec<ClockEvent> {
    let mut events = Vec::new();
    for _ in 0..steps {
        clock.advance(&mut events);
    }
    events
}

#[test]
fn three_days_of_steps_are_exactly_three_days() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    let events = run_steps(&mut clock, 3 * 216_000);
    let new_days: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, ClockEvent::NewDay(_)))
        .collect();
    assert_eq!(new_days.len(), 3);
    assert_eq!(clock.day(), 3);
    assert_eq!(clock.position_units(), 0);
}

#[test]
fn a_day_at_twenty_times_is_exactly_ten_thousand_eight_hundred_steps() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    clock.set_multiplier(20.0);
    let events = run_steps(&mut clock, 10_800);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, ClockEvent::NewDay(_)))
            .count(),
        1
    );
    assert_eq!(clock.position_units(), 0);
}

#[test]
fn changing_the_multiplier_mid_day_keeps_the_position_exact() {
    let mut clock = Clock::new(day_clock(), STEP_SECONDS);
    run_steps(&mut clock, 108_000); // half a day at 1.0
    assert_eq!(clock.minute_of_day(), 720.0);
    clock.set_multiplier(2.0);
    let events = run_steps(&mut clock, 54_000); // the other half at 2.0 takes exactly 54,000 steps
                                                // The second half holds the 18:00 sunset and then the new day, in that order.
    assert_eq!(events, vec![ClockEvent::Sunset, ClockEvent::NewDay(1)]);
    assert_eq!(clock.position_units(), 0);
}
