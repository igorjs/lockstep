// SPDX-License-Identifier: Apache-2.0
//! Recordings as the consumer sees them: one made with the first intent and configuration shapes,
//! committed as bytes, converts and replays on the current code, and a recording of another
//! version is refused naming both versions.

use lockstep_core::{
    recorded_simulation_id, recorded_versions, replay, Message, Migrations, Recording,
    RecordingVersions, ReplayError, ReplayOutcome, RECORDING_MAGIC,
};
use tariffs::{
    configuration_migrations, fixture_hash, intent_migrations, read_recording, Configuration,
    ConfigurationV1, Intent, IntentV1, Tariffs, DEFAULT_SEED, LEGACY_RECORDING_STEPS,
    OPENING_BALANCE_MINOR, SIMULATION_ID,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// A recording of the first shapes in the versioned form, committed as bytes; no test writes it
/// again. Its legacy bytes are what `record` wrote before versions existed.
const LEGACY_RECORDING: &[u8] = include_bytes!("../fixtures/legacy-recording-v1.bin");

#[test]
fn the_committed_old_recording_names_the_first_shapes() {
    assert!(LEGACY_RECORDING.starts_with(&RECORDING_MAGIC));
    assert_eq!(
        recorded_versions(LEGACY_RECORDING).unwrap(),
        RecordingVersions {
            intent: 1,
            event: 1,
            configuration: 1,
        }
    );
    assert_eq!(
        recorded_simulation_id(LEGACY_RECORDING).unwrap(),
        SIMULATION_ID
    );
    let old = Recording::<IntentV1>::from_bytes(LEGACY_RECORDING).unwrap();
    assert_eq!(old.steps.len() as u64, LEGACY_RECORDING_STEPS);
    assert!(!old.checkpoints.is_empty());
    let configuration: ConfigurationV1 = lockstep_core::decode(&old.configuration).unwrap();
    assert_eq!(configuration.opening_balance_minor, OPENING_BALANCE_MINOR);
}

#[test]
fn the_old_recording_converts_and_replays_to_the_committed_hash() {
    let converted = read_recording(LEGACY_RECORDING).unwrap();
    let (runner, outcome) = replay::<Tariffs>(&converted).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    let committed = include_str!("../fixtures/legacy-recording-v1.hash").trim();
    assert_eq!(format!("{:016x}", runner.hash()), committed);
}

#[test]
fn conversion_keeps_the_behaviour_the_same_session_as_a_straight_run() {
    let converted = read_recording(LEGACY_RECORDING).unwrap();
    let (runner, _) = replay::<Tariffs>(&converted).unwrap();
    assert_eq!(
        runner.hash(),
        fixture_hash(DEFAULT_SEED, LEGACY_RECORDING_STEPS)
    );
    let decoded: Configuration = lockstep_core::decode(&converted.configuration).unwrap();
    assert_eq!(
        decoded.opening_balances_minor,
        vec![OPENING_BALANCE_MINOR; decoded.account_names.len()]
    );
    assert!(converted
        .steps
        .iter()
        .all(|step| step.intents.iter().all(|intent| !matches!(
            intent,
            Intent::Transfer {
                waive_fee: true,
                ..
            }
        ))));
}

#[test]
fn the_recorded_hashes_of_the_old_shapes_are_dropped_by_conversion() {
    let converted = read_recording(LEGACY_RECORDING).unwrap();
    assert!(converted.checkpoints.is_empty());
    assert!(converted.snapshots.is_empty());
}

#[test]
fn the_current_reader_refuses_the_old_recording_naming_both_versions() {
    let error = Recording::<Intent>::from_bytes_for::<Tariffs>(LEGACY_RECORDING).unwrap_err();
    assert_eq!(
        error,
        ReplayError::Version {
            kind: "intent",
            found: 1,
            current: <Intent as Message>::VERSION,
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("version 1") && message.contains("version 2"),
        "{message}"
    );
}

#[test]
fn a_newer_recording_is_refused_naming_both_versions() {
    let old = Recording::<IntentV1>::from_bytes(LEGACY_RECORDING).unwrap();
    let newer = old
        .to_versioned_bytes(RecordingVersions {
            intent: 9,
            event: 1,
            configuration: 1,
        })
        .unwrap();
    let error = read_recording(&newer).unwrap_err();
    assert_eq!(
        error,
        ReplayError::Version {
            kind: "intent",
            found: 9,
            current: 2,
        }
    );
}

#[test]
fn a_conversion_chain_with_a_missing_step_is_an_error() {
    let old = Recording::<IntentV1>::from_bytes(LEGACY_RECORDING).unwrap();
    let versions = recorded_versions(LEGACY_RECORDING).unwrap();
    let error = old
        .convert::<Tariffs>(versions, &Migrations::new(), &configuration_migrations())
        .unwrap_err();
    assert!(matches!(&error, ReplayError::Conversion(_)), "{error}");
    let error = old
        .convert::<Tariffs>(versions, &intent_migrations(), &Migrations::new())
        .unwrap_err();
    assert!(matches!(&error, ReplayError::Conversion(_)), "{error}");
}

#[test]
fn a_recording_made_today_replays_without_conversion() {
    let recording = tariffs::record_fixture(DEFAULT_SEED, 500, 100, false);
    let bytes = recording
        .to_versioned_bytes(RecordingVersions::of::<Tariffs>())
        .unwrap();
    let read = read_recording(&bytes).unwrap();
    assert_eq!(read, recording);
    let (_, outcome) = replay::<Tariffs>(&read).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
}

#[test]
fn a_recording_without_the_header_is_read_as_the_first_shapes() {
    let converted = read_recording(&LEGACY_RECORDING[16..]).unwrap();
    assert_eq!(converted, read_recording(LEGACY_RECORDING).unwrap());
}
