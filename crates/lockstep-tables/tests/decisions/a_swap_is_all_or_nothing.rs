// SPDX-License-Identifier: Apache-2.0
//! Decision 0038: a table lives in the simulation's state and a swap is checked whole before it is
//! applied. A swap that fails any check changes nothing, and a swap with the same content changes
//! nothing either. Because the table is state, a save taken after a swap restores the swapped
//! table.
//! Alternative rejected: applying the rows that pass and skipping the rest, which leaves a table
//! that no file ever described.
//! Would change if: a rejected swap changes the state hash, or a restored save runs on a table
//! other than the one it was saved with.

use crate::common::{runner, swap, Counter, Intent, BROKEN, LARGE};
use lockstep_core::Runner;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_rejected_swap_leaves_the_table_and_the_totals_alone() {
    let mut with = runner();
    let mut without = runner();
    with.step_once(&[Intent::Add]);
    without.step_once(&[Intent::Add]);
    let events = with.step_once(&[swap(BROKEN)]).events;
    without.step_once(&[]);
    assert!(!events[0].swapped);
    assert_eq!(with.hash(), without.hash());
}

#[test]
fn a_save_after_a_swap_restores_the_swapped_table() {
    let mut live = runner();
    live.step_once(&[swap(LARGE), Intent::Add]);
    let save = live.save("counter");
    let mut restored =
        Runner::<Counter>::from_save(&save, &lockstep_core::Migrations::new()).unwrap();
    live.step_once(&[Intent::Add]);
    restored.step_once(&[Intent::Add]);
    assert_eq!(restored.simulation().total, 20);
    assert_eq!(restored.hash(), live.hash());
}
