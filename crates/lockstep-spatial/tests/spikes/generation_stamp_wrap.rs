// SPDX-License-Identifier: Apache-2.0
//! Spike: does forcing the pathfinder's generation counter to wrap give the same paths as a fresh
//! pathfinder?
//!
//! The counter lets the search reset its buffers in constant time. After four billion searches it
//! wraps to zero, and the pathfinder must then clear its stamps. A stale stamp from an old search
//! must never be mistaken for a current one.

use crate::common::{free_cell, random_map};
use lockstep_core::Streams;
use lockstep_spatial::{GridMap, Occupancy, PathOptions, Pathfinder, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn wrapping_the_generation_counter_changes_no_path() {
    let map: GridMap<Square8> = random_map(11, 28, 28, 24);
    let occupancy = Occupancy::new(&map);
    let mut streams = Streams::new(4);
    let mut wrapping = Pathfinder::new(&map);
    // Dirty the stamps with a few real searches, then jump the counter to just before the wrap.
    let mut scratch = Vec::new();
    for _ in 0..5 {
        let (from, to) = (
            free_cell(&map, &mut streams, "a"),
            free_cell(&map, &mut streams, "b"),
        );
        wrapping.find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut scratch,
        );
    }
    wrapping.force_generation(u32::MAX - 2);
    for _ in 0..8 {
        // These searches cross the wrap: generation u32::MAX - 1, u32::MAX, then 0 (cleared) and 1.
        let (from, to) = (
            free_cell(&map, &mut streams, "a"),
            free_cell(&map, &mut streams, "b"),
        );
        let (mut left, mut right) = (Vec::new(), Vec::new());
        let wrapped = wrapping.find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut left,
        );
        let fresh = Pathfinder::new(&map).find(
            &map,
            &occupancy,
            from,
            to,
            PathOptions::default(),
            &mut right,
        );
        assert_eq!((wrapped, left), (fresh, right));
    }
}
