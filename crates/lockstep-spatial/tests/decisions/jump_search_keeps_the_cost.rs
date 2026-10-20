// SPDX-License-Identifier: Apache-2.0
//! Decision: on uniform open ground the jump search (`JumpPathfinder`) is an opt-in exact search.
//! It returns the same result and the same cost as `Pathfinder` on every query, and a walkable
//! path, but among equal-cost paths it may choose other cells, so a fixture made with one search
//! is not valid for the other. `Pathfinder` stays the default.
//! Alternatives rejected: making it the default (it would change paths and fixture hashes), and
//! matching the cells of `Pathfinder` exactly (the jumps do not visit the cells that decide its
//! tie break). Decision 0075 has the numbers.
//! Would change if: any query over the maps below returns another result or cost than the plain
//! search, or the jump search expands more than the plain search on the benchmark shape.

use lockstep_core::Streams;
use lockstep_spatial::{
    Cell, GridMap, JumpPathfinder, Occupancy, PathOptions, PathResult, Pathfinder, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn random_walls(streams: &mut Streams, width: u32, height: u32, density: i32) -> GridMap<Square8> {
    let mut map: GridMap<Square8> = GridMap::new(width, height);
    for index in 0..map.cell_count() as u32 {
        if streams.range("wall", 0, 100) < density {
            map.set_passable(Cell(index), false);
        }
    }
    map
}

fn assert_walkable(map: &GridMap<Square8>, from: Cell, path: &[Cell], cost: u32) {
    let mut at = from;
    let mut total = 0;
    for next in path {
        assert!(
            map.can_step(at, *next),
            "{at:?} to {next:?} is not a legal step"
        );
        total += map.step_cost(at, *next);
        at = *next;
    }
    assert_eq!(total, cost, "the path does not cost what was reported");
}

#[test]
fn the_cost_equals_the_plain_search_on_many_random_maps() {
    let mut streams = Streams::new(75);
    let mut queries = 0;
    for round in 0..400 {
        // Some maps are wider than a 64 cell word, so runs cross word borders.
        let wide = round % 4 == 0;
        let width = streams.range("width", 2, if wide { 150 } else { 40 }) as u32;
        let height = streams.range("height", 2, 40) as u32;
        let density = streams.range("density", 0, 45);
        let map = random_walls(&mut streams, width, height, density);
        let occupancy = Occupancy::new(&map);
        let mut plain = Pathfinder::new(&map);
        let mut jump = JumpPathfinder::new(&map).expect("a map with only walls is uniform");
        for _ in 0..30 {
            let from = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
            let to = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
            let (mut expected, mut found) = (Vec::new(), Vec::new());
            let options = PathOptions::default();
            let wanted = plain.find(&map, &occupancy, from, to, options, &mut expected);
            let got = jump.find(&map, &occupancy, from, to, options, &mut found);
            assert_eq!(
                got, wanted,
                "round {round}, {width}x{height}, {from:?} to {to:?}"
            );
            if let PathResult::Found { cost } = got {
                assert_walkable(&map, from, &found, cost);
                assert_eq!(found.last().copied().unwrap_or(from), to);
            }
            queries += 1;
        }
    }
    assert_eq!(queries, 12_000);
}

#[test]
fn only_uniform_ground_is_accepted() {
    let mut map: GridMap<Square8> = GridMap::new(8, 8);
    assert!(JumpPathfinder::new(&map).is_some());
    map.set_cost(Cell(10), 3);
    assert!(JumpPathfinder::new(&map).is_none(), "mud is not uniform");
    map.set_cost(Cell(10), 1);
    map.set_elevation(Cell(11), 2);
    assert!(JumpPathfinder::new(&map).is_none(), "a hill is not uniform");
    map.set_passable(Cell(11), false);
    assert!(
        JumpPathfinder::new(&map).is_some(),
        "a wall has no ground to compare"
    );
}

#[test]
fn bodies_as_walls_and_a_wall_start_get_the_plain_answer() {
    let mut map: GridMap<Square8> = GridMap::new(12, 12);
    map.set_passable(Cell(30), false);
    let occupancy = Occupancy::new(&map);
    let mut plain = Pathfinder::new(&map);
    let mut jump = JumpPathfinder::new(&map).unwrap();
    let options = PathOptions {
        treat_occupants_as_walls: true,
        ..PathOptions::default()
    };
    for (from, to) in [
        (Cell(0), Cell(143)),
        (Cell(30), Cell(100)),
        (Cell(5), Cell(30)),
    ] {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        assert_eq!(
            jump.find(&map, &occupancy, from, to, options, &mut a),
            plain.find(&map, &occupancy, from, to, options, &mut b)
        );
        assert_eq!(a, b);
    }
}

#[test]
#[should_panic(expected = "the map changed")]
fn a_changed_map_needs_a_new_jump_pathfinder() {
    let mut map: GridMap<Square8> = GridMap::new(8, 8);
    let mut jump = JumpPathfinder::new(&map).unwrap();
    map.set_passable(Cell(9), false);
    let occupancy = Occupancy::new(&map);
    jump.find(
        &map,
        &occupancy,
        Cell(0),
        Cell(63),
        PathOptions::default(),
        &mut Vec::new(),
    );
}

#[test]
fn it_expands_fewer_cells_than_the_plain_search() {
    let mut streams = Streams::new(76);
    let map = random_walls(&mut streams, 200, 200, 20);
    let occupancy = Occupancy::new(&map);
    let mut plain = Pathfinder::new(&map);
    let mut jump = JumpPathfinder::new(&map).unwrap();
    let (mut plain_work, mut jump_work) = (0u64, 0u64);
    for _ in 0..40 {
        let mut pick = || loop {
            let cell = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
            if map.is_passable(cell) {
                return cell;
            }
        };
        let (from, to) = (pick(), pick());
        let options = PathOptions::default();
        plain.find(&map, &occupancy, from, to, options, &mut Vec::new());
        jump.find(&map, &occupancy, from, to, options, &mut Vec::new());
        plain_work += plain.last_expansions() as u64;
        jump_work += jump.last_expansions() as u64;
    }
    assert!(jump_work < plain_work, "{jump_work} against {plain_work}");
}
