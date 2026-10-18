// SPDX-License-Identifier: Apache-2.0
//! Decision 0036: a save is taken at a step boundary and holds the runner's hashed state (step,
//! clock, streams, snapshot), nothing more. A recording continues from it by skipping the steps
//! before the save and comparing every checkpoint from the save's step on, so the save itself
//! is checked against the recording first.
//! Alternative rejected: saving the accumulator of unspent real time and the queued intents.
//! Neither is in the runner hash, a replay never sees either, and a save mid-frame would make
//! two saves of the same step differ.
//! Would change if: saving and loading changes the hash at any step, or a save that differs
//! from the recording at its own step continues without a divergence.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{replay, replay_from, Migrations, Recorder, ReplayOutcome};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn saving_in_the_middle_and_continuing_the_recording_ends_on_the_same_hash() {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 5 },
        77,
        step_configuration(),
        clock_configuration(),
        100,
    );
    let mut saves = Vec::new();
    for step in 0..1_000 {
        if step % 250 == 0 {
            saves.push(recorder.save());
        }
        recorder.set_clock_multiplier(1.0 + (step % 5) as f32);
        recorder.queue([Add(step)]);
        // Frames of 40 milliseconds leave a remainder unspent, which a save must not need.
        recorder.advance(0.04);
    }
    let recording = recorder.into_recording();
    let (full, outcome) = replay::<Honest>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert!(recording.steps.len() >= 900);
    for save in saves {
        let (resumed, outcome) =
            replay_from::<Honest>(&recording, &save, &Migrations::new()).unwrap();
        assert_eq!(outcome, ReplayOutcome::Identical);
        assert_eq!(resumed.hash(), full.hash());
        assert_eq!(resumed.step_number(), full.step_number());
    }
}
