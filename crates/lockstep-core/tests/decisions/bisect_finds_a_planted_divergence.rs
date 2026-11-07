// SPDX-License-Identifier: Apache-2.0
//! Decision: a recording holds only the inputs (intents and the clock multiplier per step) plus a
//! hash at regular checkpoints. Replaying it finds the first checkpoint where the world differs,
//! and the snapshots a recording may keep show what differs there.
//! Alternative rejected: recording every step's state, which turns a ten hour session from
//! megabytes into gigabytes; and recording events, which a rule change would also change.
//! Would change if: a planted rule change is not reported at the first checkpoint after it, or
//! checkpoints every step do not name the exact step (`exact_step`, decision 0079).

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
    assert_eq!(
        (found.last_good_step, found.first_bad_step),
        (Some(1_200), 1_300)
    );
    assert_eq!(
        found.differences,
        [
            format!("-     total: {},", recorded_total(&recording, 1_300)),
            format!("+     total: {},", recorded_total(&recording, 1_300) + 66),
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
        (Some(PLANTED), PLANTED + 1)
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

#[test]
fn a_change_in_create_is_found_at_step_zero_with_nothing_good_before_it() {
    // A planted step of 0 changes the very first step, but the world at step 0 is the same, so use
    // a recording whose starting total differs instead: the step 0 checkpoint itself differs.
    let mut recording = record(100);
    recording.configuration = lockstep_core::encode(&Start { total: 1 });
    let found = bisect::<Honest>(&recording).unwrap().expect("a divergence");
    assert_eq!((found.last_good_step, found.first_bad_step), (None, 0));
}

/// Records with a hash every step, snapshots every 100, and checks the exact step of a planted
/// change at each listed step.
macro_rules! exact_step_table {
    ($($planted:literal),*) => {{
        let recording = record_with(1, 100);
        $(
            let found = bisect::<Tally<$planted>>(&recording)
                .unwrap()
                .expect("a divergence");
            assert_eq!(found.exact_step, Some($planted), "planted at {}", $planted);
            assert_eq!(found.checkpoint_interval(), Some(1));
        )*
    }};
}

fn record_with(every: u64, snapshot_every: u64) -> Recording<Add> {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 0 },
        7,
        step_configuration(),
        clock_configuration(),
        every,
    )
    .keep_snapshots()
    .snapshot_every(snapshot_every);
    for step in 0..3_000 {
        recorder.step_once(&[Add(step % 5)]);
    }
    recorder.into_recording()
}

#[test]
fn an_exact_step_is_named_for_each_planted_change() {
    exact_step_table!(1, 2, 100, 101, 1_200, 1_234);
}

#[test]
fn a_change_in_create_has_no_exact_step() {
    let mut recording = record_with(1, 100);
    recording.configuration = lockstep_core::encode(&Start { total: 1 });
    let found = bisect::<Honest>(&recording).unwrap().expect("a divergence");
    assert_eq!((found.first_bad_step, found.exact_step), (0, None));
}

#[test]
fn wider_checkpoints_give_the_interval_and_no_exact_step() {
    for every in [10, 100] {
        let found = bisect::<Planted>(&record_with(every, every))
            .unwrap()
            .expect("a divergence");
        assert_eq!(found.exact_step, None, "every {every}");
        assert_eq!(found.checkpoint_interval(), Some(every), "every {every}");
    }
    let found = bisect::<Planted>(&record_with(1, 100))
        .unwrap()
        .expect("a divergence");
    assert_eq!(found.exact_step, Some(PLANTED));
}

#[test]
fn snapshots_stay_coarse_while_hashes_cover_every_step() {
    let recording = record_with(1, 100);
    assert_eq!(recording.checkpoints.len(), 3_001);
    assert_eq!(recording.snapshots.len(), 31);
    // Between kept snapshots bisect has no snapshot to compare, and says so.
    let found = bisect::<Planted>(&recording)
        .unwrap()
        .expect("a divergence");
    assert!(!found.compared_snapshots);
    assert!(found.differences.is_empty());
}
