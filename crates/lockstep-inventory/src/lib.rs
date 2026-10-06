// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod catalogue;
mod inventory;

pub use catalogue::{Affix, AffixId, Band, Catalogue, CatalogueError, Kind, KindId, SlotId};
pub use inventory::{
    Container, Equipment, Inventory, InventoryEvent, Item, Place, Refusal, Wearers,
};
