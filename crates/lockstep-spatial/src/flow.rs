// SPDX-License-Identifier: Apache-2.0
use crate::cell::Cell;
use crate::map::GridMap;
use crate::topology::Topology;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const UNREACHED: u32 = u32::MAX;

/// A distance field from a set of targets, with the next cell to step to from every cell.
///
/// One pass outward from the targets, bounded by distance, stores for each cell the cheapest cost to
/// the nearest target and the neighbour to step to. A hundred bodies then move by table lookup,
/// with no pathfinding. Ties keep the first route found in the fixed neighbour order, so every
/// platform builds the same field.
#[derive(Clone, Debug)]
pub struct FlowField {
    distance: Vec<u32>,
    next: Vec<Cell>,
}

impl FlowField {
    /// Builds the field. Costs are in tenths, and cells farther than `maximum_distance` from every
    /// target are left unreached. A body can move from a cell to its next cell only where
    /// `GridMap::can_step` allows it, and the field respects that.
    pub fn build<T: Topology>(map: &GridMap<T>, targets: &[Cell], maximum_distance: u32) -> Self {
        let cells = map.cell_count();
        let mut field = Self {
            distance: vec![UNREACHED; cells],
            next: (0..cells as u32).map(Cell).collect(),
        };
        let mut open: BinaryHeap<Reverse<(u32, Cell)>> = BinaryHeap::new();
        for target in targets {
            if field.distance[target.0 as usize] != 0 {
                field.distance[target.0 as usize] = 0;
                open.push(Reverse((0, *target)));
            }
        }
        let mut neighbours = Vec::new();
        while let Some(Reverse((distance, cell))) = open.pop() {
            if distance > field.distance[cell.0 as usize] {
                continue;
            }
            map.neighbours(cell, &mut neighbours);
            for before in neighbours.iter().copied() {
                // The field runs backwards: a body at `before` steps onto `cell`. A wall is not a
                // place a body can stand, so it is never a starting point either.
                if !map.is_passable(before) || !map.can_step(before, cell) {
                    continue;
                }
                let candidate = distance.saturating_add(map.step_cost(before, cell));
                if candidate > maximum_distance {
                    continue;
                }
                if candidate < field.distance[before.0 as usize] {
                    field.distance[before.0 as usize] = candidate;
                    field.next[before.0 as usize] = cell;
                    open.push(Reverse((candidate, before)));
                }
            }
        }
        field
    }

    /// The cost from a cell to the nearest target, in tenths, or `None` when it is out of reach.
    pub fn distance(&self, cell: Cell) -> Option<u32> {
        match self.distance[cell.0 as usize] {
            UNREACHED => None,
            distance => Some(distance),
        }
    }

    /// The cell to step to from `cell`, or `None` at a target or out of reach.
    pub fn step_from(&self, cell: Cell) -> Option<Cell> {
        match self.distance[cell.0 as usize] {
            UNREACHED | 0 => None,
            _ => Some(self.next[cell.0 as usize]),
        }
    }
}
