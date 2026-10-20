// SPDX-License-Identifier: Apache-2.0
//! Benchmark for the spatial crate, with a committed baseline.
//!
//! Measurements, each the best of five runs:
//! - 500 paths on a 512 by 512 map with 20 percent walls: exact, as a batch, and through the
//!   hierarchy (the target is under 50 milliseconds on the development MacBook, which only the
//!   hierarchy meets; decision 0047),
//! - building that hierarchy,
//! - `within` at radius 24 for 1,000 agents,
//! - one flow field over the whole map.
//!
//! `--sweep` also prints the hierarchy's chunk width against its greed.
//!
//! The baseline lives in `benches/baseline.txt` as `name microseconds` lines. The benchmark warns
//! when a measurement is more than 10 percent slower than its baseline, and never fails the build:
//! machines differ, so a warning asks for a look, not a stop. Run `just bench-baseline` after an
//! intended change to write a new baseline.

use lockstep_core::{Handle, StableVector, Streams};
use lockstep_spatial::{
    find_jump_paths, find_paths, find_paths_serially, Cell, FlowField, GridMap,
    HierarchicalPathfinder, Hierarchy, JumpPathfinder, Occupancy, PathOptions, PathRequest,
    PathResult, Pathfinder, Square8,
};

/// Chunk width of the benchmarked hierarchy (decision 0047 has the sweep that chose it).
const HIERARCHY_SIDE: u32 = 16;
const DEFAULT_GREED: u32 = lockstep_spatial::DEFAULT_GREED;
use std::time::Instant;

const BASELINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benches/baseline.txt");

fn big_map() -> GridMap<Square8> {
    let mut streams = Streams::new(1);
    let mut map: GridMap<Square8> = GridMap::new(512, 512);
    for index in 0..map.cell_count() as u32 {
        if streams.range("wall", 0, 100) < 20 {
            map.set_passable(Cell(index), false);
        }
    }
    map
}

fn free(map: &GridMap<Square8>, streams: &mut Streams) -> Cell {
    loop {
        let cell = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
        if map.is_passable(cell) {
            return cell;
        }
    }
}

fn best_of_five(mut work: impl FnMut() -> u64) -> (u128, u64) {
    let mut best = u128::MAX;
    let mut check = 0;
    for _ in 0..5 {
        let start = Instant::now();
        check = work();
        best = best.min(start.elapsed().as_micros());
    }
    (best, check)
}

fn main() {
    let map = big_map();
    let occupancy = Occupancy::new(&map);

    let mut streams = Streams::new(2);
    let pairs: Vec<(Cell, Cell)> = (0..500)
        .map(|_| (free(&map, &mut streams), free(&map, &mut streams)))
        .collect();
    let mut pathfinder = Pathfinder::new(&map);
    let mut expansions = 0u64;
    let (paths, path_check) = best_of_five(|| {
        expansions = 0;
        let mut path = Vec::new();
        let mut total = 0u64;
        for (from, to) in &pairs {
            pathfinder.find(
                &map,
                &occupancy,
                *from,
                *to,
                PathOptions::default(),
                &mut path,
            );
            total += path.len() as u64;
            expansions += pathfinder.last_expansions() as u64;
        }
        total
    });

    // The jump search: exact, so its total cost must equal the plain search's.
    let mut jump_cost = 0u64;
    let mut jump_pathfinder = JumpPathfinder::new(&map).expect("the benchmark map is uniform");
    let mut jump_expansions = 0u64;
    let (jump, _) = best_of_five(|| {
        jump_cost = 0;
        jump_expansions = 0;
        let mut path = Vec::new();
        for (from, to) in &pairs {
            if let PathResult::Found { cost } = jump_pathfinder.find(
                &map,
                &occupancy,
                *from,
                *to,
                PathOptions::default(),
                &mut path,
            ) {
                jump_cost += cost as u64;
            }
            jump_expansions += jump_pathfinder.last_expansions() as u64;
        }
        jump_cost
    });

    // The hierarchy: built once per map, then searched. The exact cost is the yardstick for how
    // much dearer its paths are.
    let mut exact_cost = 0u64;
    let mut unreachable = 0;
    {
        let mut path = Vec::new();
        for (from, to) in &pairs {
            match pathfinder.find(
                &map,
                &occupancy,
                *from,
                *to,
                PathOptions::default(),
                &mut path,
            ) {
                PathResult::Found { cost } => exact_cost += cost as u64,
                _ => unreachable += 1,
            }
        }
    }
    assert_eq!(
        jump_cost, exact_cost,
        "the jump search found a different cost"
    );
    println!("jump: {jump_expansions} jump points expanded");
    let hierarchy_search = |side: u32, greed: u32| {
        let hierarchy = Hierarchy::build(&map, side);
        let mut guided = HierarchicalPathfinder::new(&map, &hierarchy);
        guided.set_greed(greed);
        let mut cost = 0u64;
        let mut expanded = 0u64;
        let (time, _) = best_of_five(|| {
            let mut path = Vec::new();
            cost = 0;
            expanded = 0;
            for (from, to) in &pairs {
                let result = guided.find(
                    &map,
                    &occupancy,
                    &hierarchy,
                    *from,
                    *to,
                    PathOptions::default(),
                    &mut path,
                );
                if let PathResult::Found { cost: found } = result {
                    cost += found as u64;
                }
                expanded += guided.last_expansions() as u64;
            }
            cost
        });
        (time, cost, expanded, hierarchy.node_count())
    };
    if std::env::args().any(|argument| argument == "--sweep") {
        for side in [8, 16, 24, 32] {
            for greed in [10, 11, 12, 13, 15] {
                let (time, cost, expanded, nodes) = hierarchy_search(side, greed);
                println!(
                    "sweep side {side} greed {greed}: {nodes} entrances, {time} microseconds, {expanded} expanded, {}.{:02} percent dearer",
                    (cost - exact_cost) * 100 / exact_cost,
                    ((cost - exact_cost) * 10_000 / exact_cost) % 100
                );
            }
        }
    }
    let (build, _) = best_of_five(|| Hierarchy::build(&map, HIERARCHY_SIDE).node_count() as u64);
    let (hierarchical, hierarchical_cost, hierarchical_expanded, _) =
        hierarchy_search(HIERARCHY_SIDE, DEFAULT_GREED);
    println!(
        "hierarchy: side {HIERARCHY_SIDE}, {hierarchical_expanded} cells and entrances expanded, paths {}.{:02} percent dearer than exact ({unreachable} of 500 unreachable)",
        (hierarchical_cost - exact_cost) * 100 / exact_cost,
        ((hierarchical_cost - exact_cost) * 10_000 / exact_cost) % 100
    );

    // The same 500 paths as one batch: on a thread pool with the `parallel` feature, serially
    // without it. Its answers must equal the serial ones, in order.
    let batch: Vec<PathRequest> = pairs
        .iter()
        .map(|(from, to)| PathRequest {
            from: *from,
            to: *to,
        })
        .collect();
    let (batched, batch_check) = best_of_five(|| {
        find_paths(&map, &occupancy, &batch, PathOptions::default())
            .iter()
            .map(|(_, path)| path.len() as u64)
            .sum()
    });
    assert_eq!(batch_check, path_check, "the batch walked different paths");
    assert_eq!(
        find_paths(&map, &occupancy, &batch, PathOptions::default()),
        find_paths_serially(&map, &occupancy, &batch, PathOptions::default()),
        "the batch answered in a different order"
    );

    let (jump_batched, jump_batch_cost) = best_of_five(|| {
        find_jump_paths(&map, &occupancy, &batch, PathOptions::default())
            .expect("the benchmark map is uniform")
            .iter()
            .map(|(result, _)| match result {
                PathResult::Found { cost } => *cost as u64,
                _ => 0,
            })
            .sum()
    });
    assert_eq!(
        jump_batch_cost, exact_cost,
        "the jump batch found a different cost"
    );

    let mut crowd = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let agents: Vec<Handle> = (0..1_000).map(|_| entities.insert(())).collect();
    let mut streams = Streams::new(3);
    for agent in &agents {
        loop {
            let cell = Cell(streams.range("agent", 0, map.cell_count() as i32) as u32);
            if crowd.place(cell, *agent).is_ok() {
                break;
            }
        }
    }
    let centres: Vec<Cell> = agents
        .iter()
        .map(|agent| crowd.cell_of(*agent).unwrap())
        .collect();
    let (within, within_check) = best_of_five(|| {
        let mut found = Vec::new();
        let mut total = 0u64;
        for centre in &centres {
            crowd.within(&map, *centre, 240, &mut found);
            total += found.len() as u64;
        }
        total
    });

    let target = free(&map, &mut Streams::new(4));
    let (flow, flow_check) = best_of_five(|| {
        let field = FlowField::build(&map, &[target], u32::MAX);
        field.distance(Cell(0)).unwrap_or(0) as u64 + 1
    });

    let measurements = [
        ("paths_500", paths),
        ("paths_500_batch", batched),
        ("within_1000_agents", within),
        ("flow_field", flow),
        ("hierarchy_build", build),
        ("paths_500_hierarchical", hierarchical),
        ("paths_500_jump", jump),
        ("paths_500_jump_batch", jump_batched),
    ];
    println!(
        "checksums (must not change): paths {path_check}, within {within_check}, flow {flow_check}"
    );
    println!(
        "paths: {path_check} cells walked, {expansions} cells expanded, {} nanoseconds per expansion",
        paths * 1_000 / expansions.max(1) as u128
    );
    let write = std::env::args().any(|argument| argument == "--write-baseline");
    if write {
        let text: String = std::iter::once("# SPDX-License-Identifier: Apache-2.0\n".to_string())
            .chain(
                measurements
                    .iter()
                    .map(|(name, microseconds)| format!("{name} {microseconds}\n")),
            )
            .collect();
        std::fs::write(BASELINE, text).expect("write the baseline");
        println!("baseline written to {BASELINE}");
    }
    let baseline = std::fs::read_to_string(BASELINE).unwrap_or_default();
    for (name, microseconds) in measurements {
        let reference = baseline
            .lines()
            .find_map(|line| line.strip_prefix(name)?.trim().parse::<u128>().ok());
        match reference {
            Some(before) if microseconds * 100 > before * 110 => {
                println!(
                    "WARNING {name}: {microseconds} microseconds, {}% slower than the baseline {before}",
                    (microseconds * 100 / before.max(1)) - 100
                )
            }
            Some(before) => {
                println!("ok      {name}: {microseconds} microseconds (baseline {before})")
            }
            None => println!("new     {name}: {microseconds} microseconds (no baseline yet)"),
        }
    }
    println!(
        "target: 500 paths in under 50 milliseconds; exact {} milliseconds, hierarchical {} milliseconds",
        paths / 1_000,
        hierarchical / 1_000
    );
}
