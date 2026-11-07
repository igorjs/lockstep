// SPDX-License-Identifier: Apache-2.0
//! Decision 0081: the jump search stops as soon as the goal's cost is not above the estimate of
//! the next open jump point, which keeps the result and the cost of `Pathfinder` on every query.
//! Timing is measured only in the benchmark, never here.
//! Alternatives rejected: a bucket open set (measured slower, 366 against 326 milliseconds with a
//! heap per bucket and 550 with a scan, the estimates are too flat), a check on every push (it
//! costs a distance per push and saved no further expansion), and an approximate stop.
//! Would change if: any query below returns another result or cost than the plain search, or the
//! expansions total rises above the committed number.

use lockstep_core::Streams;
use lockstep_spatial::{
    Cell, GridMap, JumpPathfinder, Occupancy, PathOptions, Pathfinder, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SEED: u64 = 81;
const MAPS: u32 = 120;
const QUERIES_PER_MAP: u32 = 25;

/// Total jump expansions over the maps above; without the early stop it is 173,847.
const JUMP_EXPANSIONS: u64 = 173_667;

fn walled(streams: &mut Streams, width: u32, height: u32, density: i32) -> GridMap<Square8> {
    let mut map: GridMap<Square8> = GridMap::new(width, height);
    for index in 0..map.cell_count() as u32 {
        if streams.range("wall", 0, 100) < density {
            map.set_passable(Cell(index), false);
        }
    }
    map
}

/// Runs every query on both searches, asserting the same result, and returns the total jump and
/// plain expansions.
fn run() -> (u64, u64) {
    let mut streams = Streams::new(SEED);
    let (mut jump_total, mut plain_total) = (0u64, 0u64);
    for round in 0..MAPS {
        let width = streams.range("width", 2, if round % 3 == 0 { 140 } else { 48 }) as u32;
        let height = streams.range("height", 2, 48) as u32;
        let density = streams.range("density", 0, 40);
        let map = walled(&mut streams, width, height, density);
        let occupancy = Occupancy::new(&map);
        let mut plain = Pathfinder::new(&map);
        let mut jump = JumpPathfinder::new(&map).expect("only walls: uniform");
        for _ in 0..QUERIES_PER_MAP {
            let from = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
            let to = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
            let options = PathOptions::default();
            let wanted = plain.find(&map, &occupancy, from, to, options, &mut Vec::new());
            let got = jump.find(&map, &occupancy, from, to, options, &mut Vec::new());
            assert_eq!(got, wanted, "round {round}, {from:?} to {to:?}");
            plain_total += plain.last_expansions() as u64;
            jump_total += jump.last_expansions() as u64;
        }
    }
    (jump_total, plain_total)
}

#[test]
fn the_cost_equals_the_plain_search_with_the_early_stop() {
    run();
}

#[test]
fn the_expansions_are_deterministic_and_stay_under_the_committed_number() {
    let (jump, plain) = run();
    assert_eq!(run(), (jump, plain), "the same seed must expand the same");
    assert!(jump < plain, "{jump} against {plain}");
    assert!(jump <= JUMP_EXPANSIONS, "{jump} above {JUMP_EXPANSIONS}");
}
