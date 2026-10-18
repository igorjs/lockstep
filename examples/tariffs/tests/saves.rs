// SPDX-License-Identifier: Apache-2.0
//! Saves as the consumer sees them: an old save committed as bytes still loads, a save taken in
//! the middle of a recording continues it, and neither changes what a transfer costs.

use lockstep_core::{
    decode, replay, replay_from, Message, Recording, ReplayError, ReplayOutcome, Runner, Save,
    SaveError,
};
use tariffs::{
    continue_fixture, migrations, record_fixture, run_fixture_with_fee, Books, BooksV1, Tariffs,
    DEFAULT_SEED, DEFAULT_STEPS, LEGACY_SAVE_STEP, SIMULATION_ID,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// A save written when the books had no fee, committed as bytes. It is what shipped; no test
/// writes it again.
const LEGACY_SAVE: &[u8] = include_bytes!("../fixtures/legacy-v1.bin");

fn legacy() -> Save {
    Save::from_bytes(LEGACY_SAVE).unwrap()
}

#[test]
fn the_committed_old_save_is_the_first_shape_at_the_step_it_was_taken() {
    let save = legacy();
    assert_eq!(save.simulation_id, SIMULATION_ID);
    assert_eq!(save.snapshot_version, <BooksV1 as Message>::VERSION);
    assert_eq!(save.step_number, LEGACY_SAVE_STEP);
    let books: BooksV1 = decode(&save.snapshot).unwrap();
    assert_eq!(books.transfers_applied + books.transfers_rejected, 1_500);
}

#[test]
fn the_old_save_still_loads_and_the_session_ends_on_the_committed_hash() {
    let mut loaded = Runner::<Tariffs>::from_save(&legacy(), &migrations()).unwrap();
    assert_eq!(loaded.step_number(), LEGACY_SAVE_STEP);
    assert_eq!(<Books as Message>::VERSION, 2);
    continue_fixture(&mut loaded, DEFAULT_SEED, DEFAULT_STEPS);
    let committed = include_str!("../fixtures/legacy-v1.hash").trim();
    assert_eq!(format!("{:016x}", loaded.hash()), committed);
}

#[test]
fn loading_the_old_save_keeps_its_behaviour_a_session_with_no_fee_all_along() {
    let loaded = Runner::<Tariffs>::from_save(&legacy(), &migrations()).unwrap();
    assert_eq!(
        loaded.hash(),
        run_fixture_with_fee(DEFAULT_SEED, LEGACY_SAVE_STEP, 0).hash()
    );
    let mut loaded = loaded;
    continue_fixture(&mut loaded, DEFAULT_SEED, DEFAULT_STEPS);
    assert_eq!(
        loaded.hash(),
        run_fixture_with_fee(DEFAULT_SEED, DEFAULT_STEPS, 0).hash()
    );
    assert_eq!(loaded.simulation().books().fees_collected_minor, 0);
}

#[test]
fn without_its_migration_the_old_save_is_refused() {
    assert_eq!(
        Runner::<Tariffs>::from_save(&legacy(), &lockstep_core::Migrations::new()).err(),
        Some(SaveError::MissingMigration { from_version: 1 })
    );
}

#[test]
fn a_save_taken_in_the_middle_of_a_recording_continues_it() {
    let recording = record_fixture(DEFAULT_SEED, 2_000, 100, false);
    let (_, outcome) = replay::<Tariffs>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    // Replay the first 700 steps, save there, and continue from the save.
    let mut head = recording.clone();
    head.steps.truncate(700);
    head.checkpoints.retain(|(step, _)| *step <= 700);
    let (runner, _) = replay::<Tariffs>(&head).unwrap();
    let save = runner.save(SIMULATION_ID);
    let read = Save::from_bytes(&save.to_bytes().unwrap()).unwrap();
    let (resumed, outcome) = replay_from::<Tariffs>(&recording, &read, &migrations()).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(resumed.step_number(), 2_000);
    assert_eq!(resumed.hash(), tariffs::fixture_hash(DEFAULT_SEED, 2_000));
}

#[test]
fn a_recording_cannot_be_checked_against_a_save_of_an_older_shape() {
    let recording: Recording<_> = record_fixture(DEFAULT_SEED, 2_000, 100, false);
    assert_eq!(
        replay_from::<Tariffs>(&recording, &legacy(), &migrations()).err(),
        Some(ReplayError::NotComparable { from_version: 1 })
    );
}
