// SPDX-License-Identifier: Apache-2.0
//! Decision: hit shapes measure reach in steps and direction between cell centres, so one shape
//! means the same swing on any topology.
//! Alternative rejected: shapes as fixed cell offsets per topology, which must be written again
//! for every topology and drift apart.
//! Would change if: an arc of radius 1 and half-angle 45 degrees facing north stops covering
//! three cells on `Square8` and two on hexagons.

use crate::common::{crowded, NORTH};
use lockstep_combat::{hits, HitShape};
use lockstep_spatial::Square8;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const SWING: HitShape = HitShape::Arc {
    radius: 1,
    half_angle: 8_192,
};

#[test]
fn a_quarter_swing_covers_three_cells_on_square8() {
    let arena = crowded::<Square8>(5, 5, (2, 2));
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        SWING,
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        NORTH,
        &mut out,
    );
    let cells: Vec<_> = out
        .iter()
        .map(|(cell, _)| arena.map.coordinates(*cell))
        .collect();
    assert_eq!(cells, [(1, 1), (2, 1), (3, 1)]);
}

#[cfg(feature = "hex")]
#[test]
fn a_quarter_swing_covers_two_cells_on_hexagons() {
    let arena = crowded::<lockstep_spatial::Hex>(5, 5, (2, 2));
    let (from, attacker) = arena.bodies[0];
    let mut out = Vec::new();
    hits(
        SWING,
        &arena.map,
        &arena.occupancy,
        attacker,
        from,
        NORTH,
        &mut out,
    );
    // Row 2 is even, so its northern neighbours are columns 1 and 2 of row 1.
    let cells: Vec<_> = out
        .iter()
        .map(|(cell, _)| arena.map.coordinates(*cell))
        .collect();
    assert_eq!(cells, [(1, 1), (2, 1)]);
}
