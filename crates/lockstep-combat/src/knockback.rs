// SPDX-License-Identifier: Apache-2.0
use crate::frame::Frame;
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
    /// Where the body's anchor ended up; `None` when the body was not on the map.
    pub at: Option<Cell>,
    pub moved: u8,
    /// Set when a wall, the map edge or a body stopped it before `cells`; the simulation decides
    /// whether that deals impact damage.
    pub impact: Option<Impact>,
}

/// Pushes `who` up to `cells` steps along `heading`. The path is the line toward the cell `cells`
/// steps ahead, chosen as if the map had no edge, so a push into the edge stops instead of sliding
/// along it, and a push between two neighbours alternates instead of drifting. Every cell of the
/// body takes the same step, so a body larger than one cell keeps its shape. It stops at the first
/// wall, edge or other body, moving through occupancy. A body not on the map is left alone.
pub fn knock_back<T: Topology>(
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    who: Handle,
    heading: Turn,
    cells: u8,
) -> Knocked {
    let Some(footprint) = occupancy.cells_of(who).map(<[Cell]>::to_vec) else {
        return Knocked {
            at: None,
            moved: 0,
            impact: None,
        };
    };
    let anchor = footprint[0];
    // Pad enough for the path and the body's size.
    let size = footprint
        .iter()
        .map(|cell| map.steps(anchor, *cell))
        .max()
        .unwrap_or(0);
    let frame = Frame::around(map, cells as u32 + size + 1);
    let start = frame.to_frame(anchor);
    let end = frame.ahead::<T>(start, heading, cells as u32);
    let mut path = Vec::new();
    frame.line::<T>(start, end, &mut path);

    let mut body: Vec<Cell> = footprint.iter().map(|cell| frame.to_frame(*cell)).collect();
    let mut moved = 0;
    for step in path.windows(2) {
        let Some(direction) = frame.direction_index::<T>(step[0], step[1]) else {
            break;
        };
        let mut next = Vec::with_capacity(body.len());
        let mut impact = None;
        for (cell, frame_cell) in body.iter().map(|cell| (frame.to_map(*cell), *cell)) {
            let target = frame
                .neighbour_at::<T>(frame_cell, direction)
                .and_then(|target| frame.to_map(target).map(|on_map| (target, on_map)));
            match (cell, target) {
                (Some(cell), Some((target, on_map))) if map.can_step(cell, on_map) => {
                    if let Some(other) = occupancy.at(on_map).filter(|holder| *holder != who) {
                        impact = Some(Impact::Body(other));
                    }
                    next.push((target, on_map));
                }
                _ => impact = impact.or(Some(Impact::Wall)),
            }
        }
        if impact.is_some() {
            return Knocked {
                at: occupancy.cell_of(who),
                moved,
                impact,
            };
        }
        let targets: Vec<Cell> = next.iter().map(|(_, on_map)| *on_map).collect();
        occupancy
            .move_footprint(who, &targets)
            .expect("every target was free or already this body's");
        body = next.into_iter().map(|(target, _)| target).collect();
        moved += 1;
    }
    Knocked {
        at: occupancy.cell_of(who),
        moved,
        impact: None,
    }
}
