// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_core::{Handle, StableVector};
use lockstep_spatial::{Cell, GridMap, Occupancy, Topology};

/// A map with a body on every listed cell, the attacker first.
pub struct Arena<T: Topology> {
    pub map: GridMap<T>,
    pub occupancy: Occupancy,
    pub bodies: Vec<(Cell, Handle)>,
}

pub fn arena<T: Topology>(width: u32, height: u32, cells: &[(u32, u32)]) -> Arena<T> {
    let map: GridMap<T> = GridMap::new(width, height);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let bodies = cells
        .iter()
        .map(|(x, y)| {
            let cell = map.index(*x, *y);
            let body = store.insert(());
            occupancy.place(cell, body).unwrap();
            (cell, body)
        })
        .collect();
    Arena {
        map,
        occupancy,
        bodies,
    }
}

/// Every cell of the map except the attacker's, filled.
pub fn crowded<T: Topology>(width: u32, height: u32, attacker: (u32, u32)) -> Arena<T> {
    let mut cells = vec![attacker];
    for y in 0..height {
        for x in 0..width {
            if (x, y) != attacker {
                cells.push((x, y));
            }
        }
    }
    arena(width, height, &cells)
}

/// Facings, counter-clockwise from east.
pub const EAST: u16 = 0;
pub const NORTH: u16 = 16_384;
pub const WEST: u16 = 32_768;
pub const SOUTH: u16 = 49_152;
