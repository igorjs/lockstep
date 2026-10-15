// SPDX-License-Identifier: Apache-2.0
//! Behaviour tests for structures.

use lockstep_core::{decode, encode, StableVector};
use lockstep_spatial::{GridMap, Square4};
use lockstep_structures::{Refusal, StructureEvent, Structures};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn setup() -> (GridMap<Square4>, Structures, lockstep_core::Handle) {
    let mut owners = StableVector::new();
    (GridMap::new(8, 8), Structures::new(), owners.insert(()))
}

#[test]
fn a_structure_blocks_its_cells_until_it_collapses() {
    let (mut map, mut structures, id) = setup();
    let cells = [map.index(2, 2), map.index(2, 3)];
    let mut events = Vec::new();
    structures
        .build(&mut map, id, &cells, 4, &mut events)
        .unwrap();
    assert!(cells.iter().all(|cell| !map.is_passable(*cell)));
    assert_eq!(structures.at(cells[1]), Some(id));
    structures.damage(&mut map, id, 3, &mut events).unwrap();
    assert_eq!(structures.get(id).unwrap().durability(), 1);
    structures.damage(&mut map, id, 9, &mut events).unwrap();
    assert!(structures.get(id).is_none());
    assert!(cells.iter().all(|cell| map.is_passable(*cell)));
    assert_eq!(
        events,
        vec![
            StructureEvent::Built { structure: id },
            StructureEvent::Damaged {
                structure: id,
                durability: 1
            },
            StructureEvent::Collapsed { structure: id },
        ]
    );
}

#[test]
fn repair_stops_at_the_built_durability_and_dismantling_opens_the_cells() {
    let (mut map, mut structures, id) = setup();
    let cell = map.index(1, 1);
    let mut events = Vec::new();
    structures
        .build(&mut map, id, &[cell], 10, &mut events)
        .unwrap();
    structures.damage(&mut map, id, 6, &mut events).unwrap();
    structures.repair(id, 100, &mut events).unwrap();
    assert_eq!(structures.get(id).unwrap().durability(), 10);
    structures.dismantle(&mut map, id, &mut events).unwrap();
    assert!(map.is_passable(cell) && structures.is_empty());
    assert_eq!(
        structures.dismantle(&mut map, id, &mut events),
        Err(Refusal::Unknown)
    );
}

#[test]
fn builds_are_refused_with_a_reason() {
    let (mut map, mut structures, id) = setup();
    let mut events = Vec::new();
    let cell = map.index(0, 0);
    assert_eq!(
        structures.build(&mut map, id, &[], 1, &mut events),
        Err(Refusal::Empty)
    );
    assert_eq!(
        structures.build(&mut map, id, &[cell], 0, &mut events),
        Err(Refusal::Empty)
    );
    let off = lockstep_spatial::Cell(64);
    assert_eq!(
        structures.build(&mut map, id, &[off], 1, &mut events),
        Err(Refusal::OffMap(off))
    );
    assert_eq!(
        structures.build(&mut map, id, &[cell, cell], 1, &mut events),
        Err(Refusal::Blocked(cell))
    );
    structures
        .build(&mut map, id, &[cell], 1, &mut events)
        .unwrap();
    assert_eq!(
        structures.build(&mut map, id, &[lockstep_spatial::Cell(5)], 1, &mut events),
        Err(Refusal::AlreadyBuilt)
    );
}

#[test]
fn structures_survive_a_save() {
    let (mut map, mut structures, id) = setup();
    let mut events = Vec::new();
    structures
        .build(&mut map, id, &[lockstep_spatial::Cell(36)], 7, &mut events)
        .unwrap();
    let saved: Structures = decode(&encode(&structures)).unwrap();
    assert_eq!(saved, structures);
}
