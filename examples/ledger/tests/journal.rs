// SPDX-License-Identifier: Apache-2.0
//! The ledger as the consumer of record, replay and the timeline: a recorded session replays to
//! the fixture hash and renders as a journal.

use ledger::{
    fixture_hash, journal, record_fixture, Event, Ledger, Reason, DEFAULT_SEED, SIMULATION_ID,
};
use lockstep_core::{bisect, replay, Indexable, Recording, ReplayOutcome, Timeline};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const STEPS: u64 = 2_000;

#[test]
fn a_recorded_session_replays_to_the_fixture_hash() {
    let recording = record_fixture(DEFAULT_SEED, STEPS, 250, false);
    assert_eq!(recording.simulation_id, SIMULATION_ID);
    assert_eq!(recording.steps.len() as u64, STEPS);
    // The recording survives its bytes, then replays to the same world as the plain fixture run.
    let read = Recording::from_bytes(&recording.to_bytes().unwrap()).unwrap();
    let (runner, outcome) = replay::<Ledger>(&read).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(runner.hash(), fixture_hash(DEFAULT_SEED, STEPS));
    assert_eq!(bisect::<Ledger>(&read).unwrap(), None);
}

#[test]
fn the_journal_of_a_replay_matches_the_live_session_line_for_line() {
    let recording = record_fixture(DEFAULT_SEED, STEPS, 250, false);
    // Live: run the recorded inputs by hand and append each step's events.
    let mut runner = ledger::runner(DEFAULT_SEED);
    let mut live: Timeline<Event> = Timeline::new();
    for step in &recording.steps {
        let advanced = runner.step_once(&step.intents);
        live.append(runner.step_number() - 1, runner.clock(), &advanced.events);
    }
    let (rebuilt, outcome) = Timeline::rebuild_from::<Ledger>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(rebuilt, live);
    let books = runner.simulation().books();
    let lines = journal(&rebuilt, books);
    assert_eq!(lines, journal(&live, books));
    assert_eq!(lines.len(), STEPS as usize, "one event per step");
    assert!(lines[0].starts_with("day 0 00:00  #0 "), "{}", lines[0]);
    assert!(lines
        .iter()
        .any(|line| line.contains(" moved from account-")));
    assert!(lines
        .iter()
        .any(|line| line.contains("rejected: InsufficientFunds")));
    // A ledger game minute is a real minute, 1,800 steps, and an entry's time is read when its
    // step ends: the 1,800th step (step 1,799) is the first at 00:01.
    assert!(lines[1_798].starts_with("day 0 00:00"), "{}", lines[1_798]);
    assert!(lines[1_799].starts_with("day 0 00:01"), "{}", lines[1_799]);
    assert!(lines.last().unwrap().starts_with("day 0 00:01"));
}

#[test]
fn the_timeline_counts_agree_with_the_books() {
    let recording = record_fixture(DEFAULT_SEED, STEPS, 250, false);
    let (runner, _) = replay::<Ledger>(&recording).unwrap();
    let books = runner.simulation().books();
    let (timeline, _) = Timeline::rebuild_from::<Ledger>(&recording).unwrap();
    let kind = |event: Event| event.kind();
    let rejected = timeline
        .of_kind(kind(Event::Rejected {
            reason: Reason::Overflow,
        }))
        .count();
    assert_eq!(rejected as u64, books.transfers_rejected);
    let account = books.accounts.handles()[3];
    let by_hand = timeline
        .entries()
        .iter()
        .filter(|entry| match entry.event {
            Event::Deposited {
                account: target, ..
            } => target == account,
            Event::Transferred { from, to, .. } => from == account || to == account,
            Event::Rejected { .. } => false,
        })
        .count();
    assert_eq!(timeline.for_entity(account).count(), by_hand);
}
