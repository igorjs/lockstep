// SPDX-License-Identifier: Apache-2.0
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
            if let Some(body) = occupancy.at(target) {
                if body != attacker {
                    out.push((target, body));
                }
            }
        }
        HitShape::Adjacent => along_line(map, occupancy, from, facing, 1, true, out),
        HitShape::Reach(n) => along_line(map, occupancy, from, facing, n as u32, true, out),
        HitShape::Line { length } => {
            along_line(map, occupancy, from, facing, length as u32, false, out)
        }
        HitShape::Arc { radius, half_angle } => {
            within_angle(map, occupancy, from, radius as u32, facing, half_angle, out)
        }
        HitShape::Around(radius) => {
            within_steps(map, occupancy, from, radius as u32, out);
        }
    }
    out.retain(|(_, body)| *body != attacker);
}

/// Bodies on the line from `from` to the cell `length` steps ahead, cut at the first wall; only
/// the first when `first_only`.
fn along_line<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    from: Cell,
    facing: Turn,
    length: u32,
    first_only: bool,
    out: &mut Vec<(Cell, Handle)>,
) {
    let Some(end) = farthest_ahead(map, from, facing, length) else {
        return;
    };
    let mut cells = Vec::new();
    map.line(from, end, &mut cells);
    for cell in cells.into_iter().skip(1) {
        if !map.is_passable(cell) {
            break;
        }
        if let Some(body) = occupancy.at(cell) {
            out.push((cell, body));
            if first_only {
                break;
            }
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

/// The cell `length` steps away whose direction is nearest the facing, lowest cell on a tie.
fn farthest_ahead<T: Topology>(
    map: &GridMap<T>,
    from: Cell,
    facing: Turn,
    length: u32,
) -> Option<Cell> {
    if length == 0 {
        return None;
    }
    let (x, y) = map.coordinates(from);
    let reach = length as i64 + 1;
    let mut best: Option<(Turn, Cell)> = None;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (cx, cy) = (x as i64 + dx, y as i64 + dy);
            if cx < 0 || cy < 0 || cx >= map.width() as i64 || cy >= map.height() as i64 {
                continue;
            }
            let cell = map.index(cx as u32, cy as u32);
            if map.steps(from, cell) != length {
                continue;
            }
            let off = angle_between(direction(map, from, cell), facing);
            if best.is_none_or(|(best_off, best_cell)| (off, cell) < (best_off, best_cell)) {
                best = Some((off, cell));
            }
        }
    }
    best.map(|(_, cell)| cell)
}
