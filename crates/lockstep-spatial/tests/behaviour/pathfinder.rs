use crate::common::{free_cell, random_map};
use lockstep_core::{Handle, StableVector, Streams};
use lockstep_spatial::{
    Cell, FlowField, GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Square4, Square8,
    Topology,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn find<T: Topology>(map: &GridMap<T>, from: Cell, to: Cell) -> (PathResult, Vec<Cell>) {
    let occupancy = Occupancy::new(map);
    let mut pathfinder = Pathfinder::new(map);
    let mut path = Vec::new();
    let result = pathfinder.find(map, &occupancy, from, to, PathOptions::default(), &mut path);
    (result, path)
}

/// Every step is a legal move between neighbours and the cost is the sum of the step costs.
fn assert_valid_path<T: Topology>(map: &GridMap<T>, from: Cell, path: &[Cell], cost: u32) {
    let mut at = from;
    let mut total = 0;
    let mut neighbours = Vec::new();
    for next in path {
        map.neighbours(at, &mut neighbours);
        assert!(
            neighbours.contains(next),
            "{at:?} to {next:?} is not a neighbour step"
        );
        assert!(map.can_step(at, *next), "{at:?} to {next:?} is not allowed");
        total += map.step_cost(at, *next);
        at = *next;
    }
    assert_eq!(total, cost, "the reported cost is the sum of the steps");
}

#[test]
fn a_path_to_the_same_cell_is_empty_and_free() {
    let map: GridMap<Square8> = GridMap::new(8, 8);
    let cell = map.index(3, 3);
    assert_eq!(
        find(&map, cell, cell),
        (PathResult::Found { cost: 0 }, vec![])
    );
}

#[test]
fn a_straight_path_excludes_the_start_and_includes_the_goal() {
    let map: GridMap<Square8> = GridMap::new(10, 4);
    let (result, path) = find(&map, map.index(0, 1), map.index(5, 1));
    assert_eq!(result, PathResult::Found { cost: 50 });
    assert_eq!(path, (1..=5).map(|x| map.index(x, 1)).collect::<Vec<_>>());
}

#[test]
fn a_diagonal_costs_fourteen_per_step_and_manhattan_cannot_cut_across() {
    let eight: GridMap<Square8> = GridMap::new(8, 8);
    assert_eq!(
        find(&eight, eight.index(0, 0), eight.index(3, 3)).0,
        PathResult::Found { cost: 42 }
    );
    let four: GridMap<Square4> = GridMap::new(8, 8);
    assert_eq!(
        find(&four, four.index(0, 0), four.index(3, 3)).0,
        PathResult::Found { cost: 60 }
    );
}

#[test]
fn a_path_goes_around_a_wall_through_the_gap() {
    let mut map: GridMap<Square8> = GridMap::new(9, 9);
    for y in 0..9 {
        if y != 7 {
            map.set_passable(map.index(4, y), false);
        }
    }
    let (result, path) = find(&map, map.index(0, 0), map.index(8, 0));
    let PathResult::Found { cost } = result else {
        panic!("{result:?}")
    };
    assert!(
        path.contains(&map.index(4, 7)),
        "the only way through is the gap"
    );
    assert!(cost > 80);
    assert_valid_path(&map, map.index(0, 0), &path, cost);
}

#[test]
fn a_sealed_target_is_unreachable_and_a_small_budget_aborts() {
    let mut map: GridMap<Square8> = GridMap::new(9, 9);
    for (x, y) in [
        (3, 3),
        (4, 3),
        (5, 3),
        (3, 4),
        (5, 4),
        (3, 5),
        (4, 5),
        (5, 5),
    ] {
        map.set_passable(map.index(x, y), false);
    }
    assert_eq!(
        find(&map, map.index(0, 0), map.index(4, 4)).0,
        PathResult::Unreachable
    );

    let open: GridMap<Square8> = GridMap::new(40, 40);
    let occupancy = Occupancy::new(&open);
    let mut pathfinder = Pathfinder::new(&open);
    let options = PathOptions {
        maximum_expansions: 5,
        ..PathOptions::default()
    };
    let mut path = Vec::new();
    let result = pathfinder.find(
        &open,
        &occupancy,
        open.index(0, 0),
        open.index(39, 39),
        options,
        &mut path,
    );
    assert_eq!(result, PathResult::Aborted);
}

#[test]
fn expensive_ground_is_avoided_when_a_detour_is_cheaper() {
    let mut map: GridMap<Square4> = GridMap::new(7, 5);
    for y in 0..4 {
        map.set_cost(map.index(3, y), 20);
    }
    let (result, path) = find(&map, map.index(0, 1), map.index(6, 1));
    assert!(
        path.contains(&map.index(3, 4)),
        "the cheap row at the bottom is used"
    );
    let PathResult::Found { cost } = result else {
        panic!()
    };
    assert_valid_path(&map, map.index(0, 1), &path, cost);
    assert!(cost < 20 * 10, "cheaper than walking through the mud");
}

#[test]
fn a_climb_beyond_the_step_limit_forces_a_detour_or_blocks_the_way() {
    let mut map: GridMap<Square4> = GridMap::new(6, 3);
    for y in 0..3 {
        map.set_elevation(map.index(3, y), 3);
    }
    assert_eq!(
        find(&map, map.index(0, 1), map.index(5, 1)).0,
        PathResult::Unreachable,
        "a ridge across the map"
    );
    map.set_elevation(map.index(3, 2), 1);
    let (result, path) = find(&map, map.index(0, 1), map.index(5, 1));
    assert!(matches!(result, PathResult::Found { .. }));
    assert!(path.contains(&map.index(3, 2)), "the low cell is the pass");
}

#[test]
fn a_diagonal_never_cuts_the_corner_of_a_wall() {
    let mut map: GridMap<Square8> = GridMap::new(4, 4);
    map.set_passable(map.index(1, 0), false);
    map.set_passable(map.index(0, 1), false);
    assert_eq!(
        find(&map, map.index(0, 0), map.index(1, 1)).0,
        PathResult::Unreachable
    );
}

#[test]
fn occupants_can_be_treated_as_walls_except_the_destination() {
    let map: GridMap<Square4> = GridMap::new(5, 1);
    let mut occupancy = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let blocker: Handle = entities.insert(());
    occupancy.place(map.index(2, 0), blocker).unwrap();
    let mut pathfinder = Pathfinder::new(&map);
    let mut path = Vec::new();
    let walls = PathOptions {
        treat_occupants_as_walls: true,
        ..PathOptions::default()
    };

    assert_eq!(
        pathfinder.find(
            &map,
            &occupancy,
            map.index(0, 0),
            map.index(4, 0),
            walls,
            &mut path
        ),
        PathResult::Unreachable
    );
    assert_eq!(
        pathfinder.find(
            &map,
            &occupancy,
            map.index(0, 0),
            map.index(4, 0),
            PathOptions::default(),
            &mut path
        ),
        PathResult::Found { cost: 40 },
        "ignoring bodies, the way is open"
    );
    assert!(
        matches!(
            pathfinder.find(
                &map,
                &occupancy,
                map.index(0, 0),
                map.index(2, 0),
                walls,
                &mut path
            ),
            PathResult::Found { cost: 20 }
        ),
        "the destination itself may be occupied"
    );
}

#[test]
fn the_search_matches_a_flow_field_distance_on_random_maps_and_every_path_is_valid() {
    for seed in 0..60 {
        let map: GridMap<Square8> = random_map(seed, 24, 24, 24);
        let mut streams = Streams::new(seed ^ 0x55);
        let target = free_cell(&map, &mut streams, "target");
        let field = FlowField::build(&map, &[target], u32::MAX);
        let occupancy = Occupancy::new(&map);
        let mut pathfinder = Pathfinder::new(&map);
        let mut path = Vec::new();
        for _ in 0..6 {
            let from = free_cell(&map, &mut streams, "from");
            let result = pathfinder.find(
                &map,
                &occupancy,
                from,
                target,
                PathOptions::default(),
                &mut path,
            );
            match (result, field.distance(from)) {
                (PathResult::Found { cost }, Some(distance)) => {
                    assert_eq!(
                        cost, distance,
                        "seed {seed}: the cheapest cost agrees with the flow field"
                    );
                    assert_valid_path(&map, from, &path, cost);
                }
                (PathResult::Unreachable, None) => assert!(path.is_empty()),
                other => panic!("seed {seed}: disagreement {other:?}"),
            }
        }
    }
}

#[test]
fn one_pathfinder_reused_gives_the_same_answers_as_a_fresh_one() {
    let map: GridMap<Square8> = random_map(3, 30, 30, 22);
    let occupancy = Occupancy::new(&map);
    let mut reused = Pathfinder::new(&map);
    let mut streams = Streams::new(8);
    for _ in 0..40 {
        let (from, to) = (
            free_cell(&map, &mut streams, "a"),
            free_cell(&map, &mut streams, "b"),
        );
        let (mut left, mut right) = (Vec::new(), Vec::new());
        let reused_result = reused.find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut left,
        );
        let fresh_result = Pathfinder::new(&map).find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut right,
        );
        assert_eq!((reused_result, left), (fresh_result, right));
    }
}

#[test]
fn the_square4_topology_finds_four_connected_paths() {
    let map: GridMap<Square4> = random_map(9, 20, 20, 20);
    let mut streams = Streams::new(2);
    for _ in 0..30 {
        let (from, to) = (
            free_cell(&map, &mut streams, "a"),
            free_cell(&map, &mut streams, "b"),
        );
        let (result, path) = find(&map, from, to);
        if let PathResult::Found { cost } = result {
            assert_valid_path(&map, from, &path, cost);
        }
    }
}

#[cfg(feature = "hex")]
#[test]
fn hex_paths_are_valid_and_match_the_flow_field() {
    use lockstep_spatial::Hex;
    for seed in 0..20 {
        let map: GridMap<Hex> = random_map(seed, 20, 20, 20);
        let mut streams = Streams::new(seed);
        let target = free_cell(&map, &mut streams, "t");
        let field = FlowField::build(&map, &[target], u32::MAX);
        let from = free_cell(&map, &mut streams, "f");
        let (result, path) = find(&map, from, target);
        match (result, field.distance(from)) {
            (PathResult::Found { cost }, Some(distance)) => {
                assert_eq!(cost, distance);
                assert_valid_path(&map, from, &path, cost);
            }
            (PathResult::Unreachable, None) => {}
            other => panic!("{other:?}"),
        }
    }
}
