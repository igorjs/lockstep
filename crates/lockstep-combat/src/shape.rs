// SPDX-License-Identifier: Apache-2.0
use crate::frame::{cuts_corner, Frame};
use lockstep_core::math::{atan2, Turn};
use lockstep_core::{Handle, Message};
use lockstep_spatial::{Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// Where a hit lands, relative to the attacker's cell and facing. Facing is a `Turn` measured
/// counter-clockwise from east, with north a quarter turn, on a map whose y grows south.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum HitShape {
    /// The one cell ahead: a knife, a fist.
    Adjacent,
    /// The first body on the line up to `n` steps ahead, stopping at a wall: a spear.
    Reach(u8),
    /// Every cell within `radius` steps whose direction is within `half_angle` of the facing: a
    /// swing.
    Arc { radius: u8, half_angle: Turn },
    /// Every cell within `radius` steps: a cleave.
    Around(u8),
    /// One chosen cell: a thrown object.
    Cell(Cell),
    /// Every body on the line `length` steps ahead, up to the first wall: a beam that pierces.
    /// Finding the cell ahead scans a square `2 × length + 3` cells wide, so keep lines short.
    Line { length: u8 },
}

/// The angle from one cell's centre to another's, counter-clockwise from east.
pub fn direction<T: Topology>(map: &GridMap<T>, from: Cell, to: Cell) -> Turn {
    let (a, b) = (map.centre(from), map.centre(to));
    // The map's y grows south; angles count north as positive.
    atan2(a.y - b.y, b.x - a.x)
}

/// How far apart two angles are, either way round.
pub fn angle_between(a: Turn, b: Turn) -> Turn {
    (a.wrapping_sub(b) as i16).unsigned_abs()
}

/// The bodies a shape hits, sorted by cell, never the attacker. `out` is cleared first.
pub fn hits<T: Topology>(
    shape: HitShape,
    map: &GridMap<T>,
    occupancy: &Occupancy,
    attacker: Handle,
    from: Cell,
    facing: Turn,
    out: &mut Vec<(Cell, Handle)>,
) {
    out.clear();
    match shape {
        HitShape::Cell(target) => {
            // A target from outside may lie off the grid; it hits nobody.
            if target.0 as usize >= map.cell_count() {
                return;
            }
            if let Some(body) = occupancy.at(target) {
                if body != attacker {
                    out.push((target, body));
                }
            }
        }
        HitShape::Adjacent => along_line(map, occupancy, attacker, from, facing, 1, true, out),
        HitShape::Reach(n) => {
            along_line(map, occupancy, attacker, from, facing, n as u32, true, out)
        }
        HitShape::Line { length } => along_line(
            map,
            occupancy,
            attacker,
            from,
            facing,
            length as u32,
            false,
            out,
        ),
        HitShape::Arc { radius, half_angle } => {
            within_angle(map, occupancy, from, radius as u32, facing, half_angle, out)
        }
        HitShape::Around(radius) => {
            within_steps(map, occupancy, from, radius as u32, out);
        }
    }
    out.retain(|(_, body)| *body != attacker);
}

/// Bodies on the line from `from` toward the cell `length` steps ahead (chosen as if the map had
/// no edge), cut at the first wall, wall corner or map edge; only the first when `first_only`. The
/// attacker's own cells are passed over, so a body larger than one cell can strike along itself.
#[allow(clippy::too_many_arguments)]
fn along_line<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    attacker: Handle,
    from: Cell,
    facing: Turn,
    length: u32,
    first_only: bool,
    out: &mut Vec<(Cell, Handle)>,
) {
    let frame = Frame::around(map, length);
    let mut previous = from;
    for cell in frame.path_ahead::<T>(from, facing, length) {
        if !map.is_passable(cell) || cuts_corner(map, previous, cell) {
            break;
        }
        previous = cell;
        match occupancy.at(cell) {
            Some(body) if body != attacker => {
                out.push((cell, body));
                if first_only {
                    break;
                }
            }
            _ => {}
        }
    }
    out.sort_unstable_by_key(|(cell, _)| *cell);
}

/// Bodies within `radius` steps of `from`, not counting `from` itself.
fn within_steps<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    from: Cell,
    radius: u32,
    out: &mut Vec<(Cell, Handle)>,
) {
    // Every topology's step costs at most 14 tenths, so this radius finds every candidate.
    occupancy.within(map, from, radius.saturating_mul(14), out);
    out.retain(|(cell, _)| *cell != from && map.steps(from, *cell) <= radius);
}

fn within_angle<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    from: Cell,
    radius: u32,
    facing: Turn,
    half_angle: Turn,
    out: &mut Vec<(Cell, Handle)>,
) {
    within_steps(map, occupancy, from, radius, out);
    out.retain(|(cell, _)| angle_between(direction(map, from, *cell), facing) <= half_angle);
}
