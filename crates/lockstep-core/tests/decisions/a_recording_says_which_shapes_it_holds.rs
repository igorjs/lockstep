// SPDX-License-Identifier: Apache-2.0
//! Decision 0085: a recording can carry a 16 byte header (the magic `LKRC`, then the versions of
//! its intent, event and configuration shapes) before its legacy bytes, so an older shape can be
//! converted. `to_bytes` and every recording committed before the header stay as they were, and
//! read as version 0. A converted recording keeps no checkpoints, since their hashes cover the old
//! shapes. A `RecordingSet` stays unversioned: each session of a set is read by the code that
//! knows its variant, so a header on the set would name no one shape.
//! Alternative rejected: changing `to_bytes` to write the header, which would change every
//! committed recording and its fixture hash for no gain over reading both forms.
//! Would change if: `to_bytes` output changes, a legacy recording stops reading, or a converted
//! recording compares a hash recorded under an old shape.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{
    recorded_versions, replay, Message, Migrations, RecordedStep, Recorder, Recording,
    RecordingSet, RecordingVersions, ReplayError, ReplayOutcome, RECORDING_MAGIC,
};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// The first shape of the intent: the amount was a 32-bit number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 0)]
struct SmallAdd(i32);

fn current_recording() -> Recording<Add> {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 5 },
        4,
        step_configuration(),
        clock_configuration(),
        10,
    );
    for step in 0..40 {
        recorder.step_once(&[Add(step)]);
    }
    recorder.into_recording()
}

fn old_recording() -> Recording<SmallAdd> {
    let current = current_recording();
    Recording {
        simulation_id: current.simulation_id,
        configuration: current.configuration,
        seed: current.seed,
        step_configuration: current.step_configuration,
        clock_configuration: current.clock_configuration,
        steps: current
            .steps
            .iter()
            .map(|step| RecordedStep {
                intents: step
                    .intents
                    .iter()
                    .map(|Add(amount)| SmallAdd(*amount as i32))
                    .collect(),
                clock_multiplier: step.clock_multiplier,
            })
            .collect(),
        checkpoints: current.checkpoints,
        snapshots: Vec::new(),
    }
}

#[test]
fn the_legacy_bytes_are_the_body_of_the_versioned_form() {
    let recording = current_recording();
    let legacy = recording.to_bytes().unwrap();
    let versioned = recording
        .to_versioned_bytes(RecordingVersions::of::<Honest>())
        .unwrap();
    assert!(!legacy.starts_with(&RECORDING_MAGIC));
    assert_eq!(&versioned[16..], &legacy[..]);
}

#[test]
fn a_recording_set_is_not_versioned() {
    let set = RecordingSet {
        simulation_id: "tally".to_string(),
        sessions: vec![("only".to_string(), current_recording())],
    };
    let bytes = set.to_bytes().unwrap();
    assert!(!bytes.starts_with(&RECORDING_MAGIC));
    assert_eq!(
        recorded_versions(&bytes).unwrap(),
        RecordingVersions::LEGACY
    );
    assert_eq!(RecordingSet::<Add>::from_bytes(&bytes).unwrap(), set);
}

#[test]
fn an_old_intent_converts_and_replays_without_the_old_hashes() {
    // Version 1 of Add is the current shape; version 0 is the old one, widened by one step.
    let migrations = Migrations::new().step(|old: SmallAdd| Add(old.0 as i64));
    let bytes = old_recording()
        .to_versioned_bytes(RecordingVersions {
            intent: 0,
            event: 1,
            configuration: 1,
        })
        .unwrap();
    let versions = recorded_versions(&bytes).unwrap();
    let converted = Recording::<SmallAdd>::from_bytes(&bytes)
        .unwrap()
        .convert::<Honest>(versions, &migrations, &Migrations::new())
        .unwrap();
    assert!(converted.checkpoints.is_empty());
    assert_eq!(converted.steps, current_recording().steps);
    let (runner, outcome) = replay::<Honest>(&converted).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    let straight = replay::<Honest>(&current_recording()).unwrap().0;
    assert_eq!(runner.hash(), straight.hash());
    // Without the step the old shape cannot be reached.
    assert!(matches!(
        Recording::<SmallAdd>::from_bytes(&bytes)
            .unwrap()
            .convert::<Honest>(versions, &Migrations::new(), &Migrations::new()),
        Err(ReplayError::Conversion(_))
    ));
}
