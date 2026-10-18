// SPDX-License-Identifier: Apache-2.0
//! Decision 0037: a table reload is an intent that carries the new table's text, applied at a step
//! boundary like any other intent, so a recording holds it and a replay swaps at the same step.
//! Alternative rejected: the host replacing the table between steps with no intent. The recording
//! would not know, and a replay would run every step on the first table.
//! Would change if: a recording with a swap in it replays to a different hash than the live run, or
//! a replay without the swap intent matches the live run.

use crate::common::{recorded, Counter, Intent, LARGE};
use lockstep_core::{replay, ReplayOutcome};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_recording_with_a_swap_replays_to_the_live_hash() {
    let recording = recorded(LARGE);
    assert!(recording
        .steps
        .iter()
        .flat_map(|step| step.intents.iter())
        .any(|intent| matches!(intent, Intent::Swap(_))));
    let (replayed, outcome) = replay::<Counter>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    // Five steps of 1, then five of 10.
    assert_eq!(replayed.simulation().total, 5 + 50);
}

#[test]
fn a_replay_without_the_swap_intent_diverges() {
    // What a host that swapped the table outside the recording would leave behind: the same steps
    // with no swap in them. The replay stays on the first table and the checkpoints disagree.
    let mut recording = recorded(LARGE);
    for step in recording.steps.iter_mut() {
        step.intents
            .retain(|intent| !matches!(intent, Intent::Swap(_)));
    }
    let (_, outcome) = replay::<Counter>(&recording).unwrap();
    assert!(matches!(outcome, ReplayOutcome::Diverged { .. }));
}
