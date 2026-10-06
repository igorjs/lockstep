// SPDX-License-Identifier: Apache-2.0
//! Decision: knockback is a move through occupancy, one cell at a time; it stops at the first
//! wall, map edge or body and reports what it hit, and the simulation decides the impact damage.
//! Alternative rejected: moving the body straight to the end cell, which passes through walls and
//! bodies, or dealing impact damage inside the crate, which every simulation tunes differently.
//! Would change if: a knockback into a wall goes past it, or does not report the impact.

use crate::common::{arena, EAST};
use lockstep_combat::{knock_back, Impact};
use lockstep_spatial::Square8;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_knockback_into_a_wall_stops_there_and_reports_the_impact() {
    let mut arena = arena::<Square8>(8, 3, &[(1, 1)]);
    let wall = arena.map.index(4, 1);
    arena.map.set_passable(wall, false);
    let body = arena.bodies[0].1;
    let knocked = knock_back(&arena.map, &mut arena.occupancy, body, EAST, 5);
    assert_eq!(arena.map.coordinates(knocked.at), (3, 1));
    assert_eq!((knocked.moved, knocked.impact), (2, Some(Impact::Wall)));
    assert_eq!(arena.occupancy.cell_of(body), Some(knocked.at));
    // The simulation turns the impact into damage: here, ten per cell it did not travel.
    let impact_damage = 10 * (5 - knocked.moved as i32);
    assert_eq!(impact_damage, 30);
}
