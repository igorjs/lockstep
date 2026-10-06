use crate::cell::Cell;

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
/// diagonal steps.
fn square_line(from: Cell, to: Cell, width: u32, four_connected: bool, out: &mut Vec<Cell>) {
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
            let (nc, nr) = hex::from_cube(x + dx, z + dz);
            if inside(nc, nr, width, height) {
                out.push(join(nc, nr, width));
            }
        }
    }

    fn distance(a: Cell, b: Cell, width: u32) -> u32 {
        let ((ac, ar), (bc, br)) = (split(a, width), split(b, width));
        let (ax, ay, az) = hex::to_cube(ac, ar);
        let (bx, by, bz) = hex::to_cube(bc, br);
        10 * (((ax - bx).abs() + (ay - by).abs() + (az - bz).abs()) / 2) as u32
    }

    fn step_cost(from: Cell, to: Cell, width: u32) -> u32 {
        Self::distance(from, to, width)
    }

    fn line(from: Cell, to: Cell, width: u32, out: &mut Vec<Cell>) {
        out.clear();
        let ((fc, fr), (tc, tr)) = (split(from, width), split(to, width));
        let (ax, _, az) = hex::to_cube(fc, fr);
        let (bx, _, bz) = hex::to_cube(tc, tr);
        let (ay, by) = (-ax - az, -bx - bz);
        let steps = (((ax - bx).abs() + (ay - by).abs() + (az - bz).abs()) / 2).max(0);
        if steps == 0 {
            out.push(from);
            return;
        }
        for index in 0..=steps {
            // Interpolate every coordinate scaled by `steps`, round each to nearest, then repair the
            // one that moved the most so the three still add up to zero.
            let scaled = |a: i64, b: i64| a * (steps - index) + b * index;
            let (sx, sy, sz) = (scaled(ax, bx), scaled(ay, by), scaled(az, bz));
            let (mut rx, ry, mut rz) = (
                hex::divide_round(sx, steps),
                hex::divide_round(sy, steps),
                hex::divide_round(sz, steps),
            );
            let (error_x, error_y, error_z) = (
                (rx * steps - sx).abs(),
                (ry * steps - sy).abs(),
                (rz * steps - sz).abs(),
            );
            if error_x > error_y && error_x > error_z {
                rx = -ry - rz;
            } else if error_y <= error_z {
                rz = -rx - ry;
            }
            // When y moved the most, it is recomputed from x and z, which is what dropping it does.
            let (column, row) = hex::from_cube(rx, rz);
            out.push(join(column, row, width));
        }
    }
}
