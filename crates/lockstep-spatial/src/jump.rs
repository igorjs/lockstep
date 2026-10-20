// SPDX-License-Identifier: Apache-2.0
use crate::cell::Cell;
use crate::map::GridMap;
use crate::occupancy::Occupancy;
use crate::pathfinder::{PathOptions, PathResult, Pathfinder};
use crate::topology::Square8;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const NONE: u32 = u32::MAX;

/// Exact jump point search for uniform open ground on `Square8`.
///
/// It returns a path of the same cost as `Pathfinder` (decision 0075) while expanding far fewer
/// cells: instead of every neighbour, it jumps along straight and diagonal lines to the next cell
/// where a choice of route could matter. It needs a map where every passable cell has the same
/// terrain cost and the same elevation, which is what `new` checks; any other map gets `None`
/// and should use `Pathfinder`.
///
/// The cells of the returned path may differ from `Pathfinder`'s when several paths cost the same
/// (decision 0075 states the tie-break), so a fixture made with one search is not valid for the
/// other.
pub struct JumpPathfinder {
    /// The width of a padded row: one blocked column on each side.
    stride: i64,
    /// Passable cells in a grid padded with a blocked border, so no test needs a bounds check.
    open: Vec<u8>,
    /// The same cells as bits, row by row and column by column, for scanning 64 at a time.
    rows: Vec<u64>,
    columns: Vec<u64>,
    row_words: usize,
    column_words: usize,
    revision: u64,
    generation: u32,
    stamp: Vec<u32>,
    closed: Vec<u32>,
    cost: Vec<u32>,
    parent: Vec<u32>,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    fallback: Option<Pathfinder>,
    expansions: u32,
}

impl JumpPathfinder {
    /// A jump pathfinder for this map, or `None` when the ground is not uniform. Rebuild it after
    /// the map changes: `find` panics on a map of another revision.
    pub fn new(map: &GridMap<Square8>) -> Option<Self> {
        let cells = map.cell_count();
        let mut reference: Option<(u8, u8)> = None;
        for index in 0..cells as u32 {
            let cell = Cell(index);
            if !map.is_passable(cell) {
                continue;
            }
            let ground = (map.cost(cell).max(1), map.elevation(cell));
            match reference {
                None => reference = Some(ground),
                Some(known) if known != ground => return None,
                Some(_) => {}
            }
        }
        if map.width() > 1 << 20 || map.height() > 1 << 20 {
            return None;
        }
        let (width, height) = (map.width() as i64, map.height() as i64);
        let stride = width + 2;
        let mut open = vec![0u8; (stride * (height + 2)) as usize];
        for y in 0..height {
            for x in 0..width {
                let cell = map.index(x as u32, y as u32);
                open[((y + 1) * stride + x + 1) as usize] = map.is_passable(cell) as u8;
            }
        }
        let row_words = (stride as usize).div_ceil(64);
        let column_words = ((height + 2) as usize).div_ceil(64);
        let mut rows = vec![0u64; (height + 2) as usize * row_words];
        let mut columns = vec![0u64; stride as usize * column_words];
        for y in 0..height + 2 {
            for x in 0..stride {
                if open[(y * stride + x) as usize] != 0 {
                    rows[y as usize * row_words + x as usize / 64] |= 1 << (x % 64);
                    columns[x as usize * column_words + y as usize / 64] |= 1 << (y % 64);
                }
            }
        }
        let padded = open.len();
        if padded as u64 >= u32::MAX as u64 {
            return None;
        }
        Some(Self {
            stride,
            open,
            rows,
            columns,
            row_words,
            column_words,
            revision: map.revision(),
            generation: 0,
            stamp: vec![0; padded],
            closed: vec![0; padded],
            cost: vec![0; padded],
            parent: vec![NONE; padded],
            heap: BinaryHeap::new(),
            fallback: None,
            expansions: 0,
        })
    }

    /// How many jump points the last search expanded.
    pub fn last_expansions(&self) -> u32 {
        self.expansions
    }

    fn pad(&self, map: &GridMap<Square8>, cell: Cell) -> i64 {
        let (x, y) = map.coordinates(cell);
        (y as i64 + 1) * self.stride + x as i64 + 1
    }

    fn unpad(&self, map: &GridMap<Square8>, padded: i64) -> Cell {
        let (x, y) = (padded % self.stride - 1, padded / self.stride - 1);
        map.index(x as u32, y as u32)
    }

    fn is_open(&self, padded: i64) -> bool {
        self.open[padded as usize] != 0
    }

    /// Octile distance in tenths between two padded positions.
    fn distance(&self, a: i64, b: i64) -> u32 {
        let (ax, ay) = (a % self.stride, a / self.stride);
        let (bx, by) = (b % self.stride, b / self.stride);
        let (dx, dy) = (
            (ax - bx).unsigned_abs() as u32,
            (ay - by).unsigned_abs() as u32,
        );
        10 * dx.max(dy) + 4 * dx.min(dy)
    }

    /// Whether a diagonal step of `(column, row)` offset `delta` from `at` is allowed: the
    /// target and both corner cells are open.
    fn diagonal_open(&self, at: i64, column: i64, row: i64) -> bool {
        let across = row * self.stride;
        self.is_open(at + column + across) && self.is_open(at + column) && self.is_open(at + across)
    }

    /// Runs along a straight line to the next jump point or the goal, a word of 64 cells at a
    /// time. Returns the padded index, or -1 when the run ends at a wall first.
    fn jump_straight(&self, from: i64, column: i64, row: i64, goal: i64) -> i64 {
        let (x, y) = (from % self.stride, from / self.stride);
        let (goal_x, goal_y) = (goal % self.stride, goal / self.stride);
        let (lines, words, along, across, goal_along, goal_on_line, forward) = if row == 0 {
            (
                &self.rows,
                self.row_words,
                x,
                y,
                goal_x,
                goal_y == y,
                column > 0,
            )
        } else {
            (
                &self.columns,
                self.column_words,
                y,
                x,
                goal_y,
                goal_x == x,
                row > 0,
            )
        };
        let line = &lines[across as usize * words..][..words];
        let above = &lines[(across as usize - 1) * words..][..words];
        let below = &lines[(across as usize + 1) * words..][..words];
        let goal_position = goal_on_line.then_some(goal_along as usize);
        match scan(line, above, below, along as usize, goal_position, forward) {
            None => -1,
            Some(position) if row == 0 => y * self.stride + position as i64,
            Some(position) => position as i64 * self.stride + x,
        }
    }

    fn jump_diagonal(&self, from: i64, column: i64, row: i64, goal: i64) -> i64 {
        let step = column + row * self.stride;
        let mut at = from;
        loop {
            if !self.diagonal_open(at, column, row) {
                return -1;
            }
            at += step;
            if at == goal {
                return at;
            }
            if self.jump_straight(at, column, 0, goal) >= 0
                || self.jump_straight(at, 0, row, goal) >= 0
            {
                return at;
            }
        }
    }

    fn jump(&self, from: i64, column: i64, row: i64, goal: i64) -> i64 {
        if column != 0 && row != 0 {
            self.jump_diagonal(from, column, row, goal)
        } else {
            self.jump_straight(from, column, row, goal)
        }
    }

    /// Finds a path of the cheapest cost from `from` to `to`, with the same answer for the cost
    /// as `Pathfinder::find`. A search with occupants as walls, or from a wall, is handed to the
    /// plain search. `maximum_expansions` counts jump points here, not cells.
    pub fn find(
        &mut self,
        map: &GridMap<Square8>,
        occupancy: &Occupancy,
        from: Cell,
        to: Cell,
        options: PathOptions,
        out: &mut Vec<Cell>,
    ) -> PathResult {
        assert_eq!(
            self.revision,
            map.revision(),
            "the map changed since this jump pathfinder was made"
        );
        if options.treat_occupants_as_walls || !map.is_passable(from) {
            let plain = self.fallback.get_or_insert_with(|| Pathfinder::new(map));
            let result = plain.find(map, occupancy, from, to, options, out);
            self.expansions = plain.last_expansions();
            return result;
        }
        out.clear();
        self.expansions = 0;
        if from == to {
            return PathResult::Found { cost: 0 };
        }
        if !map.is_passable(to) {
            return PathResult::Unreachable;
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        let (start, goal) = (self.pad(map, from), self.pad(map, to));
        self.heap.clear();
        self.stamp[start as usize] = generation;
        self.cost[start as usize] = 0;
        self.parent[start as usize] = NONE;
        self.heap
            .push(Reverse((self.distance(start, goal), start as u32)));
        let mut expansions = 0;
        while let Some(Reverse((_, current))) = self.heap.pop() {
            let at = current as i64;
            if at == goal {
                self.reconstruct(map, start, goal, out);
                return PathResult::Found {
                    cost: self.cost[goal as usize],
                };
            }
            if self.closed[at as usize] == generation {
                continue;
            }
            self.closed[at as usize] = generation;
            expansions += 1;
            self.expansions = expansions;
            if expansions > options.maximum_expansions {
                return PathResult::Aborted;
            }
            let base = self.cost[at as usize];
            let previous = self.parent[at as usize];
            let (directions, count) = self.directions(at, previous);
            for (column, row) in directions.into_iter().take(count) {
                let target = self.jump(at, column, row, goal);
                if target < 0 || self.closed[target as usize] == generation {
                    continue;
                }
                let tentative = base.saturating_add(self.distance(at, target));
                let known = if self.stamp[target as usize] == generation {
                    self.cost[target as usize]
                } else {
                    u32::MAX
                };
                if tentative < known {
                    self.stamp[target as usize] = generation;
                    self.cost[target as usize] = tentative;
                    self.parent[target as usize] = current;
                    let estimate = tentative.saturating_add(self.distance(target, goal));
                    self.heap.push(Reverse((estimate, target as u32)));
                }
            }
        }
        PathResult::Unreachable
    }

    /// The directions worth a jump from `at`, given the jump point it was reached from.
    fn directions(&self, at: i64, previous: u32) -> ([(i64, i64); 8], usize) {
        const ALL: [(i64, i64); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        if previous == NONE {
            return (ALL, 8);
        }
        let from = previous as i64;
        let (ax, ay) = (at % self.stride, at / self.stride);
        let (px, py) = (from % self.stride, from / self.stride);
        let (column, row) = ((ax - px).signum(), (ay - py).signum());
        let mut found = [(0i64, 0i64); 8];
        let mut count = 0;
        let mut push = |direction: (i64, i64)| {
            found[count] = direction;
            count += 1;
        };
        if column != 0 && row != 0 {
            push((column, 0));
            push((0, row));
            push((column, row));
        } else {
            push((column, row));
            let side = if column != 0 { self.stride } else { 1 };
            let along = column + row * self.stride;
            for sign in [1i64, -1] {
                let offset = side * sign;
                if self.is_open(at + offset) && !self.is_open(at - along + offset) {
                    let (side_column, side_row) = if column != 0 { (0, sign) } else { (sign, 0) };
                    push((side_column, side_row));
                    push((column + side_column, row + side_row));
                }
            }
        }
        (found, count)
    }

    fn reconstruct(&self, map: &GridMap<Square8>, start: i64, goal: i64, out: &mut Vec<Cell>) {
        let mut at = goal;
        while at != start {
            let previous = self.parent[at as usize] as i64;
            let (ax, ay) = (at % self.stride, at / self.stride);
            let (px, py) = (previous % self.stride, previous / self.stride);
            let (column, row) = ((px - ax).signum(), (py - ay).signum());
            let step = column + row * self.stride;
            let mut cell = at;
            while cell != previous {
                out.push(self.unpad(map, cell));
                cell += step;
            }
            at = previous;
        }
        out.reverse();
    }
}

/// Finds the first jump point or goal along a line of cells stored as bits, going up or down in
/// position from `start` (not counting `start`). A cell is a jump point when a cell beside it is
/// open and the one beside the previous cell is not, on either side. A wall ends the run.
fn scan(
    line: &[u64],
    above: &[u64],
    below: &[u64],
    start: usize,
    goal: Option<usize>,
    forward: bool,
) -> Option<usize> {
    let words = line.len();
    let mut word = start / 64;
    loop {
        let bit = start % 64;
        let in_range = if word != start / 64 {
            u64::MAX
        } else if forward {
            // Bits above `start`.
            (!0u64).checked_shl(bit as u32 + 1).unwrap_or(0)
        } else {
            (1u64 << bit) - 1
        };
        let forced = if forward {
            let before = |side: &[u64]| if word > 0 { side[word - 1] >> 63 } else { 0 };
            (above[word] & !((above[word] << 1) | before(above)))
                | (below[word] & !((below[word] << 1) | before(below)))
        } else {
            let after = |side: &[u64]| {
                if word + 1 < words {
                    side[word + 1] << 63
                } else {
                    0
                }
            };
            (above[word] & !((above[word] >> 1) | after(above)))
                | (below[word] & !((below[word] >> 1) | after(below)))
        };
        let mut events = forced;
        if let Some(position) = goal {
            if position / 64 == word {
                events |= 1 << (position % 64);
            }
        }
        events &= line[word] & in_range;
        let walls = !line[word] & in_range;
        if forward {
            let event = events.trailing_zeros();
            if event < walls.trailing_zeros() {
                return Some(word * 64 + event as usize);
            }
            if walls != 0 {
                return None;
            }
            word += 1;
            if word == words {
                return None;
            }
        } else {
            let event = events.leading_zeros();
            if events != 0 && event <= walls.leading_zeros() {
                return Some(word * 64 + 63 - event as usize);
            }
            if walls != 0 {
                return None;
            }
            if word == 0 {
                return None;
            }
            word -= 1;
        }
    }
}
