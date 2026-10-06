// SPDX-License-Identifier: Apache-2.0
//! Decision: the timeline is a projection of events, rebuilt from a recording's inputs, and never
//! part of the state hash. Its times are the clock's whole minute, an integer, not the float minute.
//! Alternative rejected: keeping the event log in the snapshot, which makes every save and hash
//! grow with history and turns a journal format change into a save migration.
//! Would change if: keeping or dropping a timeline changes a runner's hash, or a rebuilt timeline
//! differs from the live one.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Noted, Start};
use lockstep_core::{Recorder, Timeline};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_timeline_changes_no_hash_and_rebuilds_from_inputs_alone() {
    let new_recorder = || {
        Recorder::<Honest>::new(
            "tally",
            Start { total: 1 },
            21,
            step_configuration(),
            clock_configuration(),
            500,
        )
    };
    let (mut with_timeline, mut without) = (new_recorder(), new_recorder());
    let mut timeline: Timeline<Noted> = Timeline::new();
    for step in 0..3_000 {
        let advanced = with_timeline.step_once(&[Add(step % 11)]);
        let runner = with_timeline.runner();
        timeline.append(runner.step_number() - 1, runner.clock(), &advanced.events);
        without.step_once(&[Add(step % 11)]);
    }
    assert_eq!(with_timeline.runner().hash(), without.runner().hash());
    assert_eq!(with_timeline.recording(), without.recording());
    let (rebuilt, outcome) = Timeline::rebuild_from::<Honest>(without.recording()).unwrap();
    assert_eq!(outcome, lockstep_core::ReplayOutcome::Identical);
    assert_eq!(rebuilt, timeline);
    assert!(timeline
        .entries()
        .iter()
        .all(|entry| entry.minute_of_day < 1_440));
}
