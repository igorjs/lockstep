// SPDX-License-Identifier: Apache-2.0
//! Decision 0061: a fixture that runs several separate sessions records as a `RecordingSet`, the
//! simulation id first and then each session with a label, in the order the fixture ran them.
//! Alternatives rejected: one recording that concatenates sessions (they start from different
//! configurations), and one file per session (the fixture hash covers them all).
//! Would change if: a set does not read back as written, or its simulation id is not readable
//! without knowing the simulation.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{recorded_simulation_id, replay, Recorder, RecordingSet, ReplayOutcome};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn session(total: i64) -> lockstep_core::Recording<Add> {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total },
        5,
        step_configuration(),
        clock_configuration(),
        10,
    );
    for step in 0..50 {
        recorder.step_once(&[Add(step)]);
    }
    recorder.into_recording()
}

#[test]
fn a_set_reads_back_with_its_sessions_in_order_and_each_replays() {
    let set = RecordingSet {
        simulation_id: "tally".to_string(),
        sessions: vec![
            ("first".to_string(), session(1)),
            ("second".to_string(), session(2)),
        ],
    };
    let bytes = set.to_bytes().unwrap();
    assert_eq!(recorded_simulation_id(&bytes).unwrap(), "tally");
    let read = RecordingSet::<Add>::from_bytes(&bytes).unwrap();
    assert_eq!(read, set);
    for (_, recording) in &read.sessions {
        let (_, outcome) = replay::<Honest>(recording).unwrap();
        assert_eq!(outcome, ReplayOutcome::Identical);
    }
}
