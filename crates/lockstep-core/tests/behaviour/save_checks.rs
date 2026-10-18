// SPDX-License-Identifier: Apache-2.0
//! A save is checked against the recording it continues, and a damaged one is an error.

use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{
    replay_from, Migrations, Recorder, ReplayError, ReplayOutcome, Save, SaveError,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn recorder(seed: u64) -> Recorder<Honest> {
    Recorder::new(
        "tally",
        Start { total: 5 },
        seed,
        step_configuration(),
        clock_configuration(),
        25,
    )
}

fn drive(recorder: &mut Recorder<Honest>, from: i64, to: i64) {
    for step in from..to {
        recorder.set_clock_multiplier(if step % 40 < 20 { 1.0 } else { 3.5 });
        recorder.step_once(&[Add(step)]);
    }
}

#[test]
fn a_save_between_checkpoints_is_still_checked_against_the_recording() {
    // Checkpoints fall every 25 steps; 307 is not one of them.
    let mut original = recorder(9);
    drive(&mut original, 0, 307);
    let mut other = recorder(10);
    drive(&mut other, 0, 307);
    let (good, bad) = (original.save(), other.save());
    drive(&mut original, 307, 500);
    let recording = original.into_recording();
    let (_, outcome) = replay_from::<Honest>(&recording, &good, &Migrations::new()).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    let (_, outcome) = replay_from::<Honest>(&recording, &bad, &Migrations::new()).unwrap();
    assert!(matches!(
        outcome,
        ReplayOutcome::Diverged { at_step: 307, .. }
    ));
}

#[test]
fn a_save_with_other_step_settings_than_the_recording_is_refused() {
    let mut original = recorder(9);
    drive(&mut original, 0, 50);
    let mut save = original.save();
    save.step_configuration.step_seconds *= 2.0;
    let recording = original.into_recording();
    assert!(matches!(
        replay_from::<Honest>(&recording, &save, &Migrations::new()),
        Err(ReplayError::Save(_))
    ));
}

#[test]
fn a_clock_that_no_runner_could_have_written_is_an_error_not_a_later_panic() {
    let mut original = recorder(9);
    drive(&mut original, 0, 10);
    let mut bytes = original.save().to_bytes().unwrap();
    // The clock's units per day follow the format (4), the id (8 + 5), the snapshot version (4),
    // the step settings (8), the clock settings (20), the step (8) and the clock's settings (20).
    let at = 4 + 13 + 4 + 8 + 20 + 8 + 20;
    assert_eq!(
        u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) % 65_536,
        0
    );
    bytes[at..at + 8].copy_from_slice(&0u64.to_le_bytes());
    assert!(matches!(
        Save::from_bytes(&bytes),
        Err(SaveError::NotASave(_))
    ));
}
