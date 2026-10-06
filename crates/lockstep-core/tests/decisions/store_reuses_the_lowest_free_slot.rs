//! Decision: an insert reuses the lowest vacant slot, and a store derives its vacant set from its
//! slots when it is loaded.
//! Alternative rejected: wrapping a slot map, whose reuse order depends on the order slots were
//! freed and is not part of its saved form. A restored store then handed out a different handle
//! than the original on the next insert.
//! Would change if: a restore at any point of any history changes any later handle (the number to
//! beat is zero differing handles over 50 random histories).

use lockstep_core::{Handle, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_lowest_vacant_slot_is_reused_whatever_order_slots_were_freed() {
    let mut forward = StableVector::new();
    let forward_handles: Vec<Handle> = (0..6).map(|value| forward.insert(value)).collect();
    forward.remove(forward_handles[1]);
    forward.remove(forward_handles[4]);

    let mut backward = StableVector::new();
    let backward_handles: Vec<Handle> = (0..6).map(|value| backward.insert(value)).collect();
    backward.remove(backward_handles[4]);
    backward.remove(backward_handles[1]);

    assert_eq!(forward.insert(9).slot_index(), 1);
    assert_eq!(backward.insert(9).slot_index(), 1);
    assert_eq!(forward.insert(9).slot_index(), 4);
    assert_eq!(backward.insert(9).slot_index(), 4);
    assert_eq!(forward.insert(9).slot_index(), 6);
}

#[test]
fn a_reused_slot_gets_a_newer_generation() {
    let mut entities = StableVector::new();
    let first = entities.insert(1);
    entities.remove(first);
    let second = entities.insert(2);
    assert_eq!(second.slot_index(), first.slot_index());
    assert!(second.generation() > first.generation());
}
