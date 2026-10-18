// SPDX-License-Identifier: Apache-2.0
use crate::common::{free_cell, random_map};
use lockstep_core::{Handle, StableVector, Streams};
use lockstep_spatial::{
    Cell, GridMap, HierarchicalPathfinder, Hierarchy, Occupancy, PathOptions, PathResult,
    Pathfinder, Square4, Square8, Topology,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SIDE: u32 = 8;

/// Every step is a legal move between neighbours and the cost is the sum of the step costs.
fn assert_valid_path<T: Topology>(map: &GridMap<T>, from: Cell, path: &[Cell], cost: u32) {
    let mut at = from;
    let mut total = 0;
    let mut neighbours = Vec::new();
    for next in path {
        map.neighbours(at, &mut neighbours);
        assert!(
            neighbours.contains(next),
            "{at:?} to {next:?} is not a step"
        );
        assert!(map.can_step(at, *next), "{at:?} to {next:?} is not allowed");
        total += map.step_cost(at, *next);
        at = *next;
    }
    assert_eq!(total, cost, "the reported cost is the sum of the steps");
}

fn check_against_exact<T: Topology>(seed: u64, width: u32, height: u32) {
    let map: GridMap<T> = random_map(seed, width, height, 22);
    let occupancy = Occupancy::new(&map);
    let hierarchy = Hierarchy::build(&map, SIDE);
    let mut hierarchical = HierarchicalPathfinder::new(&map, &hierarchy);
    let mut exact = Pathfinder::new(&map);
    let mut streams = Streams::new(seed ^ 0x55);
    let (mut near, mut far) = (Vec::new(), Vec::new());
    for _ in 0..40 {
        let from = free_cell(&map, &mut streams, "from");
        let to = free_cell(&map, &mut streams, "to");
        let options = PathOptions::default();
        let wanted = exact.find(&map, &occupancy, from, to, options, &mut near);
        let got = hierarchical.find(&map, &occupancy, &hierarchy, from, to, options, &mut far);
        match (wanted, got) {
            (PathResult::Found { cost: cheapest }, PathResult::Found { cost }) => {
                assert!(
                    cost >= cheapest,
                    "cheaper than the cheapest: {cost} < {cheapest}"
                );
                assert_valid_path(&map, from, &far, cost);
                assert_eq!(
                    far.last().copied(),
                    if from == to { None } else { Some(to) }
                );
            }
            (PathResult::Unreachable, PathResult::Unreachable) => assert!(far.is_empty()),
            other => panic!("seed {seed}: exact and hierarchical disagree: {other:?}"),
        }
    }
}

#[test]
fn paths_are_valid_and_reachability_matches_the_exact_search_on_square8() {
    for seed in 0..12 {
        check_against_exact::<Square8>(seed, 50, 44);
    }
}

#[test]
fn paths_are_valid_and_reachability_matches_the_exact_search_on_square4() {
    for seed in 20..32 {
        check_against_exact::<Square4>(seed, 41, 53);
    }
}

#[cfg(feature = "hex")]
#[test]
fn paths_are_valid_and_reachability_matches_the_exact_search_on_hexagons() {
    for seed in 40..48 {
        check_against_exact::<lockstep_spatial::Hex>(seed, 40, 36);
    }
}

#[test]
fn an_open_map_gives_a_path_within_ten_percent_of_the_cheapest() {
    let map: GridMap<Square8> = GridMap::new(64, 64);
    let occupancy = Occupancy::new(&map);
    let hierarchy = Hierarchy::build(&map, SIDE);
    let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
    finder.set_greed(10);
    let mut path = Vec::new();
    let (from, to) = (map.index(3, 5), map.index(58, 40));
    let result = finder.find(
        &map,
        &occupancy,
        &hierarchy,
        from,
        to,
        PathOptions::default(),
        &mut path,
    );
    let PathResult::Found { cost } = result else {
        panic!("no path on an open map: {result:?}");
    };
    let cheapest = map.distance(from, to);
    assert!(
        cost >= cheapest && cost * 100 <= cheapest * 110,
        "{cost} against {cheapest}"
    );
}

#[test]
fn a_target_walled_in_its_chunk_is_unreachable_without_searching_the_map() {
    let mut map: GridMap<Square8> = GridMap::new(128, 128);
    let target = map.index(100, 100);
    for (x, y) in [
        (99, 99),
        (100, 99),
        (101, 99),
        (99, 100),
        (101, 100),
        (99, 101),
        (100, 101),
        (101, 101),
    ] {
        map.set_passable(map.index(x, y), false);
    }
    let occupancy = Occupancy::new(&map);
    let hierarchy = Hierarchy::build(&map, 16);
    let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
    let mut path = vec![Cell(1)];
    let result = finder.find(
        &map,
        &occupancy,
        &hierarchy,
        map.index(2, 2),
        target,
        PathOptions::default(),
        &mut path,
    );
    assert_eq!(result, PathResult::Unreachable);
    assert!(path.is_empty());
    assert!(
        finder.last_expansions() < 2_000,
        "expanded {} cells, as many as a flood of the map",
        finder.last_expansions()
    );
}

#[test]
fn a_walled_in_start_in_another_chunk_is_unreachable() {
    let mut map: GridMap<Square8> = GridMap::new(64, 64);
    for (x, y) in [
        (9, 9),
        (10, 9),
        (11, 9),
        (9, 10),
        (11, 10),
        (9, 11),
        (10, 11),
        (11, 11),
    ] {
        map.set_passable(map.index(x, y), false);
    }
    let occupancy = Occupancy::new(&map);
    let hierarchy = Hierarchy::build(&map, 16);
    let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
    let mut path = Vec::new();
    let result = finder.find(
        &map,
        &occupancy,
        &hierarchy,
        map.index(10, 10),
        map.index(60, 60),
        PathOptions::default(),
        &mut path,
    );
    assert_eq!(result, PathResult::Unreachable);
}

#[test]
fn a_hierarchy_of_an_older_map_gives_the_exact_answer() {
    let mut map: GridMap<Square8> = random_map(5, 48, 48, 20);
    let hierarchy = Hierarchy::build(&map, SIDE);
    map.set_passable(map.index(24, 24), true);
    assert!(!hierarchy.is_current(&map));
    let occupancy = Occupancy::new(&map);
    let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
    let mut exact = Pathfinder::new(&map);
    let mut streams = Streams::new(8);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for _ in 0..20 {
        let from = free_cell(&map, &mut streams, "from");
        let to = free_cell(&map, &mut streams, "to");
        let options = PathOptions::default();
        let wanted = exact.find(&map, &occupancy, from, to, options, &mut a);
        let got = finder.find(&map, &occupancy, &hierarchy, from, to, options, &mut b);
        assert_eq!((wanted, &a), (got, &b));
    }
}

#[test]
fn bodies_as_walls_and_expansion_limits_give_the_exact_answer() {
    let map: GridMap<Square8> = random_map(9, 48, 48, 18);
    let mut occupancy = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let bodies: Vec<Handle> = (0..30).map(|_| entities.insert(())).collect();
    let mut streams = Streams::new(10);
    for body in &bodies {
        let _ = occupancy.place(free_cell(&map, &mut streams, "body"), *body);
    }
    let hierarchy = Hierarchy::build(&map, SIDE);
    let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
    let mut exact = Pathfinder::new(&map);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for options in [
        PathOptions {
            treat_occupants_as_walls: true,
            ..PathOptions::default()
        },
        PathOptions {
            maximum_expansions: 60,
            ..PathOptions::default()
        },
    ] {
        for _ in 0..20 {
            let from = free_cell(&map, &mut streams, "from");
            let to = free_cell(&map, &mut streams, "to");
            let wanted = exact.find(&map, &occupancy, from, to, options, &mut a);
            let got = finder.find(&map, &occupancy, &hierarchy, from, to, options, &mut b);
            assert_eq!((wanted, &a), (got, &b));
        }
    }
}

#[test]
fn more_greed_searches_less_and_never_finds_a_cheaper_total() {
    let map: GridMap<Square8> = random_map(11, 96, 96, 20);
    let occupancy = Occupancy::new(&map);
    let hierarchy = Hierarchy::build(&map, 12);
    let mut streams = Streams::new(12);
    let pairs: Vec<(Cell, Cell)> = (0..60)
        .map(|_| {
            (
                free_cell(&map, &mut streams, "from"),
                free_cell(&map, &mut streams, "to"),
            )
        })
        .collect();
    let mut totals = Vec::new();
    for greed in [10, 20] {
        let mut finder = HierarchicalPathfinder::new(&map, &hierarchy);
        finder.set_greed(greed);
        let (mut cost, mut work) = (0u64, 0u64);
        let mut path = Vec::new();
        for (from, to) in &pairs {
            if let PathResult::Found { cost: found } = finder.find(
                &map,
                &occupancy,
                &hierarchy,
                *from,
                *to,
                PathOptions::default(),
                &mut path,
            ) {
                cost += found as u64;
            }
            work += finder.last_expansions() as u64;
        }
        totals.push((cost, work));
    }
    assert!(
        totals[0].0 <= totals[1].0,
        "plain {:?} against greedy {:?}",
        totals[0],
        totals[1]
    );
    assert!(
        totals[1].1 < totals[0].1,
        "plain {:?} against greedy {:?}",
        totals[0],
        totals[1]
    );
}

#[test]
fn two_builds_of_one_map_search_the_same_paths() {
    let map: GridMap<Square8> = random_map(13, 64, 64, 22);
    let occupancy = Occupancy::new(&map);
    let (first, second) = (Hierarchy::build(&map, SIDE), Hierarchy::build(&map, SIDE));
    assert_eq!(first.node_count(), second.node_count());
    let mut a = HierarchicalPathfinder::new(&map, &first);
    let mut b = HierarchicalPathfinder::new(&map, &second);
    let mut streams = Streams::new(14);
    let (mut x, mut y) = (Vec::new(), Vec::new());
    for _ in 0..30 {
        let from = free_cell(&map, &mut streams, "from");
        let to = free_cell(&map, &mut streams, "to");
        let options = PathOptions::default();
        let one = a.find(&map, &occupancy, &first, from, to, options, &mut x);
        let two = b.find(&map, &occupancy, &second, from, to, options, &mut y);
        assert_eq!((one, &x), (two, &y));
    }
}
