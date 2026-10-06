// SPDX-License-Identifier: Apache-2.0
//! Spike: does a restored store continue exactly as the original would have?
//!
//! A save is taken, loaded, and both copies then insert new entities. If the stores hand out
//! different handles, a replay that starts from a save would diverge from a straight run.

use lockstep_core::{hash_of, Handle, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_restored_store_hands_out_the_same_handles_as_the_original() {
    let mut original = StableVector::new();
    let handles: Vec<Handle> = (0..6).map(|value| original.insert(value)).collect();
    // Free slots in an order that is not ascending: the free list now holds 4 before 2.
    original.remove(handles[1]);
    original.remove(handles[3]);

    let bytes = bincode::serialize(&original).unwrap();
    let mut restored: StableVector<i32> = bincode::deserialize(&bytes).unwrap();
    assert_eq!(hash_of(&restored), hash_of(&original));

    let from_original: Vec<Handle> = (10..14).map(|value| original.insert(value)).collect();
    let from_restored: Vec<Handle> = (10..14).map(|value| restored.insert(value)).collect();
    assert_eq!(from_restored, from_original);
    assert_eq!(hash_of(&restored), hash_of(&original));
}

#[test]
fn restoring_at_a_random_point_of_a_random_history_never_changes_what_follows() {
    use lockstep_core::Streams;

    for seed in 0..50u64 {
        let mut randomness = Streams::new(seed);
        let mut original: StableVector<u32> = StableVector::new();
        let mut live: Vec<Handle> = Vec::new();
        let restore_at = randomness.range("when", 50, 250);
        let mut restored: Option<StableVector<u32>> = None;
        let mut restored_live: Vec<Handle> = Vec::new();

        for operation in 0..400i32 {
            if operation == restore_at {
                let bytes = bincode::serialize(&original).unwrap();
                restored = Some(bincode::deserialize(&bytes).unwrap());
                restored_live = live.clone();
            }
            let insert = live.is_empty() || randomness.chance("churn", 0.55);
            let victim = if insert {
                0
            } else {
                randomness.pick("churn", live.len())
            };
            if insert {
                live.push(original.insert(operation as u32));
                if let Some(copy) = restored.as_mut() {
                    restored_live.push(copy.insert(operation as u32));
                }
            } else {
                original.remove(live.swap_remove(victim));
                if let Some(copy) = restored.as_mut() {
                    copy.remove(restored_live.swap_remove(victim));
                }
            }
            if let Some(copy) = restored.as_ref() {
                assert_eq!(restored_live, live, "seed {seed}, operation {operation}");
                assert_eq!(
                    hash_of(copy),
                    hash_of(&original),
                    "seed {seed}, operation {operation}"
                );
            }
        }
    }
}
