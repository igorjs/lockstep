// SPDX-License-Identifier: Apache-2.0
//! The versioned form of a recording: a 16 byte header, then the legacy bytes unchanged.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{
    recorded_simulation_id, recorded_versions, replay, Recorder, Recording, RecordingVersions,
    ReplayError, ReplayOutcome, DECODE_LIMIT, RECORDING_HEADER_LENGTH, RECORDING_MAGIC,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn recording() -> Recording<Add> {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 5 },
        9,
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
fn the_header_is_sixteen_bytes_the_magic_then_three_little_endian_versions() {
    let versions = RecordingVersions {
        intent: 7,
        event: 8,
        configuration: 9,
    };
    let recording = recording();
    let legacy = recording.to_bytes().unwrap();
    let bytes = recording.to_versioned_bytes(versions).unwrap();
    assert_eq!(RECORDING_HEADER_LENGTH, 16);
    assert_eq!(bytes.len(), 16 + legacy.len());
    assert_eq!(&bytes[..4], b"LKRC");
    assert_eq!(&bytes[..4], &RECORDING_MAGIC);
    assert_eq!(bytes[4..8], 7u32.to_le_bytes());
    assert_eq!(bytes[8..12], 8u32.to_le_bytes());
    assert_eq!(bytes[12..16], 9u32.to_le_bytes());
    assert_eq!(&bytes[16..], &legacy[..]);
    assert_eq!(recorded_versions(&bytes).unwrap(), versions);
}

#[test]
fn the_magic_read_as_a_length_is_past_the_decode_limit() {
    // So it can never be the length prefix of a legacy recording's first field.
    let as_length = u32::from_le_bytes(RECORDING_MAGIC) as u64;
    assert!(as_length > DECODE_LIMIT);
}

#[test]
fn legacy_bytes_read_as_version_zero_and_both_forms_read_back_equal() {
    let recording = recording();
    let legacy = recording.to_bytes().unwrap();
    assert_eq!(
        recorded_versions(&legacy).unwrap(),
        RecordingVersions::LEGACY
    );
    let versioned = recording
        .to_versioned_bytes(RecordingVersions::of::<Honest>())
        .unwrap();
    assert_eq!(Recording::<Add>::from_bytes(&legacy).unwrap(), recording);
    assert_eq!(Recording::<Add>::from_bytes(&versioned).unwrap(), recording);
    let (_, outcome) =
        replay::<Honest>(&Recording::<Add>::from_bytes(&versioned).unwrap()).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
}

#[test]
fn the_simulation_id_reads_from_both_forms() {
    let recording = recording();
    let legacy = recording.to_bytes().unwrap();
    let versioned = recording
        .to_versioned_bytes(RecordingVersions::of::<Honest>())
        .unwrap();
    assert_eq!(recorded_simulation_id(&legacy).unwrap(), "tally");
    assert_eq!(recorded_simulation_id(&versioned).unwrap(), "tally");
    assert!(recorded_simulation_id(&versioned[..10]).is_err());
    assert!(recorded_simulation_id(&versioned[..17]).is_err());
}

#[test]
fn a_cut_header_is_not_a_recording() {
    assert!(matches!(
        Recording::<Add>::from_bytes(b"LKRC\x01\x00"),
        Err(ReplayError::NotARecording(_))
    ));
}

#[test]
fn from_bytes_for_accepts_current_and_legacy_and_refuses_a_newer_shape_naming_both() {
    let recording = recording();
    let current = RecordingVersions::of::<Honest>();
    let versioned = recording.to_versioned_bytes(current).unwrap();
    assert_eq!(
        Recording::<Add>::from_bytes_for::<Honest>(&versioned).unwrap(),
        recording
    );
    let legacy = recording.to_bytes().unwrap();
    assert_eq!(
        Recording::<Add>::from_bytes_for::<Honest>(&legacy).unwrap(),
        recording
    );
    let newer = RecordingVersions {
        intent: current.intent + 1,
        ..current
    };
    let error =
        Recording::<Add>::from_bytes_for::<Honest>(&recording.to_versioned_bytes(newer).unwrap())
            .unwrap_err();
    assert_eq!(
        error,
        ReplayError::Version {
            kind: "intent",
            found: current.intent + 1,
            current: current.intent,
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("version 2") && message.contains("version 1"),
        "{message}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn the_decode_limit_applies_to_the_total_with_the_header() {
    let mut recording = recording();
    let versions = RecordingVersions::of::<Honest>();
    recording.snapshots.push((0, Vec::new()));
    let base = recording.to_bytes().unwrap().len() as u64;
    // One byte past limit minus header: `to_bytes` alone still accepts it, the total does not.
    let fits = (DECODE_LIMIT - RECORDING_HEADER_LENGTH as u64 - base) as usize;
    recording.snapshots[0].1 = vec![0; fits + 1];
    assert!(base + fits as u64 + 1 <= DECODE_LIMIT);
    assert_eq!(
        recording.to_versioned_bytes(versions),
        Err(ReplayError::TooLarge(DECODE_LIMIT + 1))
    );
}
