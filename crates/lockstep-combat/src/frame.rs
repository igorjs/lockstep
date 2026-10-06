// SPDX-License-Identifier: Apache-2.0
//! A copy of the grid padded on every side, so a direction can be chosen as if the map had no
//! edge. The padding is even, which keeps the row parity hexagons depend on, so a neighbour's
//! direction is the same in the copy as on the map.

use lockstep_core::math::{atan2, Turn};
use lockstep_spatial::{Cell, GridMap, Topology};

pub(crate) struct Frame {
    width: u32,
    height: u32,
    padding: u32,
    real_width: u32,
    real_height: u32,
}

impl Frame {
    /// A frame with at least `reach` cells of padding on every side.
    pub(crate) fn around<T: Topology>(map: &GridMap<T>, reach: u32) -> Self {
        let padding = (reach + 1).div_ceil(2) * 2;
        Frame {
            width: map.width() + 2 * padding,
            height: map.height() + 2 * padding,
            padding,
            real_width: map.width(),
            real_height: map.height(),
        }
    }

    pub(crate) fn to_frame(&self, cell: Cell) -> Cell {
        let (x, y) = (cell.0 % self.real_width, cell.0 / self.real_width);
        Cell((y + self.padding) * self.width + x + self.padding)
    }

    /// The map cell for a frame cell, or `None` when it lies in the padding.
    pub(crate) fn to_map(&self, cell: Cell) -> Option<Cell> {
        let (x, y) = (
            (cell.0 % self.width) as i64 - self.padding as i64,
            (cell.0 / self.width) as i64 - self.padding as i64,
        );
        let inside = x >= 0 && y >= 0 && x < self.real_width as i64 && y < self.real_height as i64;
        inside.then(|| Cell(y as u32 * self.real_width + x as u32))
    }

    /// Every neighbour of a frame cell, in the topology's fixed order, none cut off by an edge.
    pub(crate) fn neighbours<T: Topology>(&self, cell: Cell, out: &mut Vec<Cell>) {
        T::neighbours(cell, self.width, self.height, out);
    }

    /// The angle from one frame cell's centre to another's, counter-clockwise from east.
    pub(crate) fn direction<T: Topology>(&self, from: Cell, to: Cell) -> Turn {
        let (a, b) = (T::centre(from, self.width), T::centre(to, self.width));
        atan2(a.y - b.y, b.x - a.x)
    }

    pub(crate) fn steps<T: Topology>(&self, a: Cell, b: Cell) -> u32 {
        T::steps(a, b, self.width)
    }

    pub(crate) fn line<T: Topology>(&self, from: Cell, to: Cell, out: &mut Vec<Cell>) {
        T::line(from, to, self.width, out);
    }

    /// The frame cell exactly `length` steps from `from` whose direction is nearest `facing`,
    /// lowest cell on a tie. The frame must pad at least `length` cells.
    pub(crate) fn ahead<T: Topology>(&self, from: Cell, facing: Turn, length: u32) -> Cell {
        let (x, y) = ((from.0 % self.width) as i64, (from.0 / self.width) as i64);
        let reach = length as i64 + 1;
        let mut best: Option<(Turn, Cell)> = None;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (cx, cy) = (x + dx, y + dy);
                if cx < 0 || cy < 0 || cx >= self.width as i64 || cy >= self.height as i64 {
                    continue;
                }
                let cell = Cell(cy as u32 * self.width + cx as u32);
                if self.steps::<T>(from, cell) != length {
                    continue;
                }
                let off = crate::shape::angle_between(self.direction::<T>(from, cell), facing);
                if best.is_none_or(|current| (off, cell) < current) {
                    best = Some((off, cell));
                }
            }
        }
        best.map(|(_, cell)| cell).unwrap_or(from)
    }

    /// The cells of the line from `from` toward the cell `length` steps ahead, as map cells,
    /// starting after `from` and ending before the line leaves the map.
    pub(crate) fn path_ahead<T: Topology>(
        &self,
        from: Cell,
        facing: Turn,
        length: u32,
    ) -> Vec<Cell> {
        if length == 0 {
            return Vec::new();
        }
        let start = self.to_frame(from);
        let end = self.ahead::<T>(start, facing, length);
        let mut cells = Vec::new();
        self.line::<T>(start, end, &mut cells);
        cells
            .into_iter()
            .skip(1)
            .map(|cell| self.to_map(cell))
            .take_while(Option::is_some)
            .flatten()
            .collect()
    }

    /// Which of `from`'s neighbours `to` is, by its place in the topology's fixed order.
    pub(crate) fn direction_index<T: Topology>(&self, from: Cell, to: Cell) -> Option<usize> {
        let mut neighbours = Vec::new();
        self.neighbours::<T>(from, &mut neighbours);
        neighbours.iter().position(|cell| *cell == to)
    }

    /// The neighbour of `cell` at a place in the topology's fixed order.
    pub(crate) fn neighbour_at<T: Topology>(&self, cell: Cell, index: usize) -> Option<Cell> {
        let mut neighbours = Vec::new();
        self.neighbours::<T>(cell, &mut neighbours);
        neighbours.get(index).copied()
    }
}

/// Whether a step from one map cell to the next cuts a wall corner: a diagonal step with a wall on
/// either side, the same rule `GridMap::can_step` uses for bodies.
pub(crate) fn cuts_corner<T: Topology>(map: &GridMap<T>, from: Cell, to: Cell) -> bool {
    if map.distance(from, to) <= 10 {
        return false;
    }
    let ((from_x, from_y), (to_x, to_y)) = (map.coordinates(from), map.coordinates(to));
    !map.is_passable(map.index(from_x, to_y)) || !map.is_passable(map.index(to_x, from_y))
}
