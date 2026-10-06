use lockstep_spatial::{Cell, GridMap, Square4, Square8, CHUNK};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn indices_and_coordinates_round_trip() {
    let map: GridMap<Square8> = GridMap::new(70, 40);
    for (x, y) in [(0, 0), (69, 0), (0, 39), (69, 39), (33, 17)] {
        let cell = map.index(x, y);
        assert_eq!(cell, Cell(y * 70 + x));
        assert_eq!(map.coordinates(cell), (x, y));
    }
    assert_eq!(map.cell_count(), 2_800);
}

#[test]
#[should_panic(expected = "outside the map")]
fn an_index_outside_the_map_panics() {
    let map: GridMap<Square8> = GridMap::new(4, 4);
    map.index(4, 0);
}

#[test]
fn a_new_map_is_open_flat_and_cheap() {
    let map: GridMap<Square8> = GridMap::new(5, 5);
    let cell = map.index(2, 2);
    assert!(map.is_passable(cell));
    assert_eq!(
        (map.cost(cell), map.elevation(cell), map.wall_height(cell)),
        (1, 0, 0)
    );
    assert_eq!(map.step_limit(), 1);
}

#[test]
fn setters_change_one_cell_only() {
    let mut map: GridMap<Square8> = GridMap::new(5, 5);
    let (target, other) = (map.index(2, 2), map.index(3, 2));
    map.set_cost(target, 4);
    map.set_elevation(target, 3);
    map.set_passable(target, false);
    assert_eq!(
        (
            map.cost(target),
            map.elevation(target),
            map.is_passable(target)
        ),
        (4, 3, false)
    );
    assert_eq!(
        map.wall_height(target),
        255,
        "an impassable cell is a full wall"
    );
    assert_eq!(
        (
            map.cost(other),
            map.elevation(other),
            map.is_passable(other)
        ),
        (1, 0, true)
    );
    map.set_passable(target, true);
    assert_eq!(map.wall_height(target), 0, "clearing the wall");
}

#[test]
fn a_low_wall_is_impassable_but_not_a_full_wall() {
    let mut map: GridMap<Square8> = GridMap::new(5, 5);
    let wall = map.index(2, 2);
    map.set_low_wall(wall, 2);
    assert!(!map.is_passable(wall));
    assert_eq!(map.wall_height(wall), 2);
    map.set_low_wall(wall, 0);
    assert!(map.is_passable(wall));
}

#[test]
fn a_step_needs_a_passable_target_and_a_climb_within_the_limit() {
    let mut map: GridMap<Square8> = GridMap::new(5, 5);
    let (from, to) = (map.index(1, 1), map.index(2, 1));
    assert!(map.can_step(from, to));
    map.set_elevation(to, 1);
    assert!(
        map.can_step(from, to),
        "a climb of one is allowed by the default limit"
    );
    map.set_elevation(to, 2);
    assert!(!map.can_step(from, to), "a climb of two is too steep");
    map.set_step_limit(2);
    assert!(map.can_step(from, to));
    map.set_elevation(from, 9);
    assert!(map.can_step(from, to), "a drop of any size is allowed");
    map.set_passable(to, false);
    assert!(!map.can_step(from, to));
}

#[test]
fn a_diagonal_step_may_not_cut_the_corner_of_a_wall() {
    let mut map: GridMap<Square8> = GridMap::new(5, 5);
    let (from, to) = (map.index(1, 1), map.index(2, 2));
    assert!(map.can_step(from, to));
    map.set_passable(map.index(2, 1), false);
    assert!(
        !map.can_step(from, to),
        "one wall beside the step is enough"
    );
    map.set_passable(map.index(2, 1), true);
    map.set_passable(map.index(1, 2), false);
    assert!(!map.can_step(from, to));
    assert!(map.can_step(from, map.index(1, 0)) || !map.is_passable(map.index(1, 0)));
}

#[test]
fn square4_has_no_diagonal_corner_rule_to_apply() {
    let mut map: GridMap<Square4> = GridMap::new(5, 5);
    map.set_passable(map.index(2, 1), false);
    assert!(map.can_step(map.index(1, 1), map.index(1, 2)));
}

#[test]
fn changes_mark_their_chunk_dirty_once_and_taking_clears_them() {
    let mut map: GridMap<Square8> = GridMap::new(100, 70);
    assert!(map.take_dirty_chunks().is_empty());
    let (a, b, c) = (
        map.index(1, 1),
        map.index(CHUNK + 1, 1),
        map.index(1, CHUNK * 2 + 1),
    );
    map.set_cost(c, 3);
    map.set_cost(a, 2);
    map.set_elevation(a, 1);
    map.set_passable(b, false);
    // 100 cells wide is four chunks across: chunk (1, 0) is index 1, chunk (0, 2) is index 8.
    assert_eq!(map.take_dirty_chunks(), vec![0, 1, 8]);
    assert!(map.take_dirty_chunks().is_empty());
}

#[test]
fn many_chunks_can_be_dirty_across_several_words() {
    let mut map: GridMap<Square8> = GridMap::new(CHUNK * 10, CHUNK * 10);
    for chunk_y in 0..10 {
        for chunk_x in 0..10 {
            map.set_cost(map.index(chunk_x * CHUNK, chunk_y * CHUNK), 2);
        }
    }
    assert_eq!(map.take_dirty_chunks(), (0..100).collect::<Vec<u32>>());
}

#[test]
fn a_map_survives_a_save_round_trip_exactly() {
    let mut map: GridMap<Square8> = GridMap::new(40, 40);
    map.set_cost(map.index(3, 3), 5);
    map.set_low_wall(map.index(4, 4), 3);
    map.set_elevation(map.index(5, 5), 2);
    let bytes = bincode::serialize(&map).unwrap();
    let restored: GridMap<Square8> = bincode::deserialize(&bytes).unwrap();
    assert_eq!(restored, map);
}

#[test]
fn the_map_forwards_the_topology_functions() {
    let map: GridMap<Square8> = GridMap::new(10, 10);
    let mut out = Vec::new();
    map.neighbours(map.index(0, 0), &mut out);
    assert_eq!(out.len(), 3);
    assert_eq!(map.distance(map.index(0, 0), map.index(3, 1)), 34);
    assert_eq!(map.step_cost(map.index(0, 0), map.index(1, 1)), 14);
    map.line(map.index(0, 0), map.index(2, 0), &mut out);
    assert_eq!(out.len(), 3);
}
