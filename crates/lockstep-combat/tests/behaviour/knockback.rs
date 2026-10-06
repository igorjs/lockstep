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
        (arena.map.coordinates(knocked.at), knocked.moved),
        ((1, 2), 4)
    );
    assert_eq!(knocked.impact, None);
}

#[test]
fn a_body_in_the_way_stops_the_knockback_and_is_named() {
    let mut arena = arena::<Square8>(8, 3, &[(1, 1), (4, 1)]);
    let (body, blocker) = (arena.bodies[0].1, arena.bodies[1].1);
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 5);
    assert_eq!(arena.map.coordinates(knocked.at), (3, 1));
    assert_eq!(knocked.impact, Some(Impact::Body(blocker)));
}

#[test]
fn the_map_edge_stops_a_knockback_instead_of_sliding_along_it() {
    let mut arena = arena::<Square8>(5, 5, &[(3, 2)]);
    let body = arena.bodies[0].1;
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 3);
    assert_eq!(
        arena.map.coordinates(knocked.at),
        (4, 2),
        "one step, then the edge"
    );
    assert_eq!((knocked.moved, knocked.impact), (1, Some(Impact::Wall)));
}
