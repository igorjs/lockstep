//! Decision: a multi-cell body moves atomically. Either every target cell is free (or already its
//! own) and the whole body moves, or nothing changes at all.
//! Alternative rejected: moving cell by cell and undoing on failure, which can leave a body
//! half-moved if the undo itself is interrupted or reordered.
//! Would change if: a blocked move changes any cell, anchor, or other body (the number to beat is
//! zero changes).

use lockstep_core::{Handle, StableVector};
use lockstep_spatial::{GridMap, Occupancy, Occupied, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_two_by_two_move_blocked_on_one_cell_changes_nothing() {
    let map: GridMap<Square8> = GridMap::new(12, 12);
    let mut occupancy = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let (wretch, guard): (Handle, Handle) = (entities.insert(()), entities.insert(()));

    let start = [
        map.index(2, 2),
        map.index(3, 2),
        map.index(2, 3),
        map.index(3, 3),
    ];
    occupancy.move_footprint(wretch, &start).unwrap();
    occupancy.place(map.index(6, 5), guard).unwrap();

    // The target block touches the guard on its last cell only.
    let target = [
        map.index(5, 4),
        map.index(6, 4),
        map.index(5, 5),
        map.index(6, 5),
    ];
    assert_eq!(
        occupancy.move_footprint(wretch, &target),
        Err(Occupied(guard))
    );

    assert_eq!(
        occupancy.cells_of(wretch),
        Some(start.as_slice()),
        "the body did not move"
    );
    for cell in target {
        let expected = if cell == map.index(6, 5) {
            Some(guard)
        } else {
            None
        };
        assert_eq!(occupancy.at(cell), expected, "{cell:?} is unchanged");
    }
    for cell in start {
        assert_eq!(occupancy.at(cell), Some(wretch));
    }
    assert_eq!(occupancy.cell_of(guard), Some(map.index(6, 5)));
    assert_eq!(occupancy.len(), 2);

    // Once the way is clear, the same move succeeds in one piece.
    occupancy.vacate(guard);
    occupancy.move_footprint(wretch, &target).unwrap();
    assert_eq!(occupancy.cells_of(wretch), Some(target.as_slice()));
    for cell in start {
        assert_eq!(occupancy.at(cell), None);
    }
}
