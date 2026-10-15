// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each states the decision, the alternative that was rejected, and
//! the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

use lockstep_core::StableVector;
use lockstep_spatial::{Cell, GridMap, Square4};
use lockstep_structures::{Refusal, Structures};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Decision: a structure is built all or nothing. Every cell is checked first; when one is
/// blocked, the map is left exactly as it was.
/// Alternative rejected: walling cells one by one and stopping at the first blocked one, which
/// leaves half a structure standing with no owner.
/// Would change if: a refused build changes any cell or marks any chunk.
#[test]
fn a_refused_build_leaves_the_map_alone() {
    let mut map: GridMap<Square4> = GridMap::new(8, 8);
    let mut owners = StableVector::new();
    let wall = owners.insert(());
    map.set_passable(map.index(3, 4), false);
    map.take_dirty_chunks();
    let before = map.clone();
    let cells = [map.index(3, 2), map.index(3, 3), map.index(3, 4)];
    let mut structures = Structures::new();
    let mut events = Vec::new();
    assert_eq!(
        structures.build(&mut map, wall, &cells, 5, &mut events),
        Err(Refusal::Blocked(cells[2]))
    );
    assert_eq!(map, before);
    assert!(map.take_dirty_chunks().is_empty());
    assert!(structures.is_empty() && events.is_empty());
}

/// Decision: raising and bringing down a structure goes through the map's own setters, so the
/// chunks holding its cells are marked dirty at both moments. The structures keep no dirty
/// bookkeeping of their own.
/// Alternative rejected: a separate list of changed chunks, which a host would have to merge with
/// the map's and which could disagree with it.
/// Would change if: a build or a collapse across a chunk border fails to report both chunks.
#[test]
fn passability_changes_mark_the_chunks_dirty() {
    let mut map: GridMap<Square4> = GridMap::new(64, 64);
    let mut owners = StableVector::new();
    let wall = owners.insert(());
    let cells: Vec<Cell> = (30..34).map(|x| map.index(x, 5)).collect();
    map.take_dirty_chunks();
    let mut structures = Structures::new();
    let mut events = Vec::new();
    structures
        .build(&mut map, wall, &cells, 3, &mut events)
        .unwrap();
    assert_eq!(map.take_dirty_chunks(), vec![0, 1]);
    structures.damage(&mut map, wall, 2, &mut events).unwrap();
    assert!(map.take_dirty_chunks().is_empty());
    structures.damage(&mut map, wall, 1, &mut events).unwrap();
    assert!(cells.iter().all(|cell| map.is_passable(*cell)));
    assert_eq!(map.take_dirty_chunks(), vec![0, 1]);
}
