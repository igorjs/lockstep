// SPDX-License-Identifier: Apache-2.0
use franchise::{
    configuration, fixture_hash, play_season, run_fixture, runner, script, Event, Franchise,
    Intent, Refusal, DEFAULT_SEED, DEFAULT_STEPS, SEASONS,
};
use lockstep_core::{hash_of, Clock, Context, Handle, Runner, Simulation, Streams};
use lockstep_meta::{open, seal, Chronicle, ControlEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn start(chronicle: &Chronicle) -> Runner<Franchise> {
    runner(configuration(1, chronicle), 1)
}

fn depart(runner: &mut Runner<Franchise>, who: Handle) -> Vec<Event> {
    runner.step_once(&[Intent::Depart { who }]).events
}

fn hire(runner: &mut Runner<Franchise>, line: &str, points: u32) -> Vec<Event> {
    runner
        .step_once(&[Intent::Hire {
            line: line.into(),
            archetype: "cook".into(),
            purchases: vec![("stamina".into(), points)],
        }])
        .events
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/franchise.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: the chronicle survives a new season (a fresh simulation run). What season one lost
/// is in the chronicle after it is saved and opened, season two starts without those lines and
/// refuses to hire their names, and nothing in the chronicle is forgotten by the seasons after.
#[test]
fn the_chronicle_survives_a_new_season() {
    let mut chronicle = Chronicle::new();
    let first = play_season(&mut chronicle, DEFAULT_SEED, DEFAULT_STEPS);
    let lost_in_first: Vec<_> = chronicle.lost().to_vec();
    assert!(!lost_in_first.is_empty(), "a season loses a line");
    assert_eq!(chronicle.runs(), 1);

    let mut chronicle: Chronicle = open(&seal(&chronicle)).unwrap();
    assert_eq!(chronicle.lost(), &lost_in_first[..]);

    // A new valley: a fresh simulation, which holds none of the first one's state.
    let mut second = runner(configuration(2, &chronicle), DEFAULT_SEED + 1);
    for record in &lost_in_first {
        assert!(second.simulation().is_burned(&record.name));
        assert!(second.simulation().crew().iter().all(|who| second
            .simulation()
            .world()
            .crew
            .get(*who)
            .unwrap()
            .line
            != record.name));
        assert!(matches!(
            hire(&mut second, &record.name, 0)[..],
            [Event::Refused {
                reason: Refusal::Burned,
                ..
            }]
        ));
    }
    assert_ne!(first.simulation().world().season, 2);

    // Later seasons only add to the record.
    play_season(&mut chronicle, DEFAULT_SEED + 1, DEFAULT_STEPS);
    play_season(&mut chronicle, DEFAULT_SEED + 2, DEFAULT_STEPS);
    assert_eq!(chronicle.runs(), SEASONS);
    assert_eq!(&chronicle.lost()[..lost_in_first.len()], &lost_in_first[..]);
}

#[test]
fn a_departing_manager_hands_control_to_the_newest_of_the_line() {
    let mut runner = start(&Chronicle::new());
    let okafor = runner.simulation().manager().unwrap();
    // Two more of the line: generation 2 then 3.
    hire(&mut runner, "Okafor", 0);
    let newest = match hire(&mut runner, "Okafor", 1)[..] {
        [Event::Hired {
            who, generation, ..
        }] => {
            assert_eq!(generation, 3);
            who
        }
        ref other => panic!("{other:?}"),
    };
    let events = depart(&mut runner, okafor);
    assert_eq!(runner.simulation().manager(), Some(newest));
    assert!(events.contains(&Event::Control(ControlEvent::Transferred {
        from: okafor,
        to: newest
    })));
    // The line lives on, so its name is not burned.
    assert!(!runner.simulation().is_burned("Okafor"));
}

#[test]
fn control_falls_back_to_another_line_when_the_manager_line_is_gone() {
    let mut runner = start(&Chronicle::new());
    let manager = runner.simulation().manager().unwrap();
    let events = depart(&mut runner, manager);
    let next = runner.simulation().manager().unwrap();
    assert_ne!(next, manager);
    assert!(events.contains(&Event::LineLost {
        line: "Okafor".into(),
        members: 1
    }));
    assert!(events.contains(&Event::Control(ControlEvent::Transferred {
        from: manager,
        to: next
    })));
}

#[test]
fn a_lost_line_is_burned_and_its_name_cannot_be_hired_again() {
    let mut runner = start(&Chronicle::new());
    let manager = runner.simulation().manager().unwrap();
    depart(&mut runner, manager);
    assert!(runner.simulation().is_burned("Okafor"));
    assert!(matches!(
        hire(&mut runner, "Okafor", 0)[..],
        [Event::Refused {
            reason: Refusal::Burned,
            ..
        }]
    ));
    // A name that was never lost is open.
    assert!(matches!(
        hire(&mut runner, "Novak", 0)[..],
        [Event::Hired { .. }]
    ));
}

#[test]
fn the_last_person_leaving_vacates_control_and_a_hire_takes_it() {
    let mut runner = start(&Chronicle::new());
    for who in runner.simulation().crew() {
        depart(&mut runner, who);
    }
    assert_eq!(runner.simulation().manager(), None);
    let events = hire(&mut runner, "Novak", 0);
    let [Event::Hired { who, .. }] = events[..] else {
        panic!("{events:?}")
    };
    assert_eq!(runner.simulation().manager(), Some(who));
}

#[test]
fn the_vacancy_is_reported() {
    let mut runner = start(&Chronicle::new());
    let crew = runner.simulation().crew();
    for who in &crew[1..] {
        depart(&mut runner, *who);
    }
    let events = depart(&mut runner, crew[0]);
    assert!(events.contains(&Event::Control(ControlEvent::Vacant { from: crew[0] })));
}

#[test]
fn a_hire_is_bought_with_points_within_the_budget() {
    let mut runner = start(&Chronicle::new());
    // A cook has stamina 4 and a budget of 6 points, one each: 2 points reach 6.
    let events = hire(&mut runner, "Novak", 2);
    let [Event::Hired { who, .. }] = events[..] else {
        panic!("{events:?}")
    };
    let member = runner.simulation().world().crew.get(who).unwrap();
    assert_eq!(member.attributes[0], ("stamina".to_string(), 6));
    // Five more points would pass the maximum of 8.
    assert!(matches!(
        hire(&mut runner, "Novak", 5)[..],
        [Event::Refused {
            reason: Refusal::Template(_),
            ..
        }]
    ));
}

#[test]
fn a_departure_that_names_no_one_is_unknown() {
    let mut runner = start(&Chronicle::new());
    let who = runner.simulation().manager().unwrap();
    depart(&mut runner, who);
    assert_eq!(depart(&mut runner, who), vec![Event::Unknown]);
}

#[test]
fn the_same_chronicle_and_seed_play_the_same_season() {
    let (chronicle_a, runner_a) = run_fixture(7, 600);
    let (chronicle_b, runner_b) = run_fixture(7, 600);
    assert_eq!(chronicle_a, chronicle_b);
    assert_eq!(runner_a.hash(), runner_b.hash());
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    franchise: &mut Franchise,
    clock: &mut Clock,
    randomness: &mut Streams,
    intents: &[Intent],
    number: u64,
) -> Vec<Event> {
    let (minutes, exact) = clock.advance_exactly(&mut Vec::new());
    let mut events = Vec::new();
    let mut context = Context {
        clock,
        elapsed_game_minutes: minutes,
        elapsed_minutes: exact,
        randomness,
        events: &mut events,
        step_number: number,
        step_seconds: 1.0 / 30.0,
    };
    franchise.step(&mut context, intents);
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let session = start(&Chronicle::new());
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Franchise::create(configuration(1, &Chronicle::new()), &mut Streams::new(5));
    let mut streams = Streams::new(77);
    for number in 0..1_200 {
        let intents = script(&kept, &mut streams);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = Franchise::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 1_200..2_400 {
        let intents = script(&kept, &mut streams);
        let a = step(&mut kept, &mut clock, &mut randomness, &intents, number);
        let b = step(
            &mut loaded,
            &mut loaded_clock,
            &mut loaded_randomness,
            &intents,
            number,
        );
        assert_eq!(a, b, "step {number}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
}
