// SPDX-License-Identifier: Apache-2.0
//! The consumer scenario for the hierarchical search (decision 0047): couriers plan long routes
//! across a large walled map, with routes from one hierarchy built once.

use lockstep_core::Streams;
use lockstep_spatial::{
    Cell, GridMap, HierarchicalPathfinder, Hierarchy, Occupancy, PathOptions, PathResult,
    Pathfinder, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SIZE: u32 = 160;

fn depot_map() -> GridMap<Square8> {
    let mut streams = Streams::new(77);
    let mut map: GridMap<Square8> = GridMap::new(SIZE, SIZE);
    for index in 0..map.cell_count() as u32 {
        if streams.range("wall", 0, 100) < 20 {
            map.set_passable(Cell(index), false);
        }
    }
    map
}

fn couriers(map: &GridMap<Square8>) -> Vec<(Cell, Cell)> {
    let mut streams = Streams::new(78);
    let mut free = |name: &str| loop {
        let cell = Cell(streams.range(name, 0, map.cell_count() as i32) as u32);
        if map.is_passable(cell) {
            return cell;
        }
    };
    (0..80).map(|_| (free("from"), free("to"))).collect()
}

fn plan(map: &GridMap<Square8>, hierarchy: &Hierarchy) -> Vec<(PathResult, Vec<Cell>)> {
    let occupancy = Occupancy::new(map);
    let mut finder = HierarchicalPathfinder::new(map, hierarchy);
    couriers(map)
        .into_iter()
        .map(|(from, to)| {
            let mut route = Vec::new();
            let result = finder.find(
                map,
                &occupancy,
                hierarchy,
                from,
                to,
                PathOptions::default(),
                &mut route,
            );
            (result, route)
        })
        .collect()
}

#[test]
fn every_courier_gets_a_walkable_route_a_little_dearer_than_the_cheapest() {
    let map = depot_map();
    let hierarchy = Hierarchy::build(&map, 16);
    let occupancy = Occupancy::new(&map);
    let mut exact = Pathfinder::new(&map);
    let (mut planned_cost, mut cheapest_cost) = (0u64, 0u64);
    for ((result, route), (from, to)) in plan(&map, &hierarchy).into_iter().zip(couriers(&map)) {
        let mut best = Vec::new();
        let cheapest = exact.find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut best,
        );
        match (result, cheapest) {
            (PathResult::Found { cost }, PathResult::Found { cost: lowest }) => {
                let mut at = from;
                for next in &route {
                    assert!(
                        map.can_step(at, *next),
                        "{at:?} to {next:?} is not a legal step"
                    );
                    at = *next;
                }
                assert_eq!(at, to);
                planned_cost += cost as u64;
                cheapest_cost += lowest as u64;
            }
            (PathResult::Unreachable, PathResult::Unreachable) => assert!(route.is_empty()),
            other => panic!("disagreement: {other:?}"),
        }
    }
    assert!(planned_cost >= cheapest_cost);
    assert!(
        planned_cost * 100 <= cheapest_cost * 115,
        "{planned_cost} against {cheapest_cost}"
    );
}

#[test]
fn the_same_map_plans_the_same_routes_every_time() {
    let map = depot_map();
    let first = plan(&map, &Hierarchy::build(&map, 16));
    let second = plan(&map, &Hierarchy::build(&map, 16));
    assert_eq!(first, second);
}

/// The jump search (decision 0075) plans the same routes at the same cost as the exact search.
#[test]
fn the_jump_search_plans_routes_of_the_cheapest_cost() {
    let map = depot_map();
    let occupancy = Occupancy::new(&map);
    let mut exact = Pathfinder::new(&map);
    let mut jump =
        lockstep_spatial::JumpPathfinder::new(&map).expect("the depot ground is uniform");
    for (from, to) in couriers(&map) {
        let (mut cheapest, mut route) = (Vec::new(), Vec::new());
        let options = PathOptions::default();
        assert_eq!(
            jump.find(&map, &occupancy, from, to, options, &mut route),
            exact.find(&map, &occupancy, from, to, options, &mut cheapest)
        );
    }
}
