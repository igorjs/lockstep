// SPDX-License-Identifier: Apache-2.0
use audit::{
    books, fixture_hash, runner, script, Audit, Event, Intent, DEFAULT_SEED, DEFAULT_STEPS,
};
use lockstep_core::{Runner, Simulation, Streams};
use lockstep_knowledge::KnowledgeEvent;
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn script_streams() -> Streams {
    Streams::new(DEFAULT_SEED ^ 0x0061_7564_6974)
}

/// Every fragment the auditor holds for the question's facts.
fn gathered(runner: &Runner<Audit>) -> u32 {
    let audit = runner.simulation();
    let mind = audit.world().knowledge.get(audit.auditor()).unwrap();
    audit.catalogue().questions[0]
        .facts
        .iter()
        .map(|fact| mind.fragments_of(*fact) as u32)
        .sum()
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/audit.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: the first question opens on the step its second fragment arrives, and not before.
#[test]
fn the_first_question_opens_after_two_fragments() {
    let mut runner = runner(books(), DEFAULT_SEED);
    let mut streams = script_streams();
    for step in 0..DEFAULT_STEPS {
        let before = gathered(&runner);
        let intents = script(runner.simulation(), &mut streams);
        let events = runner.step_once(&intents).events;
        let after = gathered(&runner);
        let opened = events
            .iter()
            .any(|event| matches!(event, Event::Knowledge(KnowledgeEvent::Opened { .. })));
        if opened {
            assert!(
                before < 2 && after >= 2,
                "step {step}: opened at {before} to {after}"
            );
            return;
        }
        assert!(
            after < 2,
            "step {step}: {after} fragments and nothing opened"
        );
    }
    panic!("the question never opened");
}

/// No finding fires twice, even with the session saved and loaded half way.
#[test]
fn nothing_fires_twice_across_a_save() {
    let mut runner = runner(books(), DEFAULT_SEED);
    let mut streams = script_streams();
    let mut fired: BTreeMap<u16, u32> = BTreeMap::new();
    let count = |events: &[Event], fired: &mut BTreeMap<u16, u32>| {
        for event in events {
            if let Event::Knowledge(KnowledgeEvent::Fired { rule, .. }) = event {
                *fired.entry(rule.0).or_insert(0) += 1;
            }
        }
    };
    for _ in 0..DEFAULT_STEPS / 2 {
        let intents = script(runner.simulation(), &mut streams);
        let events = runner.step_once(&intents).events;
        count(&events, &mut fired);
    }
    let saved = lockstep_core::encode(&runner.simulation().snapshot());
    let mut loaded = Audit::restore(lockstep_core::decode(&saved).unwrap());
    let mut clock = runner.clock().clone();
    let mut randomness = Streams::new(1);
    for number in DEFAULT_STEPS / 2..DEFAULT_STEPS * 2 {
        let intents = script(&loaded, &mut streams);
        let (minutes, exact) = clock.advance_exactly(&mut Vec::new());
        let mut events = Vec::new();
        let mut context = lockstep_core::Context {
            clock: &clock,
            elapsed_game_minutes: minutes,
            elapsed_minutes: exact,
            randomness: &mut randomness,
            events: &mut events,
            step_number: number,
            step_seconds: 1.0 / 30.0,
        };
        loaded.step(&mut context, &intents);
        count(&events, &mut fired);
    }
    assert!(fired.len() >= 3, "the findings fired: {fired:?}");
    assert!(fired.values().all(|times| *times == 1), "{fired:?}");
}

#[test]
fn a_clean_day_earns_nothing_to_see() {
    // Daytime transfers of odd amounts from one account to another, both ways: no fragments.
    let mut runner = runner(books(), 1);
    let accounts = runner.simulation().accounts();
    let mut events = Vec::new();
    for index in 0..20 {
        let (from, to) = if index % 2 == 0 {
            (accounts[0], accounts[1])
        } else {
            (accounts[1], accounts[0])
        };
        events.extend(
            runner
                .step_once(&[Intent::Transfer {
                    from,
                    to,
                    amount_minor: 1_234,
                }])
                .events,
        );
    }
    let nothing_to_see = runner
        .simulation()
        .catalogue()
        .rule_id("nothing_to_see")
        .unwrap();
    let fired: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Knowledge(KnowledgeEvent::Fired { rule, .. }) => Some(*rule),
            _ => None,
        })
        .collect();
    assert_eq!(fired, vec![nothing_to_see]);
}

#[test]
fn a_transfer_to_oneself_or_from_the_auditor_is_refused() {
    let mut runner = runner(books(), 1);
    let accounts = runner.simulation().accounts();
    let auditor = runner.simulation().auditor();
    let events = runner
        .step_once(&[
            Intent::Transfer {
                from: accounts[0],
                to: accounts[0],
                amount_minor: 100,
            },
            Intent::Transfer {
                from: auditor,
                to: accounts[0],
                amount_minor: 100,
            },
            Intent::Transfer {
                from: accounts[0],
                to: accounts[1],
                amount_minor: 0,
            },
        ])
        .events;
    assert_eq!(events, vec![Event::Refused, Event::Refused, Event::Refused]);
    assert_eq!(runner.simulation().world().transfers, 0);
}

#[test]
#[should_panic(expected = "another catalogue")]
fn a_save_from_another_catalogue_is_refused_at_load() {
    let audit = Audit::create(books(), &mut Streams::new(1));
    let mut world = audit.world().clone();
    world.catalogue_hash ^= 1;
    Audit::restore(world);
}
