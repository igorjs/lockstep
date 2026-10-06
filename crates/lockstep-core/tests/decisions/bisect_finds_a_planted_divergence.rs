// SPDX-License-Identifier: Apache-2.0
//! Decision: a recording holds only the inputs (intents and the clock multiplier per step) plus a
//! hash at regular checkpoints. Replaying it finds the first checkpoint where the world differs,
//! and the snapshots a recording may keep show what differs there.
//! Alternative rejected: recording every step's state, which turns a ten hour session from
//! megabytes into gigabytes; and recording events, which a rule change would also change.
//! Would change if: a planted rule change is not reported at the first checkpoint after it, or
//! checkpoints every step do not name the exact step.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start, Tally};
use lockstep_core::{bisect, replay, Recorder, Recording, ReplayOutcome};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// The step whose rule the planted build changes.
const PLANTED: u64 = 1_234;
type Planted = Tally<PLANTED>;

fn record(every: u64) -> Recording<Add> {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 0 },
        7,
        step_configuration(),
        clock_configuration(),
        every,
    )
    .keep_snapshots();
    for step in 0..3_000 {
        recorder.step_once(&[Add(step % 5)]);
    }
    recorder.into_recording()
}

#[test]
fn the_honest_build_replays_its_own_recording_exactly() {
    let (_, outcome) = replay::<Honest>(&record(100)).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
}

#[test]
fn bisect_finds_a_planted_divergence_at_the_first_checkpoint_after_it() {
    let recording = record(100);
    let found = bisect::<Planted>(&recording)
        .unwrap()
        .expect("a divergence");
    // The planted step runs as step 1,234, so the runner first differs at step number 1,235.
    assert_eq!((found.last_good_step, found.first_bad_step), (1_200, 1_300));
    assert_eq!(
        found.differences,
        [
            format!("- total: {},", recorded_total(&recording, 1_300)),
            format!("+ total: {},", recorded_total(&recording, 1_300) + 66),
        ]
    );
}

#[test]
fn checkpoints_every_step_name_the_exact_step() {
    let found = bisect::<Planted>(&record(1))
        .unwrap()
        .expect("a divergence");
    assert_eq!(
        (found.last_good_step, found.first_bad_step),
        (PLANTED, PLANTED + 1)
    );
}

fn recorded_total(recording: &Recording<Add>, step: u64) -> i64 {
    let (_, bytes) = recording
        .snapshots
        .iter()
        .find(|(at, _)| *at == step)
        .expect("a kept snapshot");
    lockstep_core::decode::<crate::common::tally::State>(bytes)
        .unwrap()
        .total
}
