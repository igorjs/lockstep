// SPDX-License-Identifier: Apache-2.0
//! Decision: a flow field is computed once outward from the targets and every body then moves by a
//! table lookup, with no search of its own.
//! Alternative rejected: one A* search per body per replan, which costs a hundred searches for a
//! hundred bodies chasing the same target.
//! Would change if: any cell's next step is not strictly closer to a target (the number to beat is
//! zero such cells over 40 random maps) or one field costs more than a thousand A* searches.

use crate::common::{free_cell, random_map};
use lockstep_core::Streams;
use lockstep_spatial::{Cell, FlowField, GridMap, Occupancy, PathOptions, Pathfinder, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn every_next_step_is_strictly_closer_to_a_target() {
    let mut checked = 0;
    for seed in 100..140 {
        let map: GridMap<Square8> = random_map(seed, 30, 30, 28);
        let mut streams = Streams::new(seed);
        let target = free_cell(&map, &mut streams, "target");
        let field = FlowField::build(&map, &[target], u32::MAX);
        for index in 0..map.cell_count() as u32 {
            if let (Some(distance), Some(next)) =
                (field.distance(Cell(index)), field.step_from(Cell(index)))
            {
                assert!(field.distance(next).unwrap() < distance);
                checked += 1;
            }
        }
    }
    assert!(checked > 10_000, "the test visited {checked} cells");
}

#[test]
fn one_field_does_less_work_than_a_crowd_of_searches_to_the_same_target() {
    // A field expands every reachable cell at most once. A hundred bodies each searching for the
    // same target expand far more, which is the trade this decision makes.
    let map: GridMap<Square8> = random_map(7, 64, 64, 20);
    let mut streams = Streams::new(7);
    let target = free_cell(&map, &mut streams, "target");
    let occupancy = Occupancy::new(&map);
    let mut pathfinder = Pathfinder::new(&map);
    let mut path = Vec::new();
    let mut expanded: u64 = 0;
    for _ in 0..100 {
        let from = free_cell(&map, &mut streams, "body");
        pathfinder.find(
            &map,
            &occupancy,
            from,
            target,
            PathOptions::default(),
            &mut path,
        );
        expanded += pathfinder.last_expansions() as u64;
    }
    assert!(
        expanded > map.cell_count() as u64,
        "a hundred searches expanded {expanded} cells, no more than one field's {}",
        map.cell_count()
    );
}
