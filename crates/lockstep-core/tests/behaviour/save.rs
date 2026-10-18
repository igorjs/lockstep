// SPDX-License-Identifier: Apache-2.0
use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{
    replay_from, saved_simulation_id, Migrations, Recorder, ReplayError, ReplayOutcome, Runner,
    Save, SaveError, SAVE_FORMAT,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn recorder() -> Recorder<Honest> {
    Recorder::new(
        "tally",
        Start { total: 5 },
        9,
        step_configuration(),
        clock_configuration(),
        25,
    )
}

fn run(recorder: &mut Recorder<Honest>, steps: i64) {
    for step in 0..steps {
        recorder.set_clock_multiplier(if step % 40 < 20 { 1.0 } else { 3.5 });
        recorder.step_once(&[Add(step)]);
    }
}

#[test]
fn a_save_loads_to_the_same_hash_and_continues_exactly() {
    let mut original = recorder();
    run(&mut original, 400);
    let bytes = original.save().to_bytes().unwrap();
    let mut loaded =
        Runner::<Honest>::from_save(&Save::from_bytes(&bytes).unwrap(), &Migrations::new())
            .unwrap();
    assert_eq!(loaded.hash(), original.runner().hash());
    assert_eq!(loaded.step_number(), 400);
    // The clock rate, the streams and the exact minutes all carry over: the same inputs from
    // here on give the same hash at every step.
    for step in 400..700 {
        let multiplier = if step % 40 < 20 { 1.0 } else { 3.5 };
        original.set_clock_multiplier(multiplier);
        original.step_once(&[Add(step)]);
        loaded.set_clock_multiplier(multiplier);
        loaded.step_once(&[Add(step)]);
        assert_eq!(loaded.hash(), original.runner().hash());
    }
}

#[test]
fn a_recording_continues_from_a_save_taken_in_the_middle_of_it() {
    let mut original = recorder();
    run(&mut original, 300);
    let save = original.save();
    for step in 300..700 {
        original.set_clock_multiplier(if step % 40 < 20 { 1.0 } else { 3.5 });
        original.step_once(&[Add(step)]);
    }
    let expected = original.runner().hash();
    let recording = original.into_recording();
    let (runner, outcome) = replay_from::<Honest>(&recording, &save, &Migrations::new()).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(runner.hash(), expected);
    assert_eq!(runner.step_number(), 700);
}

#[test]
fn a_save_from_a_different_run_diverges_at_its_own_checkpoint() {
    let mut original = recorder();
    run(&mut original, 300);
    let mut other = Recorder::<Honest>::new(
        "tally",
        Start { total: 5 },
        10,
        step_configuration(),
        clock_configuration(),
        25,
    );
    run(&mut other, 300);
    original.step_once(&[]);
    let recording = original.into_recording();
    let (_, outcome) =
        replay_from::<Honest>(&recording, &other.save(), &Migrations::new()).unwrap();
    assert!(matches!(
        outcome,
        ReplayOutcome::Diverged { at_step: 300, .. }
    ));
}

#[test]
fn a_save_is_refused_when_it_cannot_continue_the_recording() {
    let mut original = recorder();
    run(&mut original, 100);
    let mut save = original.save();
    let recording = original.into_recording();
    save.simulation_id = "another".to_string();
    assert!(matches!(
        replay_from::<Honest>(&recording, &save, &Migrations::new()),
        Err(ReplayError::Save(_))
    ));
    let mut save = recording_save_at(&recording, 100);
    save.step_number = 101;
    assert!(matches!(
        replay_from::<Honest>(&recording, &save, &Migrations::new()),
        Err(ReplayError::Save(_))
    ));
    save.step_number = 100;
    save.snapshot_version = 0;
    assert_eq!(
        replay_from::<Honest>(&recording, &save, &Migrations::new()).err(),
        Some(ReplayError::NotComparable { from_version: 0 })
    );
}

fn recording_save_at(recording: &lockstep_core::Recording<Add>, step: u64) -> Save {
    let mut runner = Runner::<Honest>::new(
        Start { total: 5 },
        recording.seed,
        step_configuration(),
        clock_configuration(),
    );
    for recorded in recording.steps.iter().take(step as usize) {
        runner.set_clock_multiplier(recorded.clock_multiplier);
        runner.step_once(&recorded.intents);
    }
    runner.save("tally")
}

#[test]
fn damaged_bytes_are_errors() {
    let mut original = recorder();
    run(&mut original, 10);
    let bytes = original.save().to_bytes().unwrap();
    assert!(matches!(
        Save::from_bytes(&bytes[..bytes.len() - 1]),
        Err(SaveError::NotASave(_))
    ));
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(matches!(
        Save::from_bytes(&trailing),
        Err(SaveError::NotASave(_))
    ));
    assert!(matches!(
        Save::from_bytes(&[1, 0]),
        Err(SaveError::NotASave(_))
    ));
    let mut future = bytes.clone();
    future[..4].copy_from_slice(&(SAVE_FORMAT + 1).to_le_bytes());
    assert_eq!(
        Save::from_bytes(&future),
        Err(SaveError::UnknownFormat(SAVE_FORMAT + 1))
    );
}

#[test]
fn the_simulation_id_is_read_from_the_front_without_decoding_the_rest() {
    let mut original = recorder();
    run(&mut original, 10);
    let bytes = original.save().to_bytes().unwrap();
    assert_eq!(saved_simulation_id(&bytes).unwrap(), "tally");
    assert!(saved_simulation_id(&bytes[..10]).is_err());
    assert!(saved_simulation_id(&[]).is_err());
}

#[test]
fn a_snapshot_of_the_wrong_shape_is_an_error_not_a_panic() {
    let mut original = recorder();
    run(&mut original, 10);
    let mut save = original.save();
    save.snapshot = vec![1, 2, 3];
    assert!(matches!(
        Runner::<Honest>::from_save(&save, &Migrations::new()),
        Err(SaveError::Snapshot(_))
    ));
}
