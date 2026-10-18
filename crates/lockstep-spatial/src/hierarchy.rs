// SPDX-License-Identifier: Apache-2.0
//! Hierarchical pathfinding over square chunks.
//!
//! The map is cut into chunks. Where two chunks touch, each run of open border cells gets an
//! entrance (one cell on each side). The entrances are the nodes of a small graph: an edge joins
//! the two cells of an entrance, and another joins every two entrances of one chunk, at the cost
//! of the cheapest route that stays inside the chunk. A search runs on that graph, and only the
//! chunks on the route are searched cell by cell.
//!
//! The price is that a route is not always the cheapest: it is the cheapest one that crosses chunk
//! borders at entrances. The cost reported is the cost of the path returned. Everything is integer
//! arithmetic over cells and edges in fixed orders, so the path is the same on every platform.

use crate::cell::Cell;
use crate::map::GridMap;
use crate::occupancy::Occupancy;
use crate::pathfinder::{PathOptions, PathResult, Pathfinder};
use crate::topology::{Step, Topology};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const FAR: u32 = u32::MAX;

/// How much the search over entrances overweights the straight-line estimate, in tenths: 12 means
/// 1.2 times. An estimate above the truth makes the search commit to a direction sooner and
/// expand far fewer entrances, at the price of a slightly dearer path (decision 0047).
pub const DEFAULT_GREED: u32 = 12;

/// A border run this short gets one entrance in its middle; a longer run gets one at each end.
const SHORT_RUN: u32 = 5;

#[derive(Clone, Copy)]
struct Region {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    to: u32,
    cost: u32,
    /// Where the cells of the route inside the chunk start in `paths`, and how many there are.
    /// An entrance's own crossing has none.
    path: u32,
    length: u32,
}

fn key(estimate: u32, index: u32) -> Reverse<u64> {
    Reverse(((estimate as u64) << 32) | index as u64)
}

/// A search confined to one region, with buffers sized for the largest chunk.
struct Local {
    generation: u32,
    stamp: Vec<u32>,
    closed: Vec<u32>,
    distance: Vec<u32>,
    link: Vec<u32>,
    open: BinaryHeap<Reverse<u64>>,
    steps: Vec<Step>,
    /// The cells expanded by the last run, in order.
    reached: Vec<Cell>,
    expansions: u32,
}

impl Local {
    fn new(side: u32) -> Self {
        let cells = (side * side) as usize;
        Self {
            generation: 0,
            stamp: vec![0; cells],
            closed: vec![0; cells],
            distance: vec![0; cells],
            link: vec![0; cells],
            open: BinaryHeap::new(),
            steps: Vec::new(),
            reached: Vec::new(),
            expansions: 0,
        }
    }

    fn slot<T: Topology>(map: &GridMap<T>, region: Region, cell: Cell) -> Option<usize> {
        let (x, y) = map.coordinates(cell);
        let across = x >= region.x + region.width;
        let below = y >= region.y + region.height;
        if x < region.x || y < region.y || across || below {
            return None;
        }
        Some(((y - region.y) * region.width + (x - region.x)) as usize)
    }

    /// The slot of the neighbour a step leads to, from the slot of the cell it leaves, or `None`
    /// when the neighbour is outside the region.
    fn neighbour_slot(region: Region, slot: usize, step: &Step) -> Option<usize> {
        let column = (slot as u32 % region.width) as i32 + step.column_change as i32;
        let row = (slot as u32 / region.width) as i32 + step.row_change as i32;
        if column < 0 || row < 0 || column >= region.width as i32 || row >= region.height as i32 {
            return None;
        }
        Some((row as u32 * region.width + column as u32) as usize)
    }

    /// Searches inside `region` from `origin`. Forward, it finds the cheapest cost from the origin
    /// to each cell, stopping at `goal` if given. Backward, it finds the cheapest cost from each
    /// cell to the origin. `link` leads back to the origin either way.
    fn run<T: Topology>(
        &mut self,
        map: &GridMap<T>,
        region: Region,
        origin: Cell,
        goal: Option<Cell>,
        backward: bool,
    ) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        self.expansions = 0;
        self.reached.clear();
        self.open.clear();
        let start = Self::slot(map, region, origin).expect("the origin is inside the region");
        self.stamp[start] = generation;
        self.distance[start] = 0;
        self.link[start] = origin.0;
        let heuristic = |cell: Cell| goal.map_or(0, |goal| map.straight_cost(cell, goal));
        self.open.push(key(heuristic(origin), origin.0));
        let mut steps = std::mem::take(&mut self.steps);
        while let Some(Reverse(packed)) = self.open.pop() {
            let cell = Cell(packed as u32);
            let slot = Self::slot(map, region, cell).expect("queued cells are inside the region");
            if self.closed[slot] == generation {
                continue;
            }
            self.closed[slot] = generation;
            self.expansions += 1;
            self.reached.push(cell);
            if goal == Some(cell) {
                break;
            }
            let here = self.distance[slot];
            map.steps_from(cell, &mut steps);
            for step in steps.iter() {
                let next = step.to;
                let Some(next_slot) = Self::neighbour_slot(region, slot, step) else {
                    continue;
                };
                let (allowed, terrain) = if backward {
                    (map.can_take_into(cell, step), map.cost(cell))
                } else {
                    (map.can_take(cell, step), map.cost(next))
                };
                if !allowed {
                    continue;
                }
                let tentative = here.saturating_add(step.cost * terrain.max(1) as u32);
                let known = if self.stamp[next_slot] == generation {
                    self.distance[next_slot]
                } else {
                    FAR
                };
                if tentative < known {
                    self.stamp[next_slot] = generation;
                    self.distance[next_slot] = tentative;
                    self.link[next_slot] = cell.0;
                    let estimate = tentative.saturating_add(heuristic(next));
                    self.open.push(key(estimate, next.0));
                }
            }
        }
        self.steps = steps;
    }

    /// Whether any cell the last run reached has a step across the region's edge: forward, a step
    /// out of the region; backward, a step into the cell from outside.
    fn leaves<T: Topology>(&mut self, map: &GridMap<T>, region: Region, backward: bool) -> bool {
        let mut steps = std::mem::take(&mut self.steps);
        let mut leaves = false;
        'cells: for cell in self.reached.iter().copied() {
            let slot = Self::slot(map, region, cell).expect("reached cells are inside the region");
            map.steps_from(cell, &mut steps);
            for step in steps.iter() {
                if Self::neighbour_slot(region, slot, step).is_some() {
                    continue;
                }
                let allowed = if backward {
                    map.can_take_into(cell, step)
                } else {
                    map.can_take(cell, step)
                };
                if allowed {
                    leaves = true;
                    break 'cells;
                }
            }
        }
        self.steps = steps;
        leaves
    }

    /// The cost found for a cell by the last run, or `FAR`.
    fn cost_of<T: Topology>(&self, map: &GridMap<T>, region: Region, cell: Cell) -> u32 {
        match Self::slot(map, region, cell) {
            Some(slot) if self.stamp[slot] == self.generation => self.distance[slot],
            _ => FAR,
        }
    }

    fn link_of<T: Topology>(&self, map: &GridMap<T>, region: Region, cell: Cell) -> Cell {
        Cell(self.link[Self::slot(map, region, cell).expect("inside the region")])
    }
}

/// What a hierarchical search concluded.
enum Outcome {
    Found(u32),
    /// Certainly no path: an end is walled into its chunk.
    Unreachable,
    /// No route found; the exact search decides.
    Unknown,
}

/// The entrance graph of a map. Build it once, search with a [`HierarchicalPathfinder`], and
/// build it again after the map changes: a search given a hierarchy of an older map falls back
/// to the exact search.
#[derive(Clone, Debug)]
pub struct Hierarchy {
    side: u32,
    columns: u32,
    revision: u64,
    cells: usize,
    /// Entrance cells, ordered by chunk and then by cell, so a chunk's entrances are a range.
    nodes: Vec<Cell>,
    chunk_start: Vec<u32>,
    edge_start: Vec<u32>,
    edges: Vec<Edge>,
    /// The routes of the edges inside chunks, each excluding its first cell and including its last.
    paths: Vec<Cell>,
}

impl Hierarchy {
    /// Builds the graph with chunks `side` cells wide. Smaller chunks mean cheaper searches inside
    /// a chunk and a bigger graph. Panics when `side` is zero.
    pub fn build<T: Topology>(map: &GridMap<T>, side: u32) -> Self {
        assert!(side > 0, "a chunk needs at least one cell");
        let columns = map.width().div_ceil(side);
        let rows = map.height().div_ceil(side);
        let chunks = (columns * rows) as usize;
        let chunk_id = |cell: Cell| {
            let (x, y) = map.coordinates(cell);
            (y / side) * columns + x / side
        };

        // Pairs of cells across a chunk border that a body can step between both ways. A border
        // that can be crossed one way only (a drop) is left to the exact search.
        let mut crossings: Vec<(Cell, Cell)> = Vec::new();
        let open = |a: Cell, b: Cell| map.can_step(a, b) && map.can_step(b, a);
        for border in 1..columns {
            let x = border * side - 1;
            Self::entrances(
                map.height(),
                side,
                |y| open(map.index(x, y), map.index(x + 1, y)),
                |y| crossings.push((map.index(x, y), map.index(x + 1, y))),
            );
        }
        for border in 1..rows {
            let y = border * side - 1;
            Self::entrances(
                map.width(),
                side,
                |x| open(map.index(x, y), map.index(x, y + 1)),
                |x| crossings.push((map.index(x, y), map.index(x, y + 1))),
            );
        }

        let mut ordered: Vec<(u32, u32)> = crossings
            .iter()
            .flat_map(|(a, b)| [a, b])
            .map(|cell| (chunk_id(*cell), cell.0))
            .collect();
        ordered.sort_unstable();
        ordered.dedup();
        let nodes: Vec<Cell> = ordered.iter().map(|(_, cell)| Cell(*cell)).collect();
        let mut node_of = vec![FAR; map.cell_count()];
        for (index, cell) in nodes.iter().enumerate() {
            node_of[cell.0 as usize] = index as u32;
        }
        let mut chunk_start = vec![0u32; chunks + 1];
        for (chunk, _) in &ordered {
            chunk_start[*chunk as usize + 1] += 1;
        }
        for chunk in 0..chunks {
            chunk_start[chunk + 1] += chunk_start[chunk];
        }

        let mut lists: Vec<Vec<Edge>> = vec![Vec::new(); nodes.len()];
        for (a, b) in &crossings {
            let (from, to) = (node_of[a.0 as usize], node_of[b.0 as usize]);
            let forward = map.step_cost(*a, *b);
            let backward = map.step_cost(*b, *a);
            lists[from as usize].push(Edge {
                to,
                cost: forward,
                path: 0,
                length: 0,
            });
            lists[to as usize].push(Edge {
                to: from,
                cost: backward,
                path: 0,
                length: 0,
            });
        }
        let mut paths: Vec<Cell> = Vec::new();
        let mut local = Local::new(side);
        for chunk in 0..chunks as u32 {
            let region = Self::region_of(map, side, columns, chunk);
            let range = chunk_start[chunk as usize]..chunk_start[chunk as usize + 1];
            let count = range.len();
            let mut cost = vec![FAR; count * count];
            let mut routes: Vec<Vec<Cell>> = vec![Vec::new(); count * count];
            for from in 0..count {
                let origin = nodes[range.start as usize + from];
                local.run(map, region, origin, None, false);
                for to in 0..count {
                    let target = nodes[range.start as usize + to];
                    cost[from * count + to] = local.cost_of(map, region, target);
                    if to != from && cost[from * count + to] != FAR {
                        Self::walk(
                            &local,
                            map,
                            region,
                            origin,
                            target,
                            &mut routes[from * count + to],
                        );
                    }
                }
            }
            // An edge is kept unless a route through a third entrance costs no more, so the graph
            // stays small and every distance stays the same.
            for from in 0..count {
                for to in 0..count {
                    let direct = cost[from * count + to];
                    if to == from || direct == FAR {
                        continue;
                    }
                    let detour = (0..count).any(|via| {
                        let (first, second) = (cost[from * count + via], cost[via * count + to]);
                        via != from
                            && via != to
                            && first != FAR
                            && second != FAR
                            && first.saturating_add(second) <= direct
                    });
                    if detour {
                        continue;
                    }
                    let route = &routes[from * count + to];
                    lists[range.start as usize + from].push(Edge {
                        to: range.start + to as u32,
                        cost: direct,
                        path: paths.len() as u32,
                        length: route.len() as u32,
                    });
                    paths.extend_from_slice(route);
                }
            }
        }
        let mut edge_start = Vec::with_capacity(nodes.len() + 1);
        let mut edges = Vec::new();
        for list in lists {
            edge_start.push(edges.len() as u32);
            edges.extend(list);
        }
        edge_start.push(edges.len() as u32);
        Self {
            side,
            columns,
            revision: map.revision(),
            cells: map.cell_count(),
            nodes,
            chunk_start,
            edge_start,
            edges,
            paths,
        }
    }

    /// Calls `emit` with the position of each entrance along one border: the middle of a short
    /// run of open positions, or both ends of a long one. A run also ends where a chunk ends.
    fn entrances(length: u32, side: u32, open: impl Fn(u32) -> bool, mut emit: impl FnMut(u32)) {
        let mut start = None;
        for position in 0..=length {
            let inside = position < length && open(position);
            let broken = position % side == 0;
            if let Some(first) = start {
                if !inside || broken {
                    let last = position - 1;
                    if last - first < SHORT_RUN {
                        emit(first + (last - first) / 2);
                    } else {
                        emit(first);
                        emit(last);
                    }
                    start = None;
                }
            }
            if inside && start.is_none() {
                start = Some(position);
            }
        }
    }

    /// The cells from `origin` (excluded) to `target` (included), following the last run's links.
    fn walk<T: Topology>(
        local: &Local,
        map: &GridMap<T>,
        region: Region,
        origin: Cell,
        target: Cell,
        out: &mut Vec<Cell>,
    ) {
        let begin = out.len();
        let mut cell = target;
        while cell != origin {
            out.push(cell);
            cell = local.link_of(map, region, cell);
        }
        out[begin..].reverse();
    }

    fn region_of<T: Topology>(map: &GridMap<T>, side: u32, columns: u32, chunk: u32) -> Region {
        let (x, y) = ((chunk % columns) * side, (chunk / columns) * side);
        Region {
            x,
            y,
            width: side.min(map.width() - x),
            height: side.min(map.height() - y),
        }
    }

    /// How many entrance cells the graph has.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The width of a chunk in cells.
    pub fn side(&self) -> u32 {
        self.side
    }

    /// Whether the map is unchanged since the graph was built.
    pub fn is_current<T: Topology>(&self, map: &GridMap<T>) -> bool {
        self.revision == map.revision() && self.cells == map.cell_count()
    }

    fn chunk_of<T: Topology>(&self, map: &GridMap<T>, cell: Cell) -> u32 {
        let (x, y) = map.coordinates(cell);
        (y / self.side) * self.columns + x / self.side
    }

    fn nodes_in(&self, chunk: u32) -> std::ops::Range<u32> {
        self.chunk_start[chunk as usize]..self.chunk_start[chunk as usize + 1]
    }

    fn edges_of(&self, node: u32) -> &[Edge] {
        let (start, end) = (
            self.edge_start[node as usize],
            self.edge_start[node as usize + 1],
        );
        &self.edges[start as usize..end as usize]
    }
}

/// Searches with a [`Hierarchy`]. It owns the buffers for the searches and an exact
/// [`Pathfinder`] for the cases the hierarchy cannot serve.
pub struct HierarchicalPathfinder {
    exact: Pathfinder,
    start: Local,
    finish: Local,
    refine: Local,
    generation: u32,
    stamp: Vec<u32>,
    closed: Vec<u32>,
    cost: Vec<u32>,
    parent: Vec<u32>,
    via: Vec<u32>,
    open: BinaryHeap<Reverse<u64>>,
    expansions: u32,
    greed: u32,
    side: u32,
}

impl HierarchicalPathfinder {
    pub fn new<T: Topology>(map: &GridMap<T>, hierarchy: &Hierarchy) -> Self {
        let nodes = hierarchy.nodes.len() + 1;
        Self {
            exact: Pathfinder::new(map),
            start: Local::new(hierarchy.side),
            finish: Local::new(hierarchy.side),
            refine: Local::new(hierarchy.side),
            generation: 0,
            stamp: vec![0; nodes],
            closed: vec![0; nodes],
            cost: vec![0; nodes],
            parent: vec![0; nodes],
            via: vec![0; nodes],
            open: BinaryHeap::new(),
            expansions: 0,
            greed: DEFAULT_GREED,
            side: hierarchy.side,
        }
    }

    /// Resizes the buffers when the hierarchy was built again with other dimensions.
    fn fit(&mut self, hierarchy: &Hierarchy) {
        let nodes = hierarchy.nodes.len() + 1;
        if self.side != hierarchy.side {
            self.side = hierarchy.side;
            self.start = Local::new(hierarchy.side);
            self.finish = Local::new(hierarchy.side);
            self.refine = Local::new(hierarchy.side);
        }
        if self.stamp.len() != nodes {
            self.stamp.clear();
            self.stamp.resize(nodes, 0);
            self.closed.clear();
            self.closed.resize(nodes, 0);
            self.cost.resize(nodes, 0);
            self.parent.resize(nodes, 0);
            self.via.resize(nodes, 0);
        }
    }

    /// Sets how much the entrance search overweights its estimate, in tenths: 10 is the plain
    /// estimate (the cheapest paths this hierarchy can give), and each tenth more is faster and
    /// dearer. Values under 10 are raised to 10.
    pub fn set_greed(&mut self, tenths: u32) {
        self.greed = tenths.max(10);
    }

    /// How many cells and entrances the last search expanded.
    pub fn last_expansions(&self) -> u32 {
        self.expansions
    }

    /// Finds a path from `from` to `to`, excluding `from` and including `to`, like
    /// [`Pathfinder::find`]. The path is not always the cheapest, and the cost reported is the
    /// cost of the path returned. The search is exact `Pathfinder::find` instead when the
    /// hierarchy is stale, occupants count as walls, an expansion limit is set, an end is a
    /// wall, or the hierarchy finds no route (one may exist through a chunk corner).
    #[allow(clippy::too_many_arguments)]
    pub fn find<T: Topology>(
        &mut self,
        map: &GridMap<T>,
        occupancy: &Occupancy,
        hierarchy: &Hierarchy,
        from: Cell,
        to: Cell,
        options: PathOptions,
        out: &mut Vec<Cell>,
    ) -> PathResult {
        out.clear();
        self.expansions = 0;
        self.fit(hierarchy);
        if from == to {
            return PathResult::Found { cost: 0 };
        }
        let servable = hierarchy.is_current(map)
            && !options.treat_occupants_as_walls
            && options.maximum_expansions == u32::MAX
            && map.is_passable(from)
            && map.is_passable(to);
        if servable {
            match self.search(map, hierarchy, from, to, out) {
                Outcome::Found(cost) => return PathResult::Found { cost },
                Outcome::Unreachable => {
                    out.clear();
                    return PathResult::Unreachable;
                }
                Outcome::Unknown => out.clear(),
            }
        }
        let result = self.exact.find(map, occupancy, from, to, options, out);
        self.expansions += self.exact.last_expansions();
        result
    }

    fn search<T: Topology>(
        &mut self,
        map: &GridMap<T>,
        hierarchy: &Hierarchy,
        from: Cell,
        to: Cell,
        out: &mut Vec<Cell>,
    ) -> Outcome {
        let (from_chunk, to_chunk) = (hierarchy.chunk_of(map, from), hierarchy.chunk_of(map, to));
        let (side, columns) = (hierarchy.side, hierarchy.columns);
        let from_region = Hierarchy::region_of(map, side, columns, from_chunk);
        let to_region = Hierarchy::region_of(map, side, columns, to_chunk);
        if from_chunk == to_chunk {
            self.refine.run(map, from_region, from, Some(to), false);
            self.expansions += self.refine.expansions;
            let cost = self.refine.cost_of(map, from_region, to);
            if cost == FAR {
                return Outcome::Unknown;
            }
            Self::walk_back(&self.refine, map, from_region, from, to, out);
            return Outcome::Found(cost);
        }
        self.start.run(map, from_region, from, None, false);
        self.finish.run(map, to_region, to, None, true);
        self.expansions += self.start.expansions + self.finish.expansions;
        if !self.start.leaves(map, from_region, false) || !self.finish.leaves(map, to_region, true)
        {
            return Outcome::Unreachable;
        }

        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        let goal = hierarchy.nodes.len() as u32;
        self.open.clear();
        let greed = self.greed as u64;
        let towards = |node: u32| {
            let straight = map.straight_cost(hierarchy.nodes[node as usize], to) as u64;
            (straight * greed / 10).min(FAR as u64 - 1) as u32
        };
        for node in hierarchy.nodes_in(from_chunk) {
            let cell = hierarchy.nodes[node as usize];
            let cost = self.start.cost_of(map, from_region, cell);
            if cost != FAR {
                self.relax(node, cost, FAR, FAR, generation, towards(node));
            }
        }
        let mut found = false;
        while let Some(Reverse(packed)) = self.open.pop() {
            let node = packed as u32;
            if node == goal {
                found = true;
                break;
            }
            if self.closed[node as usize] == generation {
                continue;
            }
            self.closed[node as usize] = generation;
            self.expansions += 1;
            let here = self.cost[node as usize];
            let cell = hierarchy.nodes[node as usize];
            if hierarchy.chunk_of(map, cell) == to_chunk {
                let rest = self.finish.cost_of(map, to_region, cell);
                if rest != FAR {
                    self.relax(goal, here.saturating_add(rest), node, FAR, generation, 0);
                }
            }
            let first_edge = hierarchy.edge_start[node as usize];
            for (offset, edge) in hierarchy.edges_of(node).iter().enumerate() {
                let tentative = here.saturating_add(edge.cost);
                let index = first_edge + offset as u32;
                self.relax(
                    edge.to,
                    tentative,
                    node,
                    index,
                    generation,
                    towards(edge.to),
                );
            }
        }
        if !found {
            return Outcome::Unknown;
        }

        let mut route = Vec::new();
        let mut node = self.parent[goal as usize];
        while node != FAR {
            route.push(node);
            node = self.parent[node as usize];
        }
        route.reverse();
        let first = hierarchy.nodes[route[0] as usize];
        Self::walk_back(&self.start, map, from_region, from, first, out);
        for node in route.iter().skip(1) {
            let edge = hierarchy.edges[self.via[*node as usize] as usize];
            if edge.length == 0 {
                out.push(hierarchy.nodes[*node as usize]);
            } else {
                let start = edge.path as usize;
                out.extend_from_slice(&hierarchy.paths[start..start + edge.length as usize]);
            }
        }
        let last = route[route.len() - 1];
        let mut cell = hierarchy.nodes[last as usize];
        while cell != to {
            cell = self.finish.link_of(map, to_region, cell);
            out.push(cell);
        }
        Outcome::Found(self.cost[goal as usize])
    }

    fn relax(
        &mut self,
        node: u32,
        cost: u32,
        parent: u32,
        via: u32,
        generation: u32,
        remaining: u32,
    ) {
        let slot = node as usize;
        let known = if self.stamp[slot] == generation {
            self.cost[slot]
        } else {
            FAR
        };
        if cost < known {
            self.stamp[slot] = generation;
            self.cost[slot] = cost;
            self.parent[slot] = parent;
            // The estimate is not consistent, so a settled entrance may still get cheaper.
            self.closed[slot] = 0;
            self.via[slot] = via;
            self.open.push(key(cost.saturating_add(remaining), node));
        }
    }

    /// Appends the cells from `origin` (excluded) to `target` (included), following the links of
    /// the last run back from the target.
    fn walk_back<T: Topology>(
        local: &Local,
        map: &GridMap<T>,
        region: Region,
        origin: Cell,
        target: Cell,
        out: &mut Vec<Cell>,
    ) {
        let begin = out.len();
        let mut cell = target;
        while cell != origin {
            out.push(cell);
            cell = local.link_of(map, region, cell);
        }
        out[begin..].reverse();
    }
}
