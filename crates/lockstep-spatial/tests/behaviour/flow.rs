// SPDX-License-Identifier: Apache-2.0
use crate::common::{free_cell, random_map};
use lockstep_core::Streams;
use lockstep_spatial::{Cell, FlowField, GridMap, Square4, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_target_has_distance_zero_and_no_next_step() {
    let map: GridMap<Square8> = GridMap::new(8, 8);
    let target = map.index(4, 4);
    let field = FlowField::build(&map, &[target], u32::MAX);
    assert_eq!(field.distance(target), Some(0));
    assert_eq!(field.step_from(target), None);
}

#[test]
fn distances_follow_octile_cost_on_open_ground() {
    let map: GridMap<Square8> = GridMap::new(12, 12);
    let field = FlowField::build(&map, &[map.index(0, 0)], u32::MAX);
    assert_eq!(field.distance(map.index(5, 0)), Some(50));
    assert_eq!(field.distance(map.index(3, 3)), Some(42));
    assert_eq!(field.distance(map.index(5, 2)), Some(58));
}

#[test]
fn the_nearest_of_several_targets_wins() {
    let map: GridMap<Square4> = GridMap::new(11, 1);
    let field = FlowField::build(&map, &[map.index(0, 0), map.index(10, 0)], u32::MAX);
    assert_eq!(field.distance(map.index(3, 0)), Some(30));
    assert_eq!(field.distance(map.index(7, 0)), Some(30));
    assert_eq!(field.step_from(map.index(3, 0)), Some(map.index(2, 0)));
    assert_eq!(field.step_from(map.index(7, 0)), Some(map.index(8, 0)));
}

#[test]
fn the_field_stops_at_the_maximum_distance() {
    let map: GridMap<Square4> = GridMap::new(20, 1);
    let field = FlowField::build(&map, &[map.index(0, 0)], 50);
    assert_eq!(field.distance(map.index(5, 0)), Some(50));
    assert_eq!(field.distance(map.index(6, 0)), None);
    assert_eq!(field.step_from(map.index(6, 0)), None);
}

#[test]
fn walls_are_never_reached_and_regions_behind_them_stay_unreached() {
    let mut map: GridMap<Square4> = GridMap::new(7, 3);
    for y in 0..3 {
        map.set_passable(map.index(3, y), false);
    }
    let field = FlowField::build(&map, &[map.index(0, 1)], u32::MAX);
    assert_eq!(field.distance(map.index(3, 1)), None, "a wall cell");
    assert_eq!(field.distance(map.index(5, 1)), None, "behind the wall");
    assert_eq!(field.distance(map.index(2, 1)), Some(20));
}

#[test]
fn every_cells_next_step_is_strictly_closer_and_legal_and_leads_to_a_target() {
    for seed in 0..40 {
        let map: GridMap<Square8> = random_map(seed, 28, 28, 25);
        let mut streams = Streams::new(seed);
        let targets = [
            free_cell(&map, &mut streams, "t"),
            free_cell(&map, &mut streams, "t"),
        ];
        let field = FlowField::build(&map, &targets, u32::MAX);
        for index in 0..map.cell_count() as u32 {
            let cell = Cell(index);
            let Some(distance) = field.distance(cell) else {
                continue;
            };
            let mut at = cell;
            let mut remaining = distance;
            while let Some(next) = field.step_from(at) {
                assert!(
                    map.can_step(at, next),
                    "seed {seed}: {at:?} to {next:?} is illegal"
                );
                let next_distance = field.distance(next).expect("a next step is reached");
                assert!(
                    next_distance < remaining,
                    "seed {seed}: {next_distance} is not closer than {remaining}"
                );
                assert_eq!(
                    remaining,
                    next_distance + map.step_cost(at, next),
                    "the step accounts for the distance"
                );
                (at, remaining) = (next, next_distance);
            }
            assert!(targets.contains(&at), "seed {seed}: ended on {at:?}");
            assert_eq!(remaining, 0);
        }
    }
}

#[test]
fn the_same_inputs_build_the_same_field() {
    let map: GridMap<Square8> = random_map(5, 30, 30, 20);
    let targets = [map.index(1, 1)];
    let (first, second) = (
        FlowField::build(&map, &targets, u32::MAX),
        FlowField::build(&map, &targets, u32::MAX),
    );
    for index in 0..map.cell_count() as u32 {
        assert_eq!(first.distance(Cell(index)), second.distance(Cell(index)));
        assert_eq!(first.step_from(Cell(index)), second.step_from(Cell(index)));
    }
}
