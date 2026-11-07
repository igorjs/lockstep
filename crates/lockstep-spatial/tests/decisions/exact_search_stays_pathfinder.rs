// SPDX-License-Identifier: Apache-2.0
//! Decision 0082: `Pathfinder` stays the default exact search, and the only one for ground with
//! terrain cost. A contraction hierarchy, the exact abstraction measured in the spike, took 19 to
//! 22 seconds to build and 163 to 214 milliseconds for 500 paths on one thread, against limits of 5
//! seconds and 50 milliseconds, so none is shipped.
//! Alternative rejected: a precomputed exact abstraction, which returns the same cost.
//! Would change if: a build under 5 seconds and 500 paths under 50 milliseconds on one thread are
//! measured on the benchmark map (decision 0082 has the numbers). Timing is never asserted here.

use lockstep_core::Streams;
use lockstep_spatial::{
    Cell, GridMap, JumpPathfinder, Occupancy, PathOptions, PathResult, Pathfinder, Square8,
};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SEED: u64 = 82;
const MAPS: u32 = 40;
const QUERIES_PER_MAP: u32 = 20;
const SIDE: u32 = 16;

/// The cheapest cost by plain Dijkstra over the map's own step rules, the yardstick.
fn reference(map: &GridMap<Square8>, from: Cell, to: Cell) -> Option<u32> {
    let mut best = vec![u32::MAX; map.cell_count()];
    let mut open = BinaryHeap::new();
    best[from.0 as usize] = 0;
    open.push(Reverse((0u32, from)));
    let mut next = Vec::new();
    while let Some(Reverse((cost, cell))) = open.pop() {
        if cell == to {
            return Some(cost);
        }
        if cost > best[cell.0 as usize] {
            continue;
        }
        map.neighbours(cell, &mut next);
        for neighbour in next.iter().copied() {
            if !map.can_step(cell, neighbour) {
                continue;
            }
            let through = cost + map.step_cost(cell, neighbour);
            if through < best[neighbour.0 as usize] {
                best[neighbour.0 as usize] = through;
                open.push(Reverse((through, neighbour)));
            }
        }
    }
    None
}

#[test]
fn pathfinder_is_exact_on_muddy_ground_where_the_jump_search_declines() {
    let mut streams = Streams::new(SEED);
    let mut compared = 0;
    for _ in 0..MAPS {
        let mut map: GridMap<Square8> = GridMap::new(SIDE, SIDE);
        for index in 0..map.cell_count() as u32 {
            if streams.range("wall", 0, 100) < 15 {
                map.set_passable(Cell(index), false);
            } else {
                map.set_cost(Cell(index), streams.range("mud", 1, 5) as u8);
            }
        }
        let occupancy = Occupancy::new(&map);
        let mut pathfinder = Pathfinder::new(&map);
        let mut path = Vec::new();
        let mut muddy = false;
        for index in 0..map.cell_count() as u32 {
            muddy |= map.is_passable(Cell(index)) && map.cost(Cell(index)) > 1;
        }
        assert!(muddy, "the generated map has terrain cost");
        assert!(
            JumpPathfinder::new(&map).is_none(),
            "the jump search must decline ground with terrain cost"
        );
        for _ in 0..QUERIES_PER_MAP {
            let from = Cell(streams.range("from", 0, map.cell_count() as i32) as u32);
            let to = Cell(streams.range("to", 0, map.cell_count() as i32) as u32);
            if !map.is_passable(from) || !map.is_passable(to) {
                continue;
            }
            let found = pathfinder.find(
                &map,
                &occupancy,
                from,
                to,
                PathOptions::default(),
                &mut path,
            );
            match (found, reference(&map, from, to)) {
                (PathResult::Found { cost }, Some(expected)) => assert_eq!(cost, expected),
                (PathResult::Unreachable, None) => {}
                (found, expected) => panic!("{found:?} against {expected:?}"),
            }
            compared += 1;
        }
    }
    assert!(compared > 300, "too few queries compared: {compared}");
}
