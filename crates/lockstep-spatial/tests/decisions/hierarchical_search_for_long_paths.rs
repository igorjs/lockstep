// SPDX-License-Identifier: Apache-2.0
//! Decision: long paths on large maps use a hierarchy of chunk entrances (`Hierarchy` and
//! `HierarchicalPathfinder`), which gives a path dearer than the cheapest (6.4 percent on the benchmark map, 13.2 percent on the
//! hilly and muddy test maps below) in return for several times fewer expansions. `Pathfinder` stays exact and is still the default, so no existing
//! path or fixture hash changes.
//! Alternatives rejected, each measured on 500 paths over a 512 by 512 map with 20 percent walls:
//! packing the search state tighter and stepping without division (no measurable gain), breaking
//! heap ties by depth (1 percent fewer expansions), landmark estimates (about 1.4 times fewer
//! expansions, with a build cost of 80 to 280 milliseconds), and jump point search, which needs
//! uniform terrain cost and flat ground while this map has both varied. Decision 0047 has the
//! numbers.
//! Would change if: the hierarchical total cost is more than 15 percent above the exact total, or
//! it expands fewer than four times fewer cells and entrances than the exact search, over the maps
//! below (the numbers to beat), or a hash of its paths differs natively or under WebAssembly.

use crate::common::{free_cell, random_map};
use lockstep_core::{hash_of, Streams};
use lockstep_spatial::{
    Cell, GridMap, HierarchicalPathfinder, Hierarchy, Occupancy, PathOptions, PathResult,
    Pathfinder, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn pairs(map: &GridMap<Square8>, seed: u64, count: usize) -> Vec<(Cell, Cell)> {
    let mut streams = Streams::new(seed);
    (0..count)
        .map(|_| {
            (
                free_cell(map, &mut streams, "from"),
                free_cell(map, &mut streams, "to"),
            )
        })
        .collect()
}

#[test]
fn the_paths_over_seeded_maps_hash_to_the_committed_value() {
    let mut results: Vec<(u64, String, Vec<u32>)> = Vec::new();
    for seed in 0..16u64 {
        let map: GridMap<Square8> = random_map(seed, 64, 64, 20);
        let occupancy = Occupancy::new(&map);
        let hierarchy = Hierarchy::build(&map, 8);
        let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
        let mut path = Vec::new();
        for (from, to) in pairs(&map, seed ^ 0x77, 30) {
            let result = finder.find(
                &map,
                &occupancy,
                &hierarchy,
                from,
                to,
                PathOptions::default(),
                &mut path,
            );
            results.push((
                seed,
                format!("{result:?}"),
                path.iter().map(|cell| cell.0).collect(),
            ));
        }
    }
    let committed = include_str!("../../fixtures/hierarchical_paths.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&results)), committed);
}

/// Writes the committed value. Run with
/// `cargo test -p lockstep-spatial regenerate_hierarchical_paths_hash -- --ignored`, and commit the
/// file with a message that names the rule that changed.
#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore]
fn regenerate_hierarchical_paths_hash() {
    let mut results: Vec<(u64, String, Vec<u32>)> = Vec::new();
    for seed in 0..16u64 {
        let map: GridMap<Square8> = random_map(seed, 64, 64, 20);
        let occupancy = Occupancy::new(&map);
        let hierarchy = Hierarchy::build(&map, 8);
        let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
        let mut path = Vec::new();
        for (from, to) in pairs(&map, seed ^ 0x77, 30) {
            let result = finder.find(
                &map,
                &occupancy,
                &hierarchy,
                from,
                to,
                PathOptions::default(),
                &mut path,
            );
            results.push((
                seed,
                format!("{result:?}"),
                path.iter().map(|cell| cell.0).collect(),
            ));
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/hierarchical_paths.hash"
    );
    std::fs::write(path, format!("{:016x}\n", hash_of(&results))).unwrap();
}

#[test]
fn the_hierarchy_expands_far_fewer_cells_for_some_more_cost() {
    let (mut exact_cost, mut exact_work) = (0u64, 0u64);
    let (mut cost, mut work) = (0u64, 0u64);
    for seed in 100..104u64 {
        let map: GridMap<Square8> = random_map(seed, 192, 192, 20);
        let occupancy = Occupancy::new(&map);
        let hierarchy = Hierarchy::build(&map, 16);
        let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
        let mut exact = Pathfinder::new(&map);
        let mut path = Vec::new();
        for (from, to) in pairs(&map, seed, 60) {
            let wanted = exact.find(
                &map,
                &occupancy,
                from,
                to,
                PathOptions::default(),
                &mut path,
            );
            let got = finder.find(
                &map,
                &occupancy,
                &hierarchy,
                from,
                to,
                PathOptions::default(),
                &mut path,
            );
            if let (PathResult::Found { cost: cheapest }, PathResult::Found { cost: found }) =
                (wanted, got)
            {
                exact_cost += cheapest as u64;
                cost += found as u64;
                exact_work += exact.last_expansions() as u64;
                work += finder.last_expansions() as u64;
            }
        }
    }
    let excess = (cost - exact_cost) * 1_000 / exact_cost;
    eprintln!("excess {excess} per mille, work {work} against {exact_work}");
    assert!(cost >= exact_cost);
    assert!(excess <= 150, "{excess} per mille dearer than the cheapest");
    assert!(
        work * 4 <= exact_work,
        "{work} expansions against {exact_work}"
    );
}
