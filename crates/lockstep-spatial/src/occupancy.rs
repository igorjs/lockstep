use crate::cell::Cell;
use crate::map::GridMap;
use crate::topology::Topology;
use lockstep_core::Handle;
use serde::{Deserialize, Serialize};

/// The handle that already holds a cell a body wanted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Occupied(pub Handle);

const EMPTY: u32 = u32::MAX;

/// Who stands where. A packed sparse set: cell to slot (`u32::MAX` for empty), a dense list of
/// occupants, and a handle-to-slot index sorted by handle. Lookup by cell is constant time, lookup by
/// handle is logarithmic, and moving a placed body is constant time plus its footprint. Placing a new
/// body and vacating one shift the sorted index, so they are linear in the number of bodies. Removal
/// swaps the last occupant into the freed slot, so no query depends on slot order: every query
/// returns its output sorted by cell.
///
/// A body may hold several cells (a footprint). Its first cell is its anchor.
///
/// The first caller to claim a cell gets it. A simulation that wants contested cells resolved in
/// handle order applies its moves in handle order.
///
/// It saves as the list of bodies sorted by handle, so two occupancies holding the same bodies on
/// the same cells save, compare and hash the same, whatever order they were placed in. A load checks
/// that every cell is inside the grid and held once.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "SavedOccupancy", into = "SavedOccupancy")]
pub struct Occupancy {
    width: u32,
    height: u32,
    cell_slot: Vec<u32>,
    occupants: Vec<Body>,
    /// (handle, slot), sorted by handle.
    index: Vec<(Handle, u32)>,
}

#[derive(Clone, Debug)]
struct Body {
    handle: Handle,
    cells: Vec<Cell>,
}

/// The saved form: the grid size and every body's cells, sorted by handle.
#[derive(Clone, Serialize, Deserialize)]
struct SavedOccupancy {
    width: u32,
    height: u32,
    bodies: Vec<(Handle, Vec<Cell>)>,
}

impl From<Occupancy> for SavedOccupancy {
    fn from(occupancy: Occupancy) -> Self {
        let bodies = occupancy
            .index
            .iter()
            .map(|(handle, slot)| (*handle, occupancy.occupants[*slot as usize].cells.clone()))
            .collect();
        SavedOccupancy {
            width: occupancy.width,
            height: occupancy.height,
            bodies,
        }
    }
}

impl TryFrom<SavedOccupancy> for Occupancy {
    type Error = String;

    fn try_from(saved: SavedOccupancy) -> Result<Self, String> {
        let cells = saved.width as u64 * saved.height as u64;
        if saved.width == 0 || saved.height == 0 || cells > u32::MAX as u64 {
            return Err("an occupancy needs a grid of at least one cell".into());
        }
        let mut occupancy = Occupancy::empty(saved.width, saved.height);
        for (handle, footprint) in saved.bodies {
            if footprint.is_empty() || footprint.iter().any(|cell| cell.0 as u64 >= cells) {
                return Err(format!(
                    "{handle:?} has an empty footprint or a cell outside the grid"
                ));
            }
            if occupancy.cells_of(handle).is_some() {
                return Err(format!("{handle:?} is saved twice"));
            }
            occupancy
                .move_footprint(handle, &footprint)
                .map_err(|Occupied(holder)| {
                    format!("{handle:?} and {holder:?} hold the same cell")
                })?;
        }
        Ok(occupancy)
    }
}

impl PartialEq for Occupancy {
    /// Equal when the same bodies hold the same cells. Slot order, which depends on history, is not
    /// compared.
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.index.len() == other.index.len()
            && self
                .index
                .iter()
                .zip(&other.index)
                .all(|((left, _), (right, _))| {
                    left == right && self.cells_of(*left) == other.cells_of(*right)
                })
    }
}

impl Occupancy {
    fn empty(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cell_slot: vec![EMPTY; (width * height) as usize],
            occupants: Vec::new(),
            index: Vec::new(),
        }
    }

    pub fn new<T: Topology>(map: &GridMap<T>) -> Self {
        Self::empty(map.width(), map.height())
    }

    /// The number of bodies placed.
    pub fn len(&self) -> usize {
        self.occupants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.occupants.is_empty()
    }

    pub fn at(&self, cell: Cell) -> Option<Handle> {
        match self.cell_slot[cell.0 as usize] {
            EMPTY => None,
            slot => Some(self.occupants[slot as usize].handle),
        }
    }

    fn slot_of(&self, who: Handle) -> Option<u32> {
        self.index
            .binary_search_by_key(&who, |(handle, _)| *handle)
            .ok()
            .map(|position| self.index[position].1)
    }

    /// The anchor cell of a body.
    pub fn cell_of(&self, who: Handle) -> Option<Cell> {
        self.slot_of(who)
            .map(|slot| self.occupants[slot as usize].cells[0])
    }

    /// Every cell a body holds, anchor first.
    pub fn cells_of(&self, who: Handle) -> Option<&[Cell]> {
        self.slot_of(who)
            .map(|slot| self.occupants[slot as usize].cells.as_slice())
    }

    /// Puts a body on one cell. A body already placed is moved there. Fails, changing nothing,
    /// when someone else holds the cell.
    pub fn place(&mut self, cell: Cell, who: Handle) -> Result<(), Occupied> {
        self.move_footprint(who, &[cell])
    }

    /// Removes a body. Returns its anchor cell.
    pub fn vacate(&mut self, who: Handle) -> Option<Cell> {
        let position = self
            .index
            .binary_search_by_key(&who, |(handle, _)| *handle)
            .ok()?;
        let slot = self.index.remove(position).1 as usize;
        let body = self.occupants.swap_remove(slot);
        for cell in &body.cells {
            self.cell_slot[cell.0 as usize] = EMPTY;
        }
        if slot < self.occupants.len() {
            // The last body took the freed slot: point its cells and its index entry there.
            let moved = &self.occupants[slot];
            for cell in &moved.cells {
                self.cell_slot[cell.0 as usize] = slot as u32;
            }
            let at = self
                .index
                .binary_search_by_key(&moved.handle, |(handle, _)| *handle)
                .expect("indexed");
            self.index[at].1 = slot as u32;
        }
        Some(body.cells[0])
    }

    /// Moves a body to one cell and returns the anchor it left. A body not placed yet is placed,
    /// and the new cell is returned. Fails, changing nothing, when someone else holds the cell.
    pub fn move_to(&mut self, who: Handle, cell: Cell) -> Result<Cell, Occupied> {
        let previous = self.cell_of(who);
        self.move_footprint(who, &[cell])?;
        Ok(previous.unwrap_or(cell))
    }

    /// Atomic multi-cell move for footprints (a body two cells wide). Every target cell must be
    /// free or already held by this body. On any conflict nothing changes and the first
    /// conflicting holder is reported. The first cell becomes the anchor. Cells must be distinct.
    pub fn move_footprint(&mut self, who: Handle, cells: &[Cell]) -> Result<(), Occupied> {
        assert!(!cells.is_empty(), "a footprint needs at least one cell");
        let own_slot = self.slot_of(who);
        for cell in cells {
            let slot = self.cell_slot[cell.0 as usize];
            if slot != EMPTY {
                let holder = self.occupants[slot as usize].handle;
                if holder != who {
                    return Err(Occupied(holder));
                }
            }
        }
        match own_slot {
            Some(slot) => {
                for cell in &self.occupants[slot as usize].cells {
                    self.cell_slot[cell.0 as usize] = EMPTY;
                }
                // Reuse the body's own list, so a move does not allocate.
                let held = &mut self.occupants[slot as usize].cells;
                held.clear();
                held.extend_from_slice(cells);
                for cell in cells {
                    self.cell_slot[cell.0 as usize] = slot;
                }
            }
            None => {
                let slot = self.occupants.len() as u32;
                self.occupants.push(Body {
                    handle: who,
                    cells: cells.to_vec(),
                });
                for cell in cells {
                    self.cell_slot[cell.0 as usize] = slot;
                }
                let position = self
                    .index
                    .binary_search_by_key(&who, |(handle, _)| *handle)
                    .unwrap_err();
                self.index.insert(position, (who, slot));
            }
        }
        Ok(())
    }

    /// Occupied cells within a radius (in tenths) of a centre, as (cell, handle), sorted by cell.
    /// Clears `out` first. The result does not depend on the order bodies were placed or moved.
    pub fn within<T: Topology>(
        &self,
        map: &GridMap<T>,
        centre: Cell,
        radius: u32,
        out: &mut Vec<(Cell, Handle)>,
    ) {
        debug_assert_eq!(
            (map.width(), map.height()),
            (self.width, self.height),
            "the map is not this occupancy's grid"
        );
        out.clear();
        let reach = radius.div_ceil(10) as i64 + 1;
        let (centre_x, centre_y) = (centre.0 % self.width, centre.0 / self.width);
        let low_x = (centre_x as i64 - reach).max(0) as u32;
        let high_x = (centre_x as i64 + reach).min(self.width as i64 - 1) as u32;
        let low_y = (centre_y as i64 - reach).max(0) as u32;
        let high_y = (centre_y as i64 + reach).min(self.height as i64 - 1) as u32;
        // Rows then columns in ascending order, so the output is already sorted by cell.
        for y in low_y..=high_y {
            for x in low_x..=high_x {
                let cell = Cell(y * self.width + x);
                if let Some(handle) = self.at(cell) {
                    if map.distance(centre, cell) <= radius {
                        out.push((cell, handle));
                    }
                }
            }
        }
    }
}
