// SPDX-License-Identifier: Apache-2.0
use crate::common::{arena, crowded, EAST, NORTH, SOUTH, WEST};
use lockstep_combat::{angle_between, direction, hits, HitShape};
use lockstep_spatial::{Cell, Square4, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn coordinates_hit(shape: HitShape, facing: u16) -> Vec<(u32, u32)> {
    let arena = crowded::<Square8>(7, 7, (3, 3));
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        shape,
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        facing,
        &mut out,
    );
    assert!(
        out.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "sorted by cell"
    );
    assert!(
        out.iter().all(|(_, body)| *body != attacker),
        "never the attacker"
    );
    out.iter()
        .map(|(cell, _)| arena.map.coordinates(*cell))
        .collect()
}

#[test]
fn directions_count_counter_clockwise_from_east_with_north_up() {
    let map: lockstep_spatial::GridMap<Square8> = lockstep_spatial::GridMap::new(5, 5);
    let centre = map.index(2, 2);
    assert_eq!(direction(&map, centre, map.index(3, 2)), EAST);
    assert_eq!(direction(&map, centre, map.index(2, 1)), NORTH);
    assert_eq!(direction(&map, centre, map.index(1, 2)), WEST);
    assert_eq!(direction(&map, centre, map.index(2, 3)), SOUTH);
    assert_eq!(angle_between(1_000, 65_000), 1_536);
}

#[test]
fn adjacent_reach_around_and_cell_cover_the_cells_they_name() {
    assert_eq!(coordinates_hit(HitShape::Adjacent, EAST), [(4, 3)]);
    assert_eq!(
        coordinates_hit(HitShape::Reach(3), NORTH),
        [(3, 2)],
        "the first body only"
    );
    assert_eq!(
        coordinates_hit(HitShape::Line { length: 3 }, NORTH),
        [(3, 0), (3, 1), (3, 2)]
    );
    assert_eq!(coordinates_hit(HitShape::Around(1), EAST).len(), 8);
    assert_eq!(coordinates_hit(HitShape::Around(2), EAST).len(), 24);
    assert_eq!(coordinates_hit(HitShape::Cell(Cell(0)), EAST), [(0, 0)]);
}

#[test]
fn a_cell_shape_on_the_attacker_or_empty_ground_hits_nobody() {
    let arena = arena::<Square8>(5, 5, &[(2, 2)]);
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Cell(from),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert!(out.is_empty());
    hits(
        HitShape::Cell(Cell(0)),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert!(out.is_empty());
}

#[test]
fn a_line_hits_every_body_until_the_first_wall() {
    let mut arena = arena::<Square8>(10, 3, &[(0, 1), (2, 1), (4, 1), (7, 1)]);
    arena.map.set_passable(arena.map.index(5, 1), false);
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Line { length: 8 },
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    let cells: Vec<_> = out
        .iter()
        .map(|(cell, _)| arena.map.coordinates(*cell))
        .collect();
    assert_eq!(cells, [(2, 1), (4, 1)], "the body behind the wall is safe");
}

#[test]
fn shapes_work_on_four_neighbours_too() {
    let arena = crowded::<Square4>(5, 5, (2, 2));
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Around(1),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert_eq!(
        out.len(),
        4,
        "one step on Square4 reaches only the four sides"
    );
}

#[test]
fn a_spear_reaches_past_empty_ground_to_the_first_body() {
    let arena = arena::<Square8>(8, 3, &[(0, 1), (3, 1), (5, 1)]);
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Reach(4),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert_eq!(out, [arena.bodies[1]]);
    hits(
        HitShape::Reach(2),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert!(out.is_empty(), "out of reach");
}

#[test]
fn facing_off_the_map_reaches_nothing_instead_of_a_neighbour() {
    let arena = crowded::<Square8>(7, 7, (3, 0));
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    for shape in [
        HitShape::Adjacent,
        HitShape::Reach(3),
        HitShape::Line { length: 3 },
    ] {
        hits(
            shape,
            &arena.map,
            &arena.occupancy,
            attacker,
            from,
            NORTH,
            &mut out,
        );
        assert!(out.is_empty(), "{shape:?} facing the top edge");
    }
}

#[test]
fn a_spear_does_not_pass_through_a_wall_corner() {
    let mut arena = arena::<Square8>(6, 6, &[(1, 4), (3, 2)]);
    // Walls on both sides of the first diagonal step north-east.
    arena.map.set_passable(arena.map.index(1, 3), false);
    arena.map.set_passable(arena.map.index(2, 4), false);
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Reach(3),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        8_192,
        &mut out,
    );
    assert!(out.is_empty(), "the corner blocks the thrust");
}

#[test]
fn a_long_attacker_strikes_along_its_own_body() {
    let mut arena = arena::<Square8>(8, 3, &[(1, 1), (4, 1)]);
    let attacker = arena.bodies[0].1;
    let long = [arena.map.index(1, 1), arena.map.index(2, 1)];
    arena.occupancy.move_footprint(attacker, &long).unwrap();
    let mut out = Vec::new();
    hits(
        HitShape::Reach(4),
        &arena.map,
        &arena.occupancy,
        attacker,
        long[0],
        EAST,
        &mut out,
    );
    assert_eq!(out, [arena.bodies[1]]);
}

#[test]
fn a_thrown_object_aimed_off_the_grid_hits_nobody() {
    let arena = arena::<Square8>(5, 5, &[(2, 2)]);
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        HitShape::Cell(Cell(1_000)),
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        EAST,
        &mut out,
    );
    assert!(out.is_empty());
}
