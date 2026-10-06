// SPDX-License-Identifier: Apache-2.0
use crate::shape::angle_between;
use lockstep_core::math::Turn;
use lockstep_core::{Handle, Message};
use lockstep_spatial::{Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// What stopped a knockback early.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Impact {
    Wall,
    Body(Handle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knocked {
    /// Where the body ended up.
    pub at: Cell,
    pub moved: u8,
    /// Set when a wall, the map edge or a body stopped it before `cells`; the simulation decides
    /// whether that deals impact damage.
    pub impact: Option<Impact>,
}

/// Pushes `who` up to `cells` steps along `heading`, one neighbour at a time: each step takes the
/// neighbour whose direction is nearest the heading (lowest cell on a tie), chosen as if the map
/// had no edge, so a push into the edge stops there instead of sliding along it. It stops at the
/// first wall, map edge or body. The body moves through occupancy, so collision stays occupancy.
pub fn knock_back<T: Topology>(
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    who: Handle,
    heading: Turn,
    cells: u8,
) -> Knocked {
    let mut at = occupancy.cell_of(who).expect("a body on the map");
    for moved in 0..cells {
        let impact = match next_cell(map, at, heading) {
            None => Some(Impact::Wall),
            Some(cell) if !map.can_step(at, cell) => Some(Impact::Wall),
            Some(cell) => occupancy
                .at(cell)
                .filter(|body| *body != who)
                .map(Impact::Body),
        };
        if impact.is_some() {
            return Knocked { at, moved, impact };
        }
        let cell = next_cell(map, at, heading).expect("checked above");
        occupancy.move_to(who, cell).expect("the cell was free");
        at = cell;
    }
    Knocked {
        at,
        moved: cells,
        impact: None,
    }
}

/// The neighbour of `at` nearest `heading` on a map with no edge, or `None` when it lies off the
/// real map. The search runs on a copy of the grid two cells larger on every side; two keeps the
/// row parity hexagons depend on.
fn next_cell<T: Topology>(map: &GridMap<T>, at: Cell, heading: Turn) -> Option<Cell> {
    let (x, y) = map.coordinates(at);
    let (width, height) = (map.width() + 4, map.height() + 4);
    let padded = Cell((y + 2) * width + x + 2);
    let mut neighbours = Vec::new();
    T::neighbours(padded, width, height, &mut neighbours);
    let angle = |cell: Cell| {
        let (a, b) = (T::centre(padded, width), T::centre(cell, width));
        lockstep_core::math::atan2(a.y - b.y, b.x - a.x)
    };
    let best = neighbours
        .into_iter()
        .min_by_key(|cell| (angle_between(angle(*cell), heading), *cell))?;
    let (best_x, best_y) = ((best.0 % width) as i64 - 2, (best.0 / width) as i64 - 2);
    let inside =
        best_x >= 0 && best_y >= 0 && best_x < map.width() as i64 && best_y < map.height() as i64;
    inside.then(|| map.index(best_x as u32, best_y as u32))
}
