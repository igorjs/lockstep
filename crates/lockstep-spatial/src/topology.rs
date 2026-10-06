// SPDX-License-Identifier: Apache-2.0
use crate::cell::Cell;
use lockstep_core::math::{Fixed32, Vector2};

/// How cells connect. Determinism depends on the fixed order of `neighbours`.
///
/// Distances and costs are in tenths of a straight step: 10 per straight step, 14 per diagonal
/// on `Square8`.
pub trait Topology: Copy + Default {
    /// The most neighbours a cell has.
    const NEIGHBOURS: usize;

    /// The neighbours inside the map, in a fixed order. Clears `out` first.
    fn neighbours(cell: Cell, width: u32, height: u32, out: &mut Vec<Cell>);

    /// The cost of the cheapest path between two cells on an empty map, in tenths.
    fn distance(a: Cell, b: Cell, width: u32) -> u32;

    /// The cost of one step between two adjacent cells, in tenths.
    fn step_cost(from: Cell, to: Cell, width: u32) -> u32;

    /// The cells on a line from `from` to `to`, both ends included. Clears `out` first.
    fn line(from: Cell, to: Cell, width: u32, out: &mut Vec<Cell>);

    /// How many single steps apart two cells are on an empty map, whatever each step costs: a
    /// diagonal is one step on `Square8`.
    fn steps(a: Cell, b: Cell, width: u32) -> u32;

    /// The cell's centre, in cell widths: x grows east and y grows south. Angles between centres
    /// give directions on any topology.
    fn centre(cell: Cell, width: u32) -> Vector2;
}

fn square_centre(cell: Cell, width: u32) -> Vector2 {
    let (x, y) = split(cell, width);
    Vector2::new(Fixed32::from_int(x as i32), Fixed32::from_int(y as i32))
}

fn split(cell: Cell, width: u32) -> (i64, i64) {
    ((cell.0 % width) as i64, (cell.0 / width) as i64)
}

fn join(x: i64, y: i64, width: u32) -> Cell {
    Cell((y * width as i64 + x) as u32)
}

fn inside(x: i64, y: i64, width: u32, height: u32) -> bool {
    x >= 0 && y >= 0 && x < width as i64 && y < height as i64
}

fn square_neighbours(
    directions: &[(i64, i64)],
    cell: Cell,
    width: u32,
    height: u32,
    out: &mut Vec<Cell>,
) {
    out.clear();
    let (x, y) = split(cell, width);
    for (dx, dy) in directions {
        let (nx, ny) = (x + dx, y + dy);
        if inside(nx, ny, width, height) {
            out.push(join(nx, ny, width));
        }
    }
}

/// Integer line between two points, one cell per step along the longer axis. When
/// `four_connected` is set, a step that changes both axes first changes x, so the line has no
/// diagonal steps. The line is always drawn from the smaller cell index and reversed when needed,
/// so the line from A to B passes exactly the cells of the line from B to A.
fn square_line(from: Cell, to: Cell, width: u32, four_connected: bool, out: &mut Vec<Cell>) {
    if to < from {
        square_line(to, from, width, four_connected, out);
        out.reverse();
        return;
    }
    out.clear();
    let (mut x, mut y) = split(from, width);
    let (target_x, target_y) = split(to, width);
    let dx = (target_x - x).abs();
    let dy = -(target_y - y).abs();
    let step_x = if x < target_x { 1 } else { -1 };
    let step_y = if y < target_y { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        out.push(join(x, y, width));
        if x == target_x && y == target_y {
            return;
        }
        let doubled = 2 * error;
        let move_x = doubled >= dy;
        let move_y = doubled <= dx;
        if move_x {
            error += dy;
            x += step_x;
        }
        if four_connected && move_x && move_y {
            out.push(join(x, y, width));
        }
        if move_y {
            error += dx;
            y += step_y;
        }
    }
}

/// Square cells with eight neighbours: N, NE, E, SE, S, SW, W, NW. North is a smaller y.
/// Octile distance: `10 * max(dx, dy) + 4 * min(dx, dy)`.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Square8;

const EIGHT_DIRECTIONS: [(i64, i64); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

impl Topology for Square8 {
    const NEIGHBOURS: usize = 8;

    fn neighbours(cell: Cell, width: u32, height: u32, out: &mut Vec<Cell>) {
        square_neighbours(&EIGHT_DIRECTIONS, cell, width, height, out);
    }

    fn distance(a: Cell, b: Cell, width: u32) -> u32 {
        let ((ax, ay), (bx, by)) = (split(a, width), split(b, width));
        let (dx, dy) = (
            (ax - bx).unsigned_abs() as u32,
            (ay - by).unsigned_abs() as u32,
        );
        10 * dx.max(dy) + 4 * dx.min(dy)
    }

    fn step_cost(from: Cell, to: Cell, width: u32) -> u32 {
        Self::distance(from, to, width)
    }

    fn line(from: Cell, to: Cell, width: u32, out: &mut Vec<Cell>) {
        square_line(from, to, width, false, out);
    }

    fn steps(a: Cell, b: Cell, width: u32) -> u32 {
        let ((ax, ay), (bx, by)) = (split(a, width), split(b, width));
        (ax - bx).unsigned_abs().max((ay - by).unsigned_abs()) as u32
    }

    fn centre(cell: Cell, width: u32) -> Vector2 {
        square_centre(cell, width)
    }
}

/// Square cells with four neighbours: N, E, S, W. Manhattan distance.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Square4;

const FOUR_DIRECTIONS: [(i64, i64); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

impl Topology for Square4 {
    const NEIGHBOURS: usize = 4;

    fn neighbours(cell: Cell, width: u32, height: u32, out: &mut Vec<Cell>) {
        square_neighbours(&FOUR_DIRECTIONS, cell, width, height, out);
    }

    fn distance(a: Cell, b: Cell, width: u32) -> u32 {
        let ((ax, ay), (bx, by)) = (split(a, width), split(b, width));
        10 * ((ax - bx).unsigned_abs() as u32 + (ay - by).unsigned_abs() as u32)
    }

    fn step_cost(from: Cell, to: Cell, width: u32) -> u32 {
        Self::distance(from, to, width)
    }

    fn line(from: Cell, to: Cell, width: u32, out: &mut Vec<Cell>) {
        square_line(from, to, width, true, out);
    }

    fn steps(a: Cell, b: Cell, width: u32) -> u32 {
        let ((ax, ay), (bx, by)) = (split(a, width), split(b, width));
        ((ax - bx).unsigned_abs() + (ay - by).unsigned_abs()) as u32
    }

    fn centre(cell: Cell, width: u32) -> Vector2 {
        square_centre(cell, width)
    }
}

/// Hexagonal cells with odd-row offset storage: odd rows are shifted half a cell to the right.
/// Six neighbours, clockwise from the north-east: NE, E, SE, SW, W, NW. Distance is the cube
/// distance, and a line is the integer-rounded interpolation of cube coordinates.
#[cfg(feature = "hex")]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Hex;

#[cfg(feature = "hex")]
mod hex {
    /// Cube coordinates (x, y, z) with x + y + z = 0, from an odd-row offset position.
    pub fn to_cube(column: i64, row: i64) -> (i64, i64, i64) {
        let x = column - (row - (row & 1)) / 2;
        let z = row;
        (x, -x - z, z)
    }

    pub fn from_cube(x: i64, z: i64) -> (i64, i64) {
        (x + (z - (z & 1)) / 2, z)
    }

    /// Cube-coordinate steps clockwise from the north-east.
    pub const CUBE_DIRECTIONS: [(i64, i64, i64); 6] = [
        (1, 0, -1),
        (1, -1, 0),
        (0, -1, 1),
        (-1, 0, 1),
        (-1, 1, 0),
        (0, 1, -1),
    ];

    /// Scale for the nudge that breaks ties: far finer than any real difference along a line.
    const NUDGE_SCALE: i64 = 1_000;

    /// The hex (as a column and row) at position `index` of `steps` along the line from `a` to `b`
    /// in cube coordinates, with the point nudged by `nudge` thousandths of a hex so it never lies
    /// exactly on an edge. The nudge adds up to zero, so the point stays on the cube plane.
    pub fn round_on_line(
        a: (i64, i64, i64),
        b: (i64, i64, i64),
        steps: i64,
        index: i64,
        nudge: (i64, i64, i64),
    ) -> (i64, i64) {
        let scale = steps * NUDGE_SCALE;
        let scaled = |from: i64, to: i64, offset: i64| {
            (from * (steps - index) + to * index) * NUDGE_SCALE + offset
        };
        let (sx, sy, sz) = (
            scaled(a.0, b.0, nudge.0),
            scaled(a.1, b.1, nudge.1),
            scaled(a.2, b.2, nudge.2),
        );
        let (mut rx, ry, mut rz) = (
            divide_round(sx, scale),
            divide_round(sy, scale),
            divide_round(sz, scale),
        );
        let (error_x, error_y, error_z) = (
            (rx * scale - sx).abs(),
            (ry * scale - sy).abs(),
            (rz * scale - sz).abs(),
        );
        if error_x > error_y && error_x > error_z {
            rx = -ry - rz;
        } else if error_y <= error_z {
            rz = -rx - ry;
        }
        // When y moved the most, it is recomputed from x and z, which is what dropping it does.
        from_cube(rx, rz)
    }

    /// `numerator / denominator` rounded to nearest, ties away from zero, for a positive denominator.
    pub fn divide_round(numerator: i64, denominator: i64) -> i64 {
        let half = denominator / 2;
        if numerator >= 0 {
            (numerator + half) / denominator
        } else {
            -((-numerator + half) / denominator)
        }
    }
}

#[cfg(feature = "hex")]
impl Topology for Hex {
    const NEIGHBOURS: usize = 6;

    fn neighbours(cell: Cell, width: u32, height: u32, out: &mut Vec<Cell>) {
        out.clear();
        let (column, row) = split(cell, width);
        let (x, _, z) = hex::to_cube(column, row);
        for (dx, _, dz) in hex::CUBE_DIRECTIONS {
            let (neighbour_column, neighbour_row) = hex::from_cube(x + dx, z + dz);
            if inside(neighbour_column, neighbour_row, width, height) {
                out.push(join(neighbour_column, neighbour_row, width));
            }
        }
    }

    fn distance(a: Cell, b: Cell, width: u32) -> u32 {
        let ((a_column, a_row), (b_column, b_row)) = (split(a, width), split(b, width));
        let (ax, ay, az) = hex::to_cube(a_column, a_row);
        let (bx, by, bz) = hex::to_cube(b_column, b_row);
        10 * (((ax - bx).abs() + (ay - by).abs() + (az - bz).abs()) / 2) as u32
    }

    fn step_cost(from: Cell, to: Cell, width: u32) -> u32 {
        Self::distance(from, to, width)
    }

    fn line(from: Cell, to: Cell, width: u32, out: &mut Vec<Cell>) {
        out.clear();
        let ((from_column, from_row), (to_column, to_row)) = (split(from, width), split(to, width));
        let (ax, ay, az) = hex::to_cube(from_column, from_row);
        let (bx, by, bz) = hex::to_cube(to_column, to_row);
        let steps = ((ax - bx).abs() + (ay - by).abs() + (az - bz).abs()) / 2;
        if steps == 0 {
            out.push(from);
            return;
        }
        // The map's height is unknown here, so "inside" means inside the columns; rows stay
        // between the two ends, which are inside the map.
        let inside = |column: i64| (0..width as i64).contains(&column);
        for index in 0..=steps {
            // A point exactly on the edge between two hexes is a tie. Nudge it a little one way and
            // then the other, and keep the first hex that is inside the map: on an edge column one
            // of the two always is.
            let first = hex::round_on_line((ax, ay, az), (bx, by, bz), steps, index, (1, 2, -3));
            let second = hex::round_on_line((ax, ay, az), (bx, by, bz), steps, index, (-1, -2, 3));
            let (column, row) = if inside(first.0) { first } else { second };
            out.push(join(column, row, width));
        }
    }

    fn steps(a: Cell, b: Cell, width: u32) -> u32 {
        Self::distance(a, b, width) / 10
    }

    /// Odd rows sit half a cell east, and rows are √3/2 apart, so every neighbour is one cell
    /// width away.
    fn centre(cell: Cell, width: u32) -> Vector2 {
        let (column, row) = split(cell, width);
        let shift = if row % 2 == 1 {
            Fixed32::HALF
        } else {
            Fixed32::ZERO
        };
        // √3/2 in 16.16, rounded to nearest.
        let row_height = Fixed32::from_raw(56_756);
        Vector2::new(
            Fixed32::from_int(column as i32) + shift,
            Fixed32::from_int(row as i32) * row_height,
        )
    }
}
