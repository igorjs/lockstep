// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{Column, Handle, Message};
use lockstep_spatial::{Cell, GridMap, Topology};
use serde::{Deserialize, Serialize};

/// A structure: the cells it blocks and how much wear it can take.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Structure {
    cells: Vec<Cell>,
    durability: u32,
    maximum: u32,
}

impl Structure {
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn durability(&self) -> u32 {
        self.durability
    }

    pub fn maximum(&self) -> u32 {
        self.maximum
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// No cells, or a durability of zero.
    Empty,
    /// The entity is already a structure.
    AlreadyBuilt,
    /// A cell is not on the map.
    OffMap(Cell),
    /// A cell is a wall already, or listed twice: something else holds it.
    Blocked(Cell),
    /// The entity is not a structure.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum StructureEvent {
    Built {
        structure: Handle,
    },
    Damaged {
        structure: Handle,
        durability: u32,
    },
    /// Durability ran out: the cells are passable again and the structure is gone.
    Collapsed {
        structure: Handle,
    },
    Repaired {
        structure: Handle,
        durability: u32,
    },
    /// Taken apart on purpose: the cells are passable again.
    Dismantled {
        structure: Handle,
    },
}

/// Durable multi-cell structures. A structure is an entity whose cells are walls on a map for as
/// long as it stands. Raising it, and bringing it down, change the map's passability through the
/// map's own setters, so the chunks that hold its cells are marked dirty for hosts and caches.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Structures {
    standing: Column<Structure>,
}

impl Structures {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, structure: Handle) -> Option<&Structure> {
        self.standing.get(structure)
    }

    pub fn len(&self) -> usize {
        self.standing.len()
    }

    pub fn is_empty(&self) -> bool {
        self.standing.is_empty()
    }

    /// The structure holding a cell.
    pub fn at(&self, cell: Cell) -> Option<Handle> {
        self.standing
            .iter()
            .find(|(_, structure)| structure.cells.contains(&cell))
            .map(|(handle, _)| handle)
    }

    /// Raises a structure over open cells, all or nothing. Every cell must be on the map,
    /// passable, and listed once; the first that is not is named and the map is left alone.
    pub fn build<T: Topology>(
        &mut self,
        map: &mut GridMap<T>,
        structure: Handle,
        cells: &[Cell],
        durability: u32,
        events: &mut Vec<StructureEvent>,
    ) -> Result<(), Refusal> {
        if cells.is_empty() || durability == 0 {
            return Err(Refusal::Empty);
        }
        if self.standing.has(structure) {
            return Err(Refusal::AlreadyBuilt);
        }
        for (index, cell) in cells.iter().enumerate() {
            if cell.0 as usize >= map.cell_count() {
                return Err(Refusal::OffMap(*cell));
            }
            if !map.is_passable(*cell) || cells[..index].contains(cell) {
                return Err(Refusal::Blocked(*cell));
            }
        }
        for cell in cells {
            map.set_passable(*cell, false);
        }
        self.standing.set(
            structure,
            Structure {
                cells: cells.to_vec(),
                durability,
                maximum: durability,
            },
        );
        events.push(StructureEvent::Built { structure });
        Ok(())
    }

    /// Wears a structure down. At zero it collapses: the structure is removed and its cells
    /// open again.
    pub fn damage<T: Topology>(
        &mut self,
        map: &mut GridMap<T>,
        structure: Handle,
        amount: u32,
        events: &mut Vec<StructureEvent>,
    ) -> Result<(), Refusal> {
        let held = self.standing.get_mut(structure).ok_or(Refusal::Unknown)?;
        held.durability = held.durability.saturating_sub(amount);
        if held.durability > 0 {
            events.push(StructureEvent::Damaged {
                structure,
                durability: held.durability,
            });
            return Ok(());
        }
        self.remove(map, structure);
        events.push(StructureEvent::Collapsed { structure });
        Ok(())
    }

    /// Mends a structure, up to the durability it was built with.
    pub fn repair(
        &mut self,
        structure: Handle,
        amount: u32,
        events: &mut Vec<StructureEvent>,
    ) -> Result<(), Refusal> {
        let held = self.standing.get_mut(structure).ok_or(Refusal::Unknown)?;
        held.durability = held.durability.saturating_add(amount).min(held.maximum);
        events.push(StructureEvent::Repaired {
            structure,
            durability: held.durability,
        });
        Ok(())
    }

    /// Takes a structure apart on purpose.
    pub fn dismantle<T: Topology>(
        &mut self,
        map: &mut GridMap<T>,
        structure: Handle,
        events: &mut Vec<StructureEvent>,
    ) -> Result<(), Refusal> {
        if !self.standing.has(structure) {
            return Err(Refusal::Unknown);
        }
        self.remove(map, structure);
        events.push(StructureEvent::Dismantled { structure });
        Ok(())
    }

    fn remove<T: Topology>(&mut self, map: &mut GridMap<T>, structure: Handle) {
        if let Some(gone) = self.standing.unset(structure) {
            for cell in gone.cells {
                map.set_passable(cell, true);
            }
        }
    }
}
