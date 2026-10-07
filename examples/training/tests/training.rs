// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Handle, Runner, Simulation, Streams};
use lockstep_progression::{ProgressEvent, Refusal};
use training::{
    fixture_hash, runner, script, workshop, Event, Intent, Training, DEFAULT_SEED, DEFAULT_STEPS,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn script_streams() -> Streams {
    Streams::new(DEFAULT_SEED ^ 0x0074_7261_696e)
}

fn first(runner: &Runner<Training>) -> Handle {
    runner.simulation().technicians()[0]
}

fn act(runner: &mut Runner<Training>, intent: Intent) -> Vec<Event> {
    runner.step_once(&[intent]).events
}

fn work(runner: &mut Runner<Training>, technician: Handle, trigger: &str, times: u32) {
    let trigger = runner.simulation().graph().trigger_id(trigger).unwrap();
    for _ in 0..times {
        act(
            runner,
            Intent::Work {
                technician,
                trigger,
            },
        );
    }
}

fn study(runner: &mut Runner<Training>, technician: Handle, name: &str) -> Vec<Event> {
    let node = runner.simulation().graph().node_id(name).unwrap();
    act(runner, Intent::Study { technician, node })
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/training.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate, across the scripted session: no technician ever holds both electrical and
/// mechanical, every senior and chief was taken with the experience its gate asks, and the
/// keystone is never unlearned.
#[test]
fn exclusions_lock_and_gates_enforce_across_the_session() {
    let mut runner = runner(workshop(), DEFAULT_SEED);
    let mut streams = script_streams();
    let graph = runner.simulation().graph().clone();
    let id = |name| graph.node_id(name).unwrap();
    let (electrical, mechanical, senior, chief) = (
        id("electrical"),
        id("mechanical"),
        id("senior"),
        id("chief"),
    );
    let (mut chiefs, mut excluded, mut gated) = (0, 0, 0);
    for step in 0..DEFAULT_STEPS {
        let intents = script(runner.simulation(), &mut streams);
        let events = runner.step_once(&intents).events;
        let training = runner.simulation();
        for event in &events {
            match event {
                Event::Progress(ProgressEvent::Unlocked { owner, node })
                    if *node == senior || *node == chief =>
                {
                    let needed = if *node == senior { 50 } else { 80 };
                    let experience = training.value(*owner, "experience").unwrap();
                    assert!(experience >= Fixed32::from_int(needed), "step {step}");
                    chiefs += usize::from(*node == chief);
                }
                Event::Refused {
                    reason: Refusal::ExcludedBy(_),
                    ..
                } => excluded += 1,
                Event::Refused {
                    reason: Refusal::Gate(_),
                    ..
                } => gated += 1,
                Event::Progress(ProgressEvent::Refunded { node, .. }) => {
                    assert_ne!(*node, chief, "step {step}: the keystone came back");
                }
                _ => {}
            }
        }
        for technician in training.technicians() {
            let progress = training.world().progress.get(technician).unwrap();
            assert!(
                !(progress.has_taken(electrical) && progress.has_taken(mechanical)),
                "step {step}: both sides of the exclusion"
            );
        }
    }
    assert!(chiefs > 0, "someone reached the keystone");
    // The session asks for locked and gated nodes too, so both rules are really exercised.
    assert!(excluded > 0, "an exclusion refused a study");
    assert!(gated > 0, "a gate refused a study");
}

#[test]
fn a_technician_meets_the_exclusion_and_the_gate_with_their_reasons() {
    let mut runner = runner(workshop(), 1);
    let ana = first(&runner);
    work(&mut runner, ana, "incident", 4);
    study(&mut runner, ana, "basics");
    study(&mut runner, ana, "electrical");
    let electrical = runner.simulation().graph().node_id("electrical").unwrap();
    assert_eq!(
        study(&mut runner, ana, "mechanical"),
        vec![Event::Refused {
            technician: ana,
            reason: Refusal::ExcludedBy(electrical)
        }]
    );
    study(&mut runner, ana, "diagnostics");
    // Six incidents give 12 experience; senior wants 50.
    let experience = runner.simulation().registry().id("experience").unwrap();
    work(&mut runner, ana, "incident", 2);
    assert_eq!(
        study(&mut runner, ana, "senior"),
        vec![Event::Refused {
            technician: ana,
            reason: Refusal::Gate(experience)
        }]
    );
    // Nineteen more make 50.
    work(&mut runner, ana, "incident", 19);
    let events = study(&mut runner, ana, "senior");
    assert!(matches!(
        events[0],
        Event::Progress(ProgressEvent::Unlocked { .. })
    ));
    assert!(events.len() == 1);
}

#[test]
fn unlearning_gives_half_back_and_takes_the_skill_away() {
    let mut runner = runner(workshop(), 1);
    let ana = first(&runner);
    work(&mut runner, ana, "shift", 3);
    study(&mut runner, ana, "basics");
    study(&mut runner, ana, "electrical");
    // Modifiers raise the maximum; the current value stays where it was.
    let repair_maximum = |runner: &Runner<Training>| {
        let training = runner.simulation();
        let id = training.registry().id("repair").unwrap();
        training
            .world()
            .attributes
            .get(ana)
            .unwrap()
            .get(id)
            .maximum()
    };
    assert_eq!(repair_maximum(&runner), Fixed32::from_int(110));
    let electrical = runner.simulation().graph().node_id("electrical").unwrap();
    let events = act(
        &mut runner,
        Intent::Unlearn {
            technician: ana,
            node: electrical,
        },
    );
    assert_eq!(
        events,
        vec![Event::Progress(ProgressEvent::Refunded {
            owner: ana,
            node: electrical,
            points: 1
        })]
    );
    assert_eq!(repair_maximum(&runner), Fixed32::from_int(100));
    assert_eq!(
        runner
            .simulation()
            .world()
            .progress
            .get(ana)
            .unwrap()
            .points,
        1
    );
}

#[test]
fn experience_marks_are_events() {
    let mut runner = runner(workshop(), 1);
    let ana = first(&runner);
    let trigger = runner.simulation().graph().trigger_id("incident").unwrap();
    let mut events = Vec::new();
    // Forty incidents at 2 experience each pass both 50 and 80.
    for _ in 0..40 {
        events.extend(act(
            &mut runner,
            Intent::Work {
                technician: ana,
                trigger,
            },
        ));
    }
    for mark in ["seasoned", "veteran"] {
        assert!(events.contains(&Event::Mark {
            technician: ana,
            mark: mark.into()
        }));
    }
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    training: &mut Training,
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
    training.step(&mut context, intents);
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let session = runner(workshop(), DEFAULT_SEED);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Training::create(workshop(), &mut Streams::new(5));
    let mut streams = Streams::new(77);
    for number in 0..10_000 {
        let intents = script(&kept, &mut streams);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = Training::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 10_000..20_000 {
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
