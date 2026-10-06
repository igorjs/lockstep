use lockstep_core::{Handle, StableVector, Streams};
use lockstep_spatial::{Cell, GridMap, Occupancy, Occupied, Square8};
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn setup() -> (GridMap<Square8>, Occupancy, Vec<Handle>) {
    let map: GridMap<Square8> = GridMap::new(20, 20);
    let occupancy = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let handles = (0..8).map(|index| entities.insert(index)).collect();
    (map, occupancy, handles)
}

#[test]
fn placing_and_looking_up_a_body() {
    let (map, mut occupancy, who) = setup();
    let cell = map.index(3, 4);
    assert_eq!(occupancy.at(cell), None);
    assert_eq!(occupancy.place(cell, who[0]), Ok(()));
    assert_eq!(occupancy.at(cell), Some(who[0]));
    assert_eq!(occupancy.cell_of(who[0]), Some(cell));
    assert_eq!(occupancy.len(), 1);
    assert!(!occupancy.is_empty());
}

#[test]
fn a_taken_cell_is_refused_and_reports_the_holder() {
    let (map, mut occupancy, who) = setup();
    let cell = map.index(3, 4);
    occupancy.place(cell, who[0]).unwrap();
    assert_eq!(occupancy.place(cell, who[1]), Err(Occupied(who[0])));
    assert_eq!(occupancy.at(cell), Some(who[0]));
    assert_eq!(
        occupancy.cell_of(who[1]),
        None,
        "the refused body was not placed"
    );
}

#[test]
fn placing_a_body_that_is_already_there_is_harmless_and_placing_it_elsewhere_moves_it() {
    let (map, mut occupancy, who) = setup();
    let (a, b) = (map.index(1, 1), map.index(2, 2));
    occupancy.place(a, who[0]).unwrap();
    occupancy.place(a, who[0]).unwrap();
    assert_eq!(occupancy.len(), 1);
    occupancy.place(b, who[0]).unwrap();
    assert_eq!((occupancy.at(a), occupancy.at(b)), (None, Some(who[0])));
}

#[test]
fn vacating_frees_the_cell_and_returns_it() {
    let (map, mut occupancy, who) = setup();
    let cell = map.index(5, 5);
    occupancy.place(cell, who[0]).unwrap();
    assert_eq!(occupancy.vacate(who[0]), Some(cell));
    assert_eq!(occupancy.at(cell), None);
    assert_eq!(occupancy.vacate(who[0]), None);
    assert!(occupancy.is_empty());
}

#[test]
fn moving_returns_the_cell_left_and_refuses_a_taken_target() {
    let (map, mut occupancy, who) = setup();
    let (a, b, c) = (map.index(1, 1), map.index(2, 1), map.index(3, 1));
    occupancy.place(a, who[0]).unwrap();
    occupancy.place(c, who[1]).unwrap();
    assert_eq!(occupancy.move_to(who[0], b), Ok(a));
    assert_eq!(occupancy.at(a), None);
    assert_eq!(occupancy.move_to(who[0], c), Err(Occupied(who[1])));
    assert_eq!(
        occupancy.cell_of(who[0]),
        Some(b),
        "a refused move changes nothing"
    );
    assert_eq!(
        occupancy.move_to(who[2], map.index(9, 9)),
        Ok(map.index(9, 9)),
        "an unplaced body is placed"
    );
}

#[test]
fn vacating_from_the_middle_keeps_every_other_body_findable() {
    let (map, mut occupancy, who) = setup();
    for (index, handle) in who.iter().enumerate() {
        occupancy
            .place(map.index(index as u32, 0), *handle)
            .unwrap();
    }
    occupancy.vacate(who[2]);
    occupancy.vacate(who[0]);
    for (index, handle) in who.iter().enumerate() {
        let expected = if index == 0 || index == 2 {
            None
        } else {
            Some(map.index(index as u32, 0))
        };
        assert_eq!(occupancy.cell_of(*handle), expected, "body {index}");
        if let Some(cell) = expected {
            assert_eq!(occupancy.at(cell), Some(*handle));
        }
    }
}

#[test]
fn a_footprint_holds_several_cells_with_the_first_as_its_anchor() {
    let (map, mut occupancy, who) = setup();
    let block = [
        map.index(4, 4),
        map.index(5, 4),
        map.index(4, 5),
        map.index(5, 5),
    ];
    occupancy.move_footprint(who[0], &block).unwrap();
    for cell in block {
        assert_eq!(occupancy.at(cell), Some(who[0]));
    }
    assert_eq!(occupancy.cell_of(who[0]), Some(block[0]));
    assert_eq!(occupancy.cells_of(who[0]), Some(block.as_slice()));
    assert_eq!(occupancy.vacate(who[0]), Some(block[0]));
    for cell in block {
        assert_eq!(occupancy.at(cell), None);
    }
}

#[test]
fn a_footprint_may_overlap_its_own_previous_cells() {
    let (map, mut occupancy, who) = setup();
    let first = [map.index(4, 4), map.index(5, 4)];
    let second = [map.index(5, 4), map.index(6, 4)];
    occupancy.move_footprint(who[0], &first).unwrap();
    occupancy.move_footprint(who[0], &second).unwrap();
    assert_eq!(
        occupancy.at(map.index(4, 4)),
        None,
        "the cell it left is free"
    );
    assert_eq!(occupancy.at(map.index(6, 4)), Some(who[0]));
}

#[test]
fn within_returns_occupied_cells_inside_the_radius_sorted_by_cell() {
    let (map, mut occupancy, who) = setup();
    let centre = map.index(10, 10);
    let placements = [(10, 10), (12, 10), (10, 13), (15, 15), (8, 8)];
    for (index, (x, y)) in placements.iter().enumerate().rev() {
        occupancy.place(map.index(*x, *y), who[index]).unwrap();
    }
    let mut found = Vec::new();
    occupancy.within(&map, centre, 30, &mut found);
    let cells: Vec<Cell> = found.iter().map(|(cell, _)| *cell).collect();
    assert_eq!(
        cells,
        vec![
            map.index(8, 8),
            map.index(10, 10),
            map.index(12, 10),
            map.index(10, 13)
        ]
    );
    assert_eq!(found[0].1, who[4]);
    occupancy.within(&map, centre, 0, &mut found);
    assert_eq!(
        found,
        vec![(centre, who[0])],
        "radius zero is the centre only"
    );
}

#[test]
fn within_clips_at_the_map_edge() {
    let (map, mut occupancy, who) = setup();
    occupancy.place(map.index(0, 0), who[0]).unwrap();
    occupancy.place(map.index(19, 19), who[1]).unwrap();
    let mut found = Vec::new();
    occupancy.within(&map, map.index(0, 0), 100, &mut found);
    assert_eq!(found, vec![(map.index(0, 0), who[0])]);
    occupancy.within(&map, map.index(19, 19), 1_000, &mut found);
    assert_eq!(found.len(), 2);
}

/// A plain model to compare against: a map from handle to its cells.
fn fuzz(seed: u64) {
    let mut streams = Streams::new(seed);
    let (map, mut occupancy, who) = setup();
    let mut model: BTreeMap<Handle, Vec<Cell>> = BTreeMap::new();
    for _ in 0..400 {
        let handle = who[streams.pick("who", who.len())];
        let anchor = (
            streams.range("x", 0, 19) as u32,
            streams.range("y", 0, 19) as u32,
        );
        let cells = if streams.chance("shape", 0.4) {
            vec![
                map.index(anchor.0, anchor.1),
                map.index(anchor.0 + 1, anchor.1),
                map.index(anchor.0, anchor.1 + 1),
            ]
        } else {
            vec![map.index(anchor.0, anchor.1)]
        };
        match streams.range("op", 0, 3) {
            0 | 1 => {
                let conflict = cells.iter().find_map(|cell| {
                    model
                        .iter()
                        .find(|(other, held)| **other != handle && held.contains(cell))
                        .map(|(other, _)| *other)
                });
                let result = occupancy.move_footprint(handle, &cells);
                match conflict {
                    Some(holder) => assert_eq!(result, Err(Occupied(holder))),
                    None => {
                        assert_eq!(result, Ok(()));
                        model.insert(handle, cells);
                    }
                }
            }
            _ => {
                let expected = model.remove(&handle).map(|held| held[0]);
                assert_eq!(occupancy.vacate(handle), expected);
            }
        }
        for index in 0..map.cell_count() as u32 {
            let expected = model
                .iter()
                .find(|(_, held)| held.contains(&Cell(index)))
                .map(|(handle, _)| *handle);
            assert_eq!(
                occupancy.at(Cell(index)),
                expected,
                "seed {seed}, cell {index}"
            );
        }
        for (handle, held) in &model {
            assert_eq!(occupancy.cells_of(*handle), Some(held.as_slice()));
        }
        assert_eq!(occupancy.len(), model.len());
    }
}

#[test]
fn a_random_history_always_matches_a_plain_model() {
    for seed in 0..12 {
        fuzz(seed);
    }
}
