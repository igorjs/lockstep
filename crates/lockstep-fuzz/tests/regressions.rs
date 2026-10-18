// SPDX-License-Identifier: Apache-2.0
//! One test per bug the fuzz harness found, each replaying the input that found it.

use ledger::Ledger;
use lockstep_core::replay;

/// A recording with a day of astronomical length and a step of zero seconds overflowed the
/// clock's 64-bit units when replayed (found by `decode_recording`).
#[test]
fn replaying_a_recording_with_an_astronomical_day_does_not_overflow() {
    let mut recording = ledger::record_fixture(7, 6, 3, false);
    recording.clock_configuration.day_length_real_minutes = f32::MAX;
    recording.step_configuration.step_seconds = 0.0;
    let bytes = recording.to_bytes().unwrap();
    let decoded = lockstep_core::Recording::<ledger::Intent>::from_bytes(&bytes).unwrap();
    // The replay may diverge from the recorded hashes; it must not panic.
    let _ = replay::<Ledger>(&decoded);
}
