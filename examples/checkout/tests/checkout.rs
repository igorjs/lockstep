// SPDX-License-Identifier: Apache-2.0
use checkout::{
    fixture_hash, queue_rules, run_fixture, runner, step, store, Checkout, Configuration, Event,
    Intent, DEFAULT_SEED, DEFAULT_STEPS,
};
use lockstep_agents::WaveEvent;
use lockstep_core::{decode, encode, hash_of, Clock, Context, Simulation, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/checkout.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: across the whole session the queue never passes the budget, after every step.
#[test]
fn the_queue_never_exceeds_the_budget_across_the_session() {
    let mut runner = runner(store(), DEFAULT_SEED);
    let mut streams = Streams::new(DEFAULT_SEED ^ 0x0063_6865_636b);
    let budget = runner.simulation().budget();
    let mut peak = 0;
    for step_number in 0..DEFAULT_STEPS {
        step(&mut runner, &checkout::script(step_number, &mut streams));
        let world = runner.simulation().world();
        assert!(world.queue <= budget, "step {step_number}");
        peak = peak.max(world.queue);
    }
    // The budget was reached, so the test would notice a pulse admitted whole.
    assert_eq!(peak, budget);
    assert_eq!(runner.simulation().world().peak_queue, budget);
}

#[test]
fn every_announced_shopper_is_served_in_the_end() {
    let runner = run_fixture(DEFAULT_SEED, DEFAULT_STEPS);
    let world = runner.simulation().world();
    let surge = world.surge.as_ref().expect("a second surge");
    assert!(surge.is_finished());
    assert_eq!(runner.simulation().waiting_outside(), 0);
    assert_eq!(world.queue, 0);
    assert!((300..500).contains(&surge.total()));
    assert!(world.served > surge.total());
}

#[test]
fn a_surge_is_spaced_into_pulses_and_held_when_the_queue_is_full() {
    let mut runner = runner(store(), 3);
    let mut events = step(&mut runner, &[Intent::Announce { shoppers: 100 }]);
    for _ in 0..400 {
        events.extend(step(&mut runner, &[]));
    }
    let pulses: Vec<u32> = events
        .iter()
        .filter_map(|event| match event {
            Event::Surge(WaveEvent::Pulse { arrivals, .. }) => Some(*arrivals),
            _ => None,
        })
        .collect();
    assert_eq!(pulses, vec![40, 40, 20]);
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Surge(WaveEvent::Held { .. }))));
    assert!(events.contains(&Event::Surge(WaveEvent::Finished)));
}

#[test]
fn a_surge_announced_while_one_is_arriving_is_ignored() {
    let mut runner = runner(store(), 3);
    step(&mut runner, &[Intent::Announce { shoppers: 100 }]);
    let events = step(&mut runner, &[Intent::Announce { shoppers: 5 }]);
    assert_eq!(events, vec![Event::Ignored]);
    assert_eq!(
        runner.simulation().world().surge.as_ref().unwrap().total(),
        100
    );
}

#[test]
fn lanes_stop_at_the_most() {
    let mut runner = runner(Configuration { lanes: 7 }, 3);
    step(&mut runner, &[Intent::OpenLane]);
    let events = step(&mut runner, &[Intent::OpenLane]);
    assert_eq!(events, vec![Event::NoMoreLanes]);
    assert_eq!(runner.simulation().world().lanes, 8);
    assert_eq!(queue_rules().queue_budget, 30);
}

/// Steps a simulation by hand, so a restored copy can run beside the original.
fn drive(
    checkout: &mut Checkout,
    clock: &mut Clock,
    randomness: &mut Streams,
    number: u64,
    intents: &[Intent],
) -> Vec<Event> {
    let mut events = Vec::new();
    let (elapsed_game_minutes, elapsed_minutes) = clock.advance_exactly(&mut Vec::new());
    checkout.step(
        &mut Context {
            clock,
            elapsed_game_minutes,
            elapsed_minutes,
            randomness,
            events: &mut events,
            step_number: number,
            step_seconds: 1.0 / 30.0,
        },
        intents,
    );
    events
}

/// Snapshot in the middle of a wave, with arrivals held outside and a pulse still to come.
#[test]
fn a_restored_snapshot_mid_wave_continues_exactly_like_the_one_that_kept_running() {
    let mut clock = runner(store(), 1).clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Checkout::create(store(), &mut randomness);
    drive(
        &mut kept,
        &mut clock,
        &mut randomness,
        0,
        &[Intent::Announce { shoppers: 400 }],
    );
    for number in 1..200 {
        drive(&mut kept, &mut clock, &mut randomness, number, &[]);
    }
    assert!(kept.waiting_outside() > 0);
    assert!(!kept.world().surge.as_ref().unwrap().is_finished());
    let saved = encode(&kept.snapshot());
    let mut loaded = Checkout::restore(decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 200..2_000 {
        let a = drive(&mut kept, &mut clock, &mut randomness, number, &[]);
        let b = drive(
            &mut loaded,
            &mut loaded_clock,
            &mut loaded_randomness,
            number,
            &[],
        );
        assert_eq!(a, b, "step {number}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
    assert_eq!(kept.world().queue, 0);
}
