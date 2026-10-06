// SPDX-License-Identifier: Apache-2.0
use drone_fleet::{
    fixture_hash, run_fixture, runner, Event, Fleet, Intent, Refusal, Status, DEFAULT_SEED,
    DEFAULT_STEPS, FLIGHT_DRAIN, LAUNCH_FLOOR, RECOVERY_WEAR, RETIRE_WEAR, SERVICE_REPAIR,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Handle, Runner, Simulation};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// One game minute is thirty steps.
const STEPS_PER_MINUTE: u64 = 30;

fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

fn minutes(runner: &mut Runner<Fleet>, intents: &[Intent], count: u64) -> Vec<Event> {
    let mut events = runner.step_once(intents).events;
    for _ in 1..count * STEPS_PER_MINUTE {
        events.extend(runner.step_once(&[]).events);
    }
    events
}

fn first_drone(runner: &Runner<Fleet>) -> Handle {
    runner.simulation().world().drones.handles()[0]
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/drone-fleet.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn a_flight_drains_charge_by_the_minute_and_lands_on_its_own() {
    let mut fleet = runner(1);
    let drone = first_drone(&fleet);
    let charge = fleet.simulation().ids().charge;
    let events = minutes(&mut fleet, &[Intent::Launch { drone, minutes: 10 }], 5);
    assert!(events.contains(&Event::Launched { drone }));
    assert_eq!(
        fleet.simulation().value(drone, charge),
        Some(whole(100 - 5 * FLIGHT_DRAIN))
    );
    let events = minutes(&mut fleet, &[], 6);
    assert!(events.contains(&Event::Landed { drone }));
    assert_eq!(
        fleet.simulation().world().status.get(drone),
        Some(&Status::Docked)
    );
}

#[test]
fn a_drone_that_runs_dry_is_stranded_until_recovered_and_recovery_wears_it() {
    let mut fleet = runner(2);
    let drone = first_drone(&fleet);
    let (wear, charge) = (
        fleet.simulation().ids().wear,
        fleet.simulation().ids().charge,
    );
    let events = minutes(&mut fleet, &[Intent::Launch { drone, minutes: 69 }], 60);
    let order: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Battery { level, .. } => Some(level.as_str()),
            Event::Stranded { .. } => Some("stranded"),
            _ => None,
        })
        .collect();
    assert_eq!(order, ["low", "critical", "stranded"]);
    assert_eq!(
        fleet.simulation().world().status.get(drone),
        Some(&Status::Stranded)
    );
    assert_eq!(fleet.simulation().value(drone, charge), Some(whole(0)));

    let refused = minutes(&mut fleet, &[Intent::Launch { drone, minutes: 10 }], 1);
    assert!(refused.contains(&Event::Refused {
        drone,
        reason: Refusal::NotDocked
    }));
    let wear_before = fleet.simulation().value(drone, wear).unwrap();
    let docked = minutes(&mut fleet, &[Intent::Dock { drone }], 1);
    assert!(docked.contains(&Event::Docked { drone }));
    let wear_after = fleet.simulation().value(drone, wear).unwrap();
    assert_eq!(wear_after, wear_before + whole(RECOVERY_WEAR));
}

#[test]
fn capacity_follows_wear_and_caps_the_charge() {
    let mut fleet = runner(3);
    let drone = first_drone(&fleet);
    let ids = fleet.simulation().ids();
    for _ in 0..3 {
        minutes(&mut fleet, &[Intent::Launch { drone, minutes: 69 }], 60);
        minutes(&mut fleet, &[Intent::Dock { drone }], 40);
    }
    let fleet_state = fleet.simulation();
    let wear = fleet_state.value(drone, ids.wear).unwrap();
    assert!(wear >= whole(3 * RECOVERY_WEAR));
    let capacity = fleet_state.value(drone, ids.capacity).unwrap();
    assert_eq!(capacity, whole(100) - wear);
    assert_eq!(fleet_state.maximum(drone, ids.charge), Some(capacity));
    assert_eq!(
        fleet_state.value(drone, ids.charge),
        Some(capacity),
        "forty minutes docked fills what is left"
    );
}

#[test]
fn service_repairs_wear_but_never_below_the_last_mark_passed() {
    let mut fleet = runner(4);
    let drone = first_drone(&fleet);
    let wear = fleet.simulation().ids().wear;
    let mut marks = Vec::new();
    for _ in 0..5 {
        if fleet.simulation().value(drone, wear).unwrap() >= whole(25) {
            break;
        }
        minutes(&mut fleet, &[Intent::Launch { drone, minutes: 69 }], 60);
        for event in minutes(&mut fleet, &[Intent::Dock { drone }], 40) {
            if let Event::Wear { mark, .. } = event {
                marks.push(mark);
            }
        }
    }
    assert!(
        fleet.simulation().value(drone, wear).unwrap() >= whole(25),
        "five rounds reach the mark"
    );
    assert_eq!(marks.first().map(String::as_str), Some("worn"));
    let before = fleet.simulation().value(drone, wear).unwrap();
    let events = minutes(&mut fleet, &[Intent::Service { drone }], 1);
    assert!(events.contains(&Event::Serviced { drone }));
    let after = fleet.simulation().value(drone, wear).unwrap();
    assert_eq!(after, (before - whole(SERVICE_REPAIR)).max(whole(25)));
    assert!(after >= whole(25), "the worn mark is a floor");
}

#[test]
fn orders_are_refused_with_a_reason() {
    let mut fleet = runner(5);
    let drone = first_drone(&fleet);
    let mut store = lockstep_core::StableVector::new();
    let stranger = (0..20).map(|_| store.insert(())).last().unwrap();
    let refusal = |events: Vec<Event>| match events.as_slice() {
        [Event::Refused { reason, .. }, ..] => Some(*reason),
        _ => None,
    };
    assert_eq!(
        refusal(minutes(&mut fleet, &[Intent::Dock { drone }], 1)),
        Some(Refusal::NotFlying)
    );
    assert_eq!(
        refusal(minutes(
            &mut fleet,
            &[Intent::Launch {
                drone: stranger,
                minutes: 5
            }],
            1
        )),
        Some(Refusal::UnknownDrone)
    );
    minutes(&mut fleet, &[Intent::Launch { drone, minutes: 69 }], 60);
    minutes(&mut fleet, &[Intent::Dock { drone }], 1);
    assert!(
        fleet
            .simulation()
            .value(drone, fleet.simulation().ids().charge)
            < Some(whole(LAUNCH_FLOOR))
    );
    assert_eq!(
        refusal(minutes(
            &mut fleet,
            &[Intent::Launch { drone, minutes: 5 }],
            1
        )),
        Some(Refusal::TooLow)
    );
}

#[test]
fn faults_happen_on_about_five_percent_of_launches() {
    let mut faults = 0;
    let launches = 2_000;
    for seed in 0..launches {
        let mut fleet = runner(seed);
        let drone = first_drone(&fleet);
        let events = fleet
            .step_once(&[Intent::Launch { drone, minutes: 10 }])
            .events;
        faults += events
            .iter()
            .filter(|event| matches!(event, Event::Fault { .. }))
            .count();
    }
    assert!((60..=140).contains(&faults), "{faults} of {launches}");
}

#[test]
fn a_saved_fleet_continues_exactly_like_the_one_in_memory() {
    let session = run_fixture(DEFAULT_SEED, 20_000);
    let snapshot = session.snapshot();
    let saved = bincode::deserialize(&bincode::serialize(&snapshot).unwrap()).unwrap();
    let mut kept = Fleet::restore(snapshot);
    let mut loaded = Fleet::restore(saved);
    // Step both the same way, with orders that touch every drone's capacity limit and effects.
    let drones = kept.world().drones.handles();
    let mut clock = session.clock().clone();
    let (mut first, mut second) = (
        lockstep_core::Streams::new(9),
        lockstep_core::Streams::new(9),
    );
    for step in 0..6_000u64 {
        let drone = drones[(step / 7) as usize % drones.len()];
        let intents = match step % 500 {
            0 => vec![Intent::Service { drone }],
            100 => vec![Intent::Launch { drone, minutes: 30 }],
            300 => vec![Intent::Dock { drone }],
            _ => vec![],
        };
        let (minutes, exact) = clock.advance_exactly(&mut Vec::new());
        let mut outcomes = Vec::new();
        for (fleet, randomness) in [(&mut kept, &mut first), (&mut loaded, &mut second)] {
            let mut events = Vec::new();
            let mut context = lockstep_core::Context {
                clock: &clock,
                elapsed_game_minutes: minutes,
                elapsed_minutes: exact,
                randomness,
                events: &mut events,
                step_number: step,
                step_seconds: 1.0 / 30.0,
            };
            fleet.step(&mut context, &intents);
            outcomes.push(events);
        }
        assert_eq!(outcomes[0], outcomes[1], "step {step}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
}

#[test]
fn a_retired_drone_never_launches_again() {
    let mut fleet = runner(6);
    let drone = first_drone(&fleet);
    let wear = fleet.simulation().ids().wear;
    for _ in 0..12 {
        if fleet.simulation().value(drone, wear).unwrap() >= whole(RETIRE_WEAR) {
            break;
        }
        minutes(&mut fleet, &[Intent::Launch { drone, minutes: 69 }], 60);
        minutes(&mut fleet, &[Intent::Dock { drone }], 40);
    }
    assert!(fleet.simulation().value(drone, wear).unwrap() >= whole(RETIRE_WEAR));
    // Service cannot bring it back under the mark, and a full battery does not help.
    minutes(&mut fleet, &[Intent::Service { drone }], 60);
    let events = minutes(&mut fleet, &[Intent::Launch { drone, minutes: 5 }], 1);
    assert!(events.contains(&Event::Refused {
        drone,
        reason: Refusal::Retired
    }));
}
