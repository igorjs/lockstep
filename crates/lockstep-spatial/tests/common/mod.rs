// SPDX-License-Identifier: Apache-2.0
//! Shared test helpers: seeded random maps.

#![allow(dead_code)]

use lockstep_core::Streams;
use lockstep_spatial::{Cell, GridMap, Topology};

/// A map with about `wall_percent` percent walls, a few expensive cells, and gentle hills, all drawn
/// from one seeded stream so every platform builds the same map.
pub fn random_map<T: Topology>(
    seed: u64,
    width: u32,
    height: u32,
    wall_percent: u32,
) -> GridMap<T> {
    let mut streams = Streams::new(seed);
    let mut map: GridMap<T> = GridMap::new(width, height);
    map.set_step_limit(1);
    for y in 0..height {
        for x in 0..width {
            let cell = map.index(x, y);
            if streams.range("wall", 0, 100) < wall_percent as i32 {
                map.set_passable(cell, false);
                continue;
            }
            if streams.range("mud", 0, 100) < 8 {
                map.set_cost(cell, streams.range("mud", 2, 5) as u8);
            }
            map.set_elevation(cell, ((x / 4 + y / 4) % 3) as u8);
        }
    }
    map
}

/// A free cell chosen from the stream, or the first free cell if the map is nearly full.
pub fn free_cell<T: Topology>(map: &GridMap<T>, streams: &mut Streams, name: &str) -> Cell {
    for _ in 0..200 {
        let cell = map.index(
            streams.range(name, 0, map.width() as i32) as u32,
            streams.range(name, 0, map.height() as i32) as u32,
        );
        if map.is_passable(cell) {
            return cell;
        }
    }
    (0..map.cell_count() as u32)
        .map(Cell)
        .find(|cell| map.is_passable(*cell))
        .expect("a free cell")
}
