use crate::cell::Cell;
use crate::topology::Topology;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Cells per chunk side. Chunks are 32 by 32.
pub const CHUNK: u32 = 32;

/// The wall height that blocks everything: a wall of 255 steps on top of the cell's elevation.
const FULL_WALL: u8 = 255;

/// A dense map: passability, movement cost, elevation, and wall height per cell.
///
/// Elevation is a property of a cell, not a third axis. A cell that is not passable is a wall.
/// `set_passable(cell, false)` makes a full wall (it blocks sight at the cell's elevation plus 255,
/// so nothing sees over it), and `set_low_wall` makes a wall of a given height that blocks sight
/// only when it reaches the line of sight. Interiors with floors are separate maps linked by portal
/// cells, a later feature.
///
/// A save holds the cells only. The dirty chunk marks are bookkeeping for hosts and caches, not
/// state: they are left out of saves, hashes and equality, so two maps with the same cells are
/// equal whatever history built them. A loaded map is checked the same way `new` checks its
/// arguments, and reports every chunk as changed so a host redraws it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "", try_from = "SavedMap", into = "SavedMap")]
pub struct GridMap<T: Topology> {
    width: u32,
    height: u32,
    /// 0 or 1.
    passable: Vec<u8>,
    /// 1 is normal; higher is slower. Tags for ground kind live beside it.
    cost: Vec<u8>,
    /// Gameplay height in steps.
    elevation: Vec<u8>,
    /// Height of a wall above the cell's elevation. Zero for open ground.
    wall_height: Vec<u8>,
    /// The largest climb in one move.
    step_limit: u8,
    /// One bit per 32 by 32 chunk, for hosts and caches.
    dirty_chunks: Vec<u64>,
    topology: PhantomData<T>,
}

/// The saved form of a map: everything except the dirty chunk marks.
#[derive(Clone, Serialize, Deserialize)]
struct SavedMap {
    width: u32,
    height: u32,
    passable: Vec<u8>,
    cost: Vec<u8>,
    elevation: Vec<u8>,
    wall_height: Vec<u8>,
    step_limit: u8,
}

impl<T: Topology> From<GridMap<T>> for SavedMap {
    fn from(map: GridMap<T>) -> Self {
        SavedMap {
            width: map.width,
            height: map.height,
            passable: map.passable,
            cost: map.cost,
            elevation: map.elevation,
            wall_height: map.wall_height,
            step_limit: map.step_limit,
        }
    }
}

impl<T: Topology> TryFrom<SavedMap> for GridMap<T> {
    type Error = String;

    fn try_from(saved: SavedMap) -> Result<Self, String> {
        if saved.width == 0 || saved.height == 0 {
            return Err("a map needs at least one cell".into());
        }
        let cells = saved.width as u64 * saved.height as u64;
        if cells > u32::MAX as u64 {
            return Err("the map is too large".into());
        }
        let lengths = [
            saved.passable.len(),
            saved.cost.len(),
            saved.elevation.len(),
            saved.wall_height.len(),
        ];
        if lengths.iter().any(|length| *length as u64 != cells) {
            return Err(format!(
                "a map of {cells} cells has per-cell lists of lengths {lengths:?}"
            ));
        }
        for index in 0..cells as usize {
            let passable = saved.passable[index];
            if passable > 1 || (passable == 1) != (saved.wall_height[index] == 0) {
                return Err(format!(
                    "cell {index} is passable {passable} with wall height {}",
                    saved.wall_height[index]
                ));
            }
        }
        let mut map = GridMap::new(saved.width, saved.height);
        map.passable = saved.passable;
        map.cost = saved.cost;
        map.elevation = saved.elevation;
        map.wall_height = saved.wall_height;
        map.step_limit = saved.step_limit;
        map.mark_all_dirty();
        Ok(map)
    }
}

impl<T: Topology> PartialEq for GridMap<T> {
    /// Equal when the cells are equal. The dirty chunk marks are not compared.
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.passable == other.passable
            && self.cost == other.cost
            && self.elevation == other.elevation
            && self.wall_height == other.wall_height
            && self.step_limit == other.step_limit
    }
}

impl<T: Topology> GridMap<T> {
    /// An open map: every cell passable, cost 1, elevation 0, step limit 1.
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "a map needs at least one cell");
        assert!(
            (width as u64) * (height as u64) <= u32::MAX as u64,
            "the map is too large"
        );
        let cells = (width * height) as usize;
        let chunks = (width.div_ceil(CHUNK) * height.div_ceil(CHUNK)) as usize;
        Self {
            width,
            height,
            passable: vec![1; cells],
            cost: vec![1; cells],
            elevation: vec![0; cells],
            wall_height: vec![0; cells],
            step_limit: 1,
            dirty_chunks: vec![0; chunks.div_ceil(64)],
            topology: PhantomData,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cell_count(&self) -> usize {
        self.passable.len()
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    /// The cell at (x, y). Panics when the position is outside the map.
    pub fn index(&self, x: u32, y: u32) -> Cell {
        assert!(self.contains(x, y), "({x}, {y}) is outside the map");
        Cell(y * self.width + x)
    }

    pub fn coordinates(&self, cell: Cell) -> (u32, u32) {
        (cell.0 % self.width, cell.0 / self.width)
    }

    pub fn is_passable(&self, cell: Cell) -> bool {
        self.passable[cell.0 as usize] != 0
    }

    pub fn cost(&self, cell: Cell) -> u8 {
        self.cost[cell.0 as usize]
    }

    pub fn elevation(&self, cell: Cell) -> u8 {
        self.elevation[cell.0 as usize]
    }

    pub fn wall_height(&self, cell: Cell) -> u8 {
        self.wall_height[cell.0 as usize]
    }

    pub fn step_limit(&self) -> u8 {
        self.step_limit
    }

    pub fn set_step_limit(&mut self, step_limit: u8) {
        self.step_limit = step_limit;
    }

    /// Whether a body can move from one cell to an adjacent one: the target is passable, the climb
    /// is within the step limit (a drop is always allowed), and a diagonal move does not cut the
    /// corner of a wall. A step that costs more than a straight step is a diagonal.
    pub fn can_step(&self, from: Cell, to: Cell) -> bool {
        if !self.is_passable(to) {
            return false;
        }
        if self.elevation(to) as u16 > self.elevation(from) as u16 + self.step_limit as u16 {
            return false;
        }
        if T::step_cost(from, to, self.width) > 10 {
            let (from_x, from_y) = self.coordinates(from);
            let (to_x, to_y) = self.coordinates(to);
            let across_a = Cell(from_y * self.width + to_x);
            let across_b = Cell(to_y * self.width + from_x);
            if !self.is_passable(across_a) || !self.is_passable(across_b) {
                return false;
            }
        }
        true
    }

    /// Sets a cell passable or not. Impassable means a full wall; passable clears any wall.
    pub fn set_passable(&mut self, cell: Cell, passable: bool) {
        let index = cell.0 as usize;
        self.passable[index] = passable as u8;
        self.wall_height[index] = if passable { 0 } else { FULL_WALL };
        self.mark_dirty(cell);
    }

    /// Makes the cell an impassable wall of the given height above its elevation.
    pub fn set_low_wall(&mut self, cell: Cell, height: u8) {
        let index = cell.0 as usize;
        self.passable[index] = (height == 0) as u8;
        self.wall_height[index] = height;
        self.mark_dirty(cell);
    }

    pub fn set_cost(&mut self, cell: Cell, cost: u8) {
        self.cost[cell.0 as usize] = cost;
        self.mark_dirty(cell);
    }

    pub fn set_elevation(&mut self, cell: Cell, elevation: u8) {
        self.elevation[cell.0 as usize] = elevation;
        self.mark_dirty(cell);
    }

    pub fn neighbours(&self, cell: Cell, out: &mut Vec<Cell>) {
        T::neighbours(cell, self.width, self.height, out);
    }

    pub fn distance(&self, a: Cell, b: Cell) -> u32 {
        T::distance(a, b, self.width)
    }

    /// The cost of one step in tenths: the topology's step cost times the terrain cost of the cell
    /// stepped onto (a cost of zero counts as one, so a step is never free).
    pub fn step_cost(&self, from: Cell, to: Cell) -> u32 {
        T::step_cost(from, to, self.width) * self.cost(to).max(1) as u32
    }

    /// The shortest possible cost between two cells on open ground with normal terrain: the
    /// topology distance. A heuristic that never overestimates, because terrain only adds cost.
    pub fn straight_cost(&self, a: Cell, b: Cell) -> u32 {
        T::distance(a, b, self.width)
    }

    pub fn line(&self, from: Cell, to: Cell, out: &mut Vec<Cell>) {
        T::line(from, to, self.width, out);
    }

    fn chunk_of(&self, cell: Cell) -> usize {
        let (x, y) = self.coordinates(cell);
        ((y / CHUNK) * self.width.div_ceil(CHUNK) + x / CHUNK) as usize
    }

    fn mark_all_dirty(&mut self) {
        let chunks = (self.width.div_ceil(CHUNK) * self.height.div_ceil(CHUNK)) as usize;
        for chunk in 0..chunks {
            self.dirty_chunks[chunk / 64] |= 1 << (chunk % 64);
        }
    }

    fn mark_dirty(&mut self, cell: Cell) {
        let chunk = self.chunk_of(cell);
        self.dirty_chunks[chunk / 64] |= 1 << (chunk % 64);
    }

    /// The chunks changed since the last call, in ascending order, and clears the marks.
    pub fn take_dirty_chunks(&mut self) -> Vec<u32> {
        let mut changed = Vec::new();
        for (word_index, word) in self.dirty_chunks.iter_mut().enumerate() {
            let mut remaining = *word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros();
                changed.push(word_index as u32 * 64 + bit);
                remaining &= remaining - 1;
            }
            *word = 0;
        }
        changed
    }
}
