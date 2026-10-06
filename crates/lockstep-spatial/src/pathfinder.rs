use crate::cell::Cell;
use crate::map::GridMap;
use crate::occupancy::Occupancy;
use crate::topology::Topology;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// What to do while searching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathOptions {
    /// Cells held by a body are walls, except the destination itself.
    pub treat_occupants_as_walls: bool,
    /// Give up after expanding this many cells.
    pub maximum_expansions: u32,
}

impl Default for PathOptions {
    fn default() -> Self {
        Self {
            treat_occupants_as_walls: false,
            maximum_expansions: u32::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathResult {
    /// A path was found. The cost is in tenths.
    Found { cost: u32 },
    /// The search finished and no path exists.
    Unreachable,
    /// The search hit `maximum_expansions` first.
    Aborted,
}

/// A* search with buffers reused across calls and reset in constant time by a generation stamp.
///
/// The heap key is `(f_cost, cell)`: the cell index breaks ties, which, with neighbours in a fixed
/// order, makes equal-cost paths identical on every platform.
pub struct Pathfinder {
    generation: u32,
    /// The generation in which a cell's cost and parent were last written.
    stamp: Vec<u32>,
    /// The generation in which a cell was expanded.
    closed: Vec<u32>,
    cost: Vec<u32>,
    parent: Vec<Cell>,
    open: BinaryHeap<Reverse<(u32, Cell)>>,
    neighbours: Vec<Cell>,
    expansions: u32,
}

impl Pathfinder {
    /// Buffers sized for this map. Use the pathfinder only with maps of the same size: a new map of
    /// another size needs a new pathfinder.
    pub fn new<T: Topology>(map: &GridMap<T>) -> Self {
        let cells = map.cell_count();
        Self {
            generation: 0,
            stamp: vec![0; cells],
            closed: vec![0; cells],
            cost: vec![0; cells],
            parent: vec![Cell(0); cells],
            open: BinaryHeap::new(),
            neighbours: Vec::new(),
            expansions: 0,
        }
    }

    /// How many cells the last search expanded: a measure of its work, for tuning and tests.
    pub fn last_expansions(&self) -> u32 {
        self.expansions
    }

    /// Sets the generation counter, so a test can make it wrap on the next search.
    #[doc(hidden)]
    pub fn force_generation(&mut self, generation: u32) {
        // Clear the stamps too, so a counter moved backwards cannot make old marks look current.
        self.stamp.fill(0);
        self.closed.fill(0);
        self.generation = generation;
    }

    fn set(&mut self, cell: Cell, cost: u32, parent: Cell) {
        let index = cell.0 as usize;
        self.stamp[index] = self.generation;
        self.cost[index] = cost;
        self.parent[index] = parent;
    }

    /// Finds the cheapest path from `from` to `to`. On success `out` holds the cells to walk,
    /// in order, excluding `from` and including `to`. `out` is cleared first.
    pub fn find<T: Topology>(
        &mut self,
        map: &GridMap<T>,
        occupancy: &Occupancy,
        from: Cell,
        to: Cell,
        options: PathOptions,
        out: &mut Vec<Cell>,
    ) -> PathResult {
        assert_eq!(
            self.stamp.len(),
            map.cell_count(),
            "this pathfinder was made for a map of another size"
        );
        out.clear();
        self.expansions = 0;
        if from == to {
            return PathResult::Found { cost: 0 };
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        self.open.clear();
        self.set(from, 0, from);
        self.open.push(Reverse((map.straight_cost(from, to), from)));
        let mut expansions = 0;
        while let Some(Reverse((_, current))) = self.open.pop() {
            if current == to {
                self.reconstruct(from, to, out);
                return PathResult::Found {
                    cost: self.cost[to.0 as usize],
                };
            }
            if self.closed[current.0 as usize] == generation {
                continue;
            }
            self.closed[current.0 as usize] = generation;
            expansions += 1;
            self.expansions = expansions;
            if expansions > options.maximum_expansions {
                return PathResult::Aborted;
            }
            let current_cost = self.cost[current.0 as usize];
            map.neighbours(current, &mut self.neighbours);
            // Take the buffer out while the loop writes to the other fields, then give it back.
            let neighbours = std::mem::take(&mut self.neighbours);
            for next in neighbours.iter().copied() {
                if !map.can_step(current, next) {
                    continue;
                }
                if options.treat_occupants_as_walls && next != to && occupancy.at(next).is_some() {
                    continue;
                }
                // Saturating, so a path long enough to pass four billion tenths stays the most
                // expensive instead of wrapping to a cheap one.
                let tentative = current_cost.saturating_add(map.step_cost(current, next));
                let known = if self.stamp[next.0 as usize] == generation {
                    self.cost[next.0 as usize]
                } else {
                    u32::MAX
                };
                if tentative < known {
                    self.set(next, tentative, current);
                    let estimate = tentative.saturating_add(map.straight_cost(next, to));
                    self.open.push(Reverse((estimate, next)));
                }
            }
            self.neighbours = neighbours;
        }
        PathResult::Unreachable
    }

    fn reconstruct(&self, from: Cell, to: Cell, out: &mut Vec<Cell>) {
        let mut cell = to;
        while cell != from {
            out.push(cell);
            cell = self.parent[cell.0 as usize];
        }
        out.reverse();
    }
}
