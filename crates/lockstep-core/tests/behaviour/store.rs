// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{hash_of, Column, Handle, StableVector, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_stale_handle_is_detected_after_remove_and_reinsert() {
    let mut entities = StableVector::new();
    let first = entities.insert("first");
    assert_eq!(entities.remove(first), Some("first"));
    let second = entities.insert("second");

    assert_ne!(first, second, "the generation differs");
    assert_eq!(
        first.slot_index(),
        second.slot_index(),
        "the slot was reused"
    );
    assert!(!entities.contains(first));
    assert_eq!(entities.get(first), None);
    assert_eq!(entities.get(second), Some(&"second"));
}

#[test]
fn a_stale_handle_finds_nothing_in_a_column() {
    let mut entities = StableVector::new();
    let mut names: Column<&str> = Column::new();
    let first = entities.insert(());
    names.set(first, "first");
    entities.remove(first);
    names.unset(first);
    let second = entities.insert(());
    names.set(second, "second");
    assert!(!names.has(first));
    assert_eq!(names.get(first), None);
    assert_eq!(names.get(second), Some(&"second"));
}

fn churn(operations: u32, seed: u64) -> StableVector<u32> {
    let mut randomness = Streams::new(seed);
    let mut entities = StableVector::new();
    let mut live: Vec<Handle> = Vec::new();
    for value in 0..operations {
        if live.is_empty() || randomness.chance("churn", 0.6) {
            live.push(entities.insert(value));
        } else {
            let index = randomness.pick("churn", live.len());
            entities.remove(live.swap_remove(index));
        }
    }
    entities
}

#[test]
fn iteration_follows_slot_order_whatever_the_history() {
    for seed in 0..20 {
        let entities = churn(300, seed);
        let slots: Vec<u32> = entities
            .iter()
            .map(|(handle, _)| handle.slot_index())
            .collect();
        assert!(
            slots.windows(2).all(|pair| pair[0] < pair[1]),
            "seed {seed}: slots out of order"
        );
        let handle_slots: Vec<u32> = entities
            .handles()
            .iter()
            .map(|handle| handle.slot_index())
            .collect();
        assert_eq!(slots, handle_slots);
    }
}

#[test]
fn two_histories_that_fill_the_same_slots_iterate_in_the_same_order() {
    let mut straight = StableVector::new();
    for value in 0..5 {
        straight.insert(value);
    }

    let mut churned = StableVector::new();
    let handles: Vec<Handle> = (0..5).map(|value| churned.insert(value + 100)).collect();
    for handle in handles.iter().rev() {
        churned.remove(*handle);
    }
    for value in 0..5 {
        churned.insert(value);
    }

    let straight_slots: Vec<u32> = straight
        .iter()
        .map(|(handle, _)| handle.slot_index())
        .collect();
    let churned_slots: Vec<u32> = churned
        .iter()
        .map(|(handle, _)| handle.slot_index())
        .collect();
    assert_eq!(straight_slots, vec![0, 1, 2, 3, 4]);
    assert_eq!(churned_slots, straight_slots);
}

#[test]
fn a_snapshot_round_trip_preserves_every_handle_and_hashes_identically() {
    let mut entities = churn(200, 5);
    let mut positions: Column<(i32, i32)> = Column::new();
    let mut marks: Column<()> = Column::new();
    for (handle, value) in entities.iter() {
        positions.set(handle, (*value as i32, -(*value as i32)));
        if value % 3 == 0 {
            marks.set(handle, ());
        }
    }
    let before_handles = entities.handles();

    let bytes = bincode::serialize(&(&entities, &positions, &marks)).unwrap();
    let (restored_entities, restored_positions, restored_marks): (
        StableVector<u32>,
        Column<(i32, i32)>,
        Column<()>,
    ) = bincode::deserialize(&bytes).unwrap();

    assert_eq!(restored_entities.handles(), before_handles);
    assert_eq!(restored_entities, entities);
    assert_eq!(restored_positions, positions);
    assert_eq!(restored_marks, marks);
    assert_eq!(
        hash_of(&(&restored_entities, &restored_positions, &restored_marks)),
        hash_of(&(&entities, &positions, &marks))
    );
    for handle in before_handles {
        assert!(restored_entities.contains(handle));
        assert_eq!(restored_positions.get(handle), positions.get(handle));
    }
    entities.insert(0);
}

#[test]
fn a_marker_column_records_membership_only() {
    let mut entities = StableVector::new();
    let mut frightened: Column<()> = Column::new();
    let calm = entities.insert("calm");
    let afraid = entities.insert("afraid");
    frightened.set(afraid, ());
    assert!(!frightened.has(calm));
    assert!(frightened.has(afraid));
    assert_eq!(frightened.handles(), vec![afraid]);
}

#[test]
fn handles_round_trip_through_their_raw_form() {
    let mut entities = StableVector::new();
    let handle = entities.insert(1);
    assert_eq!(Handle::from_raw(handle.raw()), handle);
}

#[test]
fn the_all_zero_handle_never_refers_to_anything() {
    let mut entities = StableVector::new();
    let mut column: Column<u8> = Column::new();
    let null = Handle::from_raw(0);
    assert!(!entities.contains(null));
    let first = entities.insert(1);
    column.set(first, 1);
    assert_ne!(first, null);
    assert!(!entities.contains(null));
    assert!(!column.has(null));
    assert_eq!(entities.remove(null), None);
}

#[test]
fn columns_with_the_same_entries_hash_the_same_whatever_their_history() {
    let mut entities = StableVector::new();
    let handles: Vec<Handle> = (0..10).map(|value| entities.insert(value)).collect();

    let mut straight: Column<u32> = Column::new();
    straight.set(handles[2], 20);

    let mut churned: Column<u32> = Column::new();
    for handle in &handles {
        churned.set(*handle, 7);
    }
    for (index, handle) in handles.iter().enumerate() {
        if index != 2 {
            churned.unset(*handle);
        }
    }
    churned.set(handles[2], 20);

    assert_eq!(straight, churned);
    assert_eq!(hash_of(&straight), hash_of(&churned));
}

#[test]
fn an_unset_with_a_stale_handle_leaves_the_newer_entry_alone() {
    let mut entities = StableVector::new();
    let mut column: Column<&str> = Column::new();
    let old = entities.insert(());
    column.set(old, "old");
    entities.remove(old);
    column.unset(old);
    let new = entities.insert(());
    column.set(new, "new");
    assert_eq!(column.unset(old), None);
    assert_eq!(column.get(new), Some(&"new"));
}

#[test]
fn a_save_with_a_zero_generation_is_refused() {
    let mut entities = StableVector::new();
    entities.insert(5u8);
    let mut bytes = bincode::serialize(&entities).unwrap();
    // Layout: length (8 bytes), then generation (4 bytes) of the first slot.
    bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
    assert!(bincode::deserialize::<StableVector<u8>>(&bytes).is_err());
}
