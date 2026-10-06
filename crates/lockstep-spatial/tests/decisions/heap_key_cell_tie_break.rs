// SPDX-License-Identifier: Apache-2.0
//! Decision: the open set is ordered by `(f_cost, cell)`: the cell index breaks ties between equal
//! costs, and neighbours are visited in a fixed order, so equal-cost paths are identical on every
//! platform.
//! Alternative rejected: an unordered tie, which lets the heap's internal layout pick the path.
//! Would change if: the hash of every path over 1,000 seeded maps differs from the committed value
//! natively or under WebAssembly (the number to beat is zero differing bits).

use crate::common::{free_cell, random_map};
use lockstep_core::{hash_of, Streams};
use lockstep_spatial::{GridMap, Occupancy, PathOptions, Pathfinder, Square4, Square8, Topology};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// The result and the path for one seeded map, as plain numbers.
fn search<T: Topology>(seed: u64) -> (String, Vec<u32>) {
    let map: GridMap<T> = random_map(seed, 24, 24, 22);
    let mut streams = Streams::new(seed ^ 0xabc);
    let (from, to) = (
        free_cell(&map, &mut streams, "from"),
        free_cell(&map, &mut streams, "to"),
    );
    let occupancy = Occupancy::new(&map);
    let mut path = Vec::new();
    let result = Pathfinder::new(&map).find(
        &map,
        &occupancy,
        from,
        to,
        PathOptions::default(),
        &mut path,
    );
    (
        format!("{result:?}"),
        path.iter().map(|cell| cell.0).collect(),
    )
}

fn paths_hash() -> u64 {
    let results: Vec<(u64, String, Vec<u32>)> = (0..1_000u64)
        .map(|seed| {
            let (result, path) = if seed % 2 == 0 {
                search::<Square8>(seed)
            } else {
                search::<Square4>(seed)
            };
            (seed, result, path)
        })
        .collect();
    hash_of(&results)
}

#[test]
fn the_paths_over_one_thousand_seeded_maps_hash_to_the_committed_value() {
    let committed = include_str!("../../fixtures/paths.hash").trim();
    assert_eq!(format!("{:016x}", paths_hash()), committed);
}

/// Writes the committed value. Run with
/// `cargo test -p lockstep-spatial regenerate_paths_hash -- --ignored`, and commit the file with a
/// message that names the rule that changed.
#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore]
fn regenerate_paths_hash() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/paths.hash");
    std::fs::write(path, format!("{:016x}\n", paths_hash())).unwrap();
}
