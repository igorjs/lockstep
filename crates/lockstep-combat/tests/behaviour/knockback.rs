// SPDX-License-Identifier: Apache-2.0
use crate::common::{arena, EAST, NORTH};
use lockstep_combat::{knock_back, Impact};
use lockstep_spatial::Square8;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_clear_knockback_travels_the_full_distance() {
    let mut arena = arena::<Square8>(8, 8, &[(1, 6)]);
    let body = arena.bodies[0].1;
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, NORTH, 4);
    assert_eq!(
        (arena.map.coordinates(knocked.at.unwrap()), knocked.moved),
        ((1, 2), 4)
    );
    assert_eq!(knocked.impact, None);
}

#[test]
fn a_body_in_the_way_stops_the_knockback_and_is_named() {
    let mut arena = arena::<Square8>(8, 3, &[(1, 1), (4, 1)]);
    let (body, blocker) = (arena.bodies[0].1, arena.bodies[1].1);
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 5);
    assert_eq!(arena.map.coordinates(knocked.at.unwrap()), (3, 1));
    assert_eq!(knocked.impact, Some(Impact::Body(blocker)));
}

#[test]
fn the_map_edge_stops_a_knockback_instead_of_sliding_along_it() {
    let mut arena = arena::<Square8>(5, 5, &[(3, 2)]);
    let body = arena.bodies[0].1;
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 3);
    assert_eq!(
        arena.map.coordinates(knocked.at.unwrap()),
        (4, 2),
        "one step, then the edge"
    );
    assert_eq!((knocked.moved, knocked.impact), (1, Some(Impact::Wall)));
}

#[test]
fn a_body_not_on_the_map_is_left_alone() {
    let mut arena = arena::<Square8>(5, 5, &[(2, 2)]);
    let body = arena.bodies[0].1;
    arena.occupancy.vacate(body);
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 3);
    assert_eq!((knocked.at, knocked.moved, knocked.impact), (None, 0, None));
}

#[test]
fn a_body_two_cells_wide_keeps_its_shape() {
    let mut arena = arena::<Square8>(8, 5, &[(1, 2)]);
    let body = arena.bodies[0].1;
    let wide = [arena.map.index(1, 2), arena.map.index(1, 3)];
    arena.occupancy.move_footprint(body, &wide).unwrap();
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 3);
    assert_eq!((knocked.moved, knocked.impact), (3, None));
    let held: Vec<_> = arena
        .occupancy
        .cells_of(body)
        .unwrap()
        .iter()
        .map(|cell| arena.map.coordinates(*cell))
        .collect();
    assert_eq!(held, [(4, 2), (4, 3)]);
}

#[test]
fn a_wide_body_stops_when_any_of_its_cells_would_hit() {
    let mut arena = arena::<Square8>(8, 5, &[(1, 2), (3, 3)]);
    let (body, blocker) = (arena.bodies[0].1, arena.bodies[1].1);
    let wide = [arena.map.index(1, 2), arena.map.index(1, 3)];
    arena.occupancy.move_footprint(body, &wide).unwrap();
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 3);
    assert_eq!(
        (knocked.moved, knocked.impact),
        (1, Some(Impact::Body(blocker)))
    );
}

#[cfg(feature = "hex")]
#[test]
fn a_push_due_north_on_hexagons_lands_due_north() {
    use crate::common::NORTH;
    let mut arena = arena::<lockstep_spatial::Hex>(9, 9, &[(4, 6)]);
    let body = arena.bodies[0].1;
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, NORTH, 2);
    assert_eq!((knocked.moved, knocked.impact), (2, None));
    assert_eq!(
        arena.map.coordinates(knocked.at.unwrap()),
        (4, 4),
        "two rows up, same column"
    );
}
