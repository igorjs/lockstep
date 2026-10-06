//! Decision: entities are generational handles into a stable vector, and components are columns.
//! Alternatives rejected: archetype storage, and one row of fields per entity.
//! Would change if: a stale handle ever resolves to a newer entity (the number to beat is zero
//! in ten thousand reuses of one slot), or a column visits an entity that lacks the component.

use lockstep_core::{Column, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn ten_thousand_reuses_of_one_slot_never_resolve_a_stale_handle() {
    let mut entities = StableVector::new();
    let mut previous = Vec::new();
    for round in 0..10_000u32 {
        let handle = entities.insert(round);
        for stale in previous.iter().rev().take(8) {
            assert!(!entities.contains(*stale));
        }
        entities.remove(handle);
        previous.push(handle);
    }
}

#[test]
fn a_sparse_column_visits_only_the_entities_that_have_the_component() {
    let mut entities = StableVector::new();
    let mut burning: Column<u8> = Column::new();
    let handles: Vec<_> = (0..1000).map(|value| entities.insert(value)).collect();
    burning.set(handles[500], 3);
    let visited: Vec<_> = burning.iter().collect();
    assert_eq!(visited.len(), 1);
    assert_eq!(visited[0].0, handles[500]);
}
