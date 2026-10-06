// SPDX-License-Identifier: Apache-2.0
use crate::catalogue::{AffixId, Catalogue, KindId, SlotId};
use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, ModifierHandle, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, Message, StableVector};
use serde::{Deserialize, Serialize};

/// Raw 16.16 units in one game minute, times 100 percent: one minute of spoiling at the base rate.
const MINUTE_AT_FULL_RATE: u64 = 65_536 * 100;

/// One item: a kind, a count of units, and its own affixes and spoilage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub kind: KindId,
    pub count: u16,
    pub affixes: Vec<AffixId>,
    /// Raw 16.16 game minutes times the spoilage percent, added exactly.
    exposure: u64,
    pub spoiled: bool,
}

impl Item {
    /// Raw 16.16 game minutes times the spoilage percent the item has been through.
    pub fn exposure(&self) -> u64 {
        self.exposure
    }
}

/// Where an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// Held by nothing: just made, split off, taken out or taken off.
    Loose,
    In(Handle),
    Worn {
        owner: Handle,
        slot: SlotId,
    },
}

/// Holds items in order, up to a number of slots and an optional weight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Container {
    pub slots: u16,
    pub weight_limit: Option<Fixed32>,
    /// Whole degrees, for spoilage.
    pub temperature: i32,
    items: Vec<Handle>,
}

impl Container {
    pub fn new(slots: u16, weight_limit: Option<Fixed32>, temperature: i32) -> Self {
        Container {
            slots,
            weight_limit,
            temperature,
            items: Vec::new(),
        }
    }

    /// The items inside, in the order they went in.
    pub fn items(&self) -> &[Handle] {
        &self.items
    }
}

/// Where a worn modifier came from, so removing an affix removes only its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Source {
    Kind,
    Affix(AffixId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Worn {
    item: Handle,
    source: Source,
    attribute: AttributeId,
    modifier: ModifierHandle,
}

/// One wearer's slots, and the modifiers its worn items hold on its attributes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
    slots: Vec<Option<Handle>>,
    modifiers: Vec<Worn>,
}

impl Equipment {
    pub fn worn(&self, slot: SlotId) -> Option<Handle> {
        self.slots.get(slot.0 as usize).copied().flatten()
    }
}

/// The wearers' attributes, which worn items and their affixes modify.
pub struct Wearers<'a> {
    pub attributes: &'a mut Column<Attributes>,
    pub registry: &'a Registry,
    pub events: &'a mut Vec<AttributeEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    UnknownItem,
    UnknownContainer,
    /// The owner has no equipment.
    UnknownWearer,
    /// Only a loose item can go into a container or be worn from nowhere.
    NotLoose,
    /// The item is worn: take it off first.
    Worn,
    /// Already in that container.
    AlreadyThere,
    /// The item is in no container to move from.
    NotContained,
    /// No free slot, and no stack it fits into whole.
    Full,
    TooHeavy,
    NotWearable,
    SlotTaken,
    NothingWorn,
    /// An affix binds the item to its wearer.
    Bound,
    /// Fewer units than asked for, or a split that would leave nothing.
    Count,
    NoAffixSlot,
    NoSuchAffix,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum InventoryEvent {
    /// Spoiled through: the item stays, marked spoiled.
    Spoiled { item: Handle },
}

/// Every item, container and wearer. Items are entities in their own store; containers and
/// wearers are keyed by the simulation's own handles, so a crate, a locker or a person can hold
/// items.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    items: StableVector<Item>,
    places: Column<Place>,
    containers: Column<Container>,
    equipment: Column<Equipment>,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(&self, item: Handle) -> Option<&Item> {
        self.items.get(item)
    }

    pub fn place(&self, item: Handle) -> Option<Place> {
        self.places.get(item).copied()
    }

    pub fn container(&self, owner: Handle) -> Option<&Container> {
        self.containers.get(owner)
    }

    pub fn equipment(&self, owner: Handle) -> Option<&Equipment> {
        self.equipment.get(owner)
    }

    /// Every item, in handle order.
    pub fn items(&self) -> impl Iterator<Item = (Handle, &Item)> {
        self.items.iter()
    }

    /// Makes a loose item of `count` units. Panics when the count is zero or above the kind's
    /// stack.
    pub fn create(&mut self, catalogue: &Catalogue, kind: KindId, count: u16) -> Handle {
        assert!(
            count >= 1 && count <= catalogue.kind(kind).stack,
            "an item holds 1 to {} units",
            catalogue.kind(kind).stack
        );
        let item = self.items.insert(Item {
            kind,
            count,
            affixes: Vec::new(),
            exposure: 0,
            spoiled: false,
        });
        self.places.set(item, Place::Loose);
        item
    }

    /// Gives an entity a container. Replacing one that holds items is refused.
    pub fn add_container(&mut self, owner: Handle, container: Container) -> Result<(), Refusal> {
        if self
            .containers
            .get(owner)
            .is_some_and(|held| !held.items.is_empty())
        {
            return Err(Refusal::Full);
        }
        self.containers.set(owner, container);
        Ok(())
    }

    /// Changes a container's temperature, such as a cold store losing power. Spoilage reads it
    /// from the next `spoil` on.
    pub fn set_temperature(&mut self, owner: Handle, degrees: i32) -> Result<(), Refusal> {
        let container = self
            .containers
            .get_mut(owner)
            .ok_or(Refusal::UnknownContainer)?;
        container.temperature = degrees;
        Ok(())
    }

    /// Gives an entity every equipment slot in the catalogue, all empty.
    pub fn add_wearer(&mut self, owner: Handle, catalogue: &Catalogue) {
        if !self.equipment.has(owner) {
            self.equipment.set(
                owner,
                Equipment {
                    slots: vec![None; catalogue.slot_count()],
                    modifiers: Vec::new(),
                },
            );
        }
    }

    /// The weight of everything in a container.
    pub fn weight_of(&self, owner: Handle, catalogue: &Catalogue) -> Fixed32 {
        let raw = self
            .containers
            .get(owner)
            .map_or(0, |container| self.raw_weight(&container.items, catalogue));
        Fixed32::from_raw(raw.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
    }

    fn raw_weight(&self, items: &[Handle], catalogue: &Catalogue) -> i64 {
        items
            .iter()
            .filter_map(|item| self.items.get(*item))
            .map(|item| unit_weight(item, catalogue) * item.count as i64)
            .sum()
    }

    /// Whether a container takes this item, and the stack it merges into when there is one: the
    /// first stack of the same kind and affixes, spoiled or not alike, with room for every unit. A put never splits an
    /// item; split it first.
    fn fits(
        &self,
        item: Handle,
        owner: Handle,
        catalogue: &Catalogue,
    ) -> Result<Option<Handle>, Refusal> {
        let held = self.items.get(item).ok_or(Refusal::UnknownItem)?;
        let container = self
            .containers
            .get(owner)
            .ok_or(Refusal::UnknownContainer)?;
        if let Some(limit) = container.weight_limit {
            let after = self.raw_weight(&container.items, catalogue)
                + unit_weight(held, catalogue) * held.count as i64;
            if after > limit.raw() as i64 {
                return Err(Refusal::TooHeavy);
            }
        }
        let stack = catalogue.kind(held.kind).stack;
        let merge = container.items.iter().copied().find(|other| {
            self.items.get(*other).is_some_and(|other| {
                other.kind == held.kind
                    && other.affixes == held.affixes
                    && other.spoiled == held.spoiled
                    && other.count as u32 + held.count as u32 <= stack as u32
            })
        });
        if merge.is_none() && container.items.len() >= container.slots as usize {
            return Err(Refusal::Full);
        }
        Ok(merge)
    }

    /// Puts a loose item into a container. Returns the item that holds the units afterwards:
    /// the item itself, or the stack it merged into, which then spoils as the staler of the two.
    pub fn put(
        &mut self,
        item: Handle,
        owner: Handle,
        catalogue: &Catalogue,
    ) -> Result<Handle, Refusal> {
        match self.places.get(item) {
            None => return Err(Refusal::UnknownItem),
            Some(Place::Loose) => {}
            Some(_) => return Err(Refusal::NotLoose),
        }
        match self.fits(item, owner, catalogue)? {
            Some(stack) => {
                let merged = self.items.remove(item).expect("checked");
                self.places.unset(item);
                let into = self.items.get_mut(stack).expect("listed");
                into.count += merged.count;
                into.exposure = into.exposure.max(merged.exposure);
                Ok(stack)
            }
            None => {
                self.containers
                    .get_mut(owner)
                    .expect("checked")
                    .items
                    .push(item);
                self.places.set(item, Place::In(owner));
                Ok(item)
            }
        }
    }

    /// Takes an item out of its container; it becomes loose.
    pub fn take(&mut self, item: Handle) -> Result<(), Refusal> {
        match self.places.get(item).copied() {
            None => Err(Refusal::UnknownItem),
            Some(Place::Loose) => Ok(()),
            Some(Place::Worn { .. }) => Err(Refusal::Worn),
            Some(Place::In(owner)) => {
                self.detach(item, owner);
                self.places.set(item, Place::Loose);
                Ok(())
            }
        }
    }

    fn detach(&mut self, item: Handle, owner: Handle) {
        if let Some(container) = self.containers.get_mut(owner) {
            container.items.retain(|held| *held != item);
        }
    }

    /// Moves an item from its container into another, all or nothing: when the other refuses,
    /// nothing changes. Returns the item that holds the units afterwards, as `put` does.
    pub fn move_between(
        &mut self,
        item: Handle,
        to: Handle,
        catalogue: &Catalogue,
    ) -> Result<Handle, Refusal> {
        let from = match self.places.get(item).copied() {
            None => return Err(Refusal::UnknownItem),
            Some(Place::Worn { .. }) => return Err(Refusal::Worn),
            Some(Place::Loose) => return Err(Refusal::NotContained),
            Some(Place::In(from)) => from,
        };
        if from == to {
            return Err(Refusal::AlreadyThere);
        }
        self.fits(item, to, catalogue)?;
        self.take(item)?;
        Ok(self.put(item, to, catalogue).expect("checked"))
    }

    /// Splits `count` units off into a new loose item with the same affixes and spoilage.
    pub fn split(&mut self, item: Handle, count: u16) -> Result<Handle, Refusal> {
        if matches!(self.places.get(item), Some(Place::Worn { .. })) {
            return Err(Refusal::Worn);
        }
        let held = self.items.get_mut(item).ok_or(Refusal::UnknownItem)?;
        if count == 0 || count >= held.count {
            return Err(Refusal::Count);
        }
        held.count -= count;
        let mut part = held.clone();
        part.count = count;
        let part = self.items.insert(part);
        self.places.set(part, Place::Loose);
        Ok(part)
    }

    /// Uses up units, wherever the item is but on a wearer. Returns the units left; at none the
    /// item is gone.
    pub fn consume(&mut self, item: Handle, count: u16) -> Result<u16, Refusal> {
        let place = self.places.get(item).copied().ok_or(Refusal::UnknownItem)?;
        if matches!(place, Place::Worn { .. }) {
            return Err(Refusal::Worn);
        }
        let held = self.items.get_mut(item).ok_or(Refusal::UnknownItem)?;
        if count > held.count {
            return Err(Refusal::Count);
        }
        held.count -= count;
        let left = held.count;
        if left == 0 {
            self.destroy(item)?;
        }
        Ok(left)
    }

    /// Removes an item, wherever it is but on a wearer.
    pub fn destroy(&mut self, item: Handle) -> Result<(), Refusal> {
        match self.places.get(item).copied() {
            None => return Err(Refusal::UnknownItem),
            Some(Place::Worn { .. }) => return Err(Refusal::Worn),
            Some(Place::In(owner)) => self.detach(item, owner),
            Some(Place::Loose) => {}
        }
        self.places.unset(item);
        self.items.remove(item);
        Ok(())
    }

    /// The items in a container whose kind has a tag, in container order.
    pub fn find_by_tag(&self, owner: Handle, tag: &str, catalogue: &Catalogue) -> Vec<Handle> {
        self.containers
            .get(owner)
            .map_or_else(Vec::new, |container| {
                container
                    .items
                    .iter()
                    .copied()
                    .filter(|item| {
                        self.items
                            .get(*item)
                            .is_some_and(|held| catalogue.kind(held.kind).has_tag(tag))
                    })
                    .collect()
            })
    }

    /// Puts an item on in its kind's slot, from loose or from a container, and applies its kind's
    /// and affixes' modifiers to the wearer's attributes, when it has any.
    pub fn equip(
        &mut self,
        owner: Handle,
        item: Handle,
        catalogue: &Catalogue,
        wearers: &mut Wearers<'_>,
    ) -> Result<SlotId, Refusal> {
        let held = self.items.get(item).ok_or(Refusal::UnknownItem)?;
        let slot = catalogue.kind(held.kind).slot.ok_or(Refusal::NotWearable)?;
        let equipment = self.equipment.get(owner).ok_or(Refusal::UnknownWearer)?;
        if equipment.worn(slot).is_some() {
            return Err(Refusal::SlotTaken);
        }
        self.take(item)?;
        let affixes = held_affixes(&self.items, item);
        let equipment = self.equipment.get_mut(owner).expect("checked");
        equipment.slots[slot.0 as usize] = Some(item);
        self.places.set(item, Place::Worn { owner, slot });
        let kind = self.items.get(item).expect("checked").kind;
        let mut sources = vec![(Source::Kind, &catalogue.kind(kind).modifiers)];
        for affix in &affixes {
            sources.push((Source::Affix(*affix), &catalogue.affix(*affix).modifiers));
        }
        for (source, modifiers) in sources {
            apply(equipment, owner, item, source, modifiers, wearers);
        }
        Ok(slot)
    }

    /// Takes off what a slot holds and removes its modifiers; it becomes loose. Refused while an
    /// affix binds it.
    pub fn unequip(
        &mut self,
        owner: Handle,
        slot: SlotId,
        catalogue: &Catalogue,
        wearers: &mut Wearers<'_>,
    ) -> Result<Handle, Refusal> {
        let equipment = self
            .equipment
            .get_mut(owner)
            .ok_or(Refusal::UnknownWearer)?;
        let item = equipment.worn(slot).ok_or(Refusal::NothingWorn)?;
        let held = self.items.get(item).expect("a worn item exists");
        if held
            .affixes
            .iter()
            .any(|affix| catalogue.affix(*affix).binds)
        {
            return Err(Refusal::Bound);
        }
        remove(equipment, owner, |worn| worn.item == item, wearers);
        equipment.slots[slot.0 as usize] = None;
        self.places.set(item, Place::Loose);
        Ok(item)
    }

    /// Adds an affix to an item with a free affix slot. On a worn item its modifiers apply at once.
    pub fn add_affix(
        &mut self,
        item: Handle,
        affix: AffixId,
        catalogue: &Catalogue,
        wearers: &mut Wearers<'_>,
    ) -> Result<(), Refusal> {
        let held = self.items.get_mut(item).ok_or(Refusal::UnknownItem)?;
        if held.affixes.len() >= catalogue.kind(held.kind).affix_slots as usize {
            return Err(Refusal::NoAffixSlot);
        }
        held.affixes.push(affix);
        if let Some(Place::Worn { owner, .. }) = self.places.get(item).copied() {
            let equipment = self.equipment.get_mut(owner).expect("a wearer");
            let modifiers = &catalogue.affix(affix).modifiers;
            apply(
                equipment,
                owner,
                item,
                Source::Affix(affix),
                modifiers,
                wearers,
            );
        }
        Ok(())
    }

    /// Removes one affix from an item, even a binding one from a worn item (a repair). On a worn
    /// item its modifiers go at once.
    pub fn remove_affix(
        &mut self,
        item: Handle,
        affix: AffixId,
        catalogue: &Catalogue,
        wearers: &mut Wearers<'_>,
    ) -> Result<(), Refusal> {
        let held = self.items.get_mut(item).ok_or(Refusal::UnknownItem)?;
        let index = held
            .affixes
            .iter()
            .position(|held| *held == affix)
            .ok_or(Refusal::NoSuchAffix)?;
        held.affixes.remove(index);
        if let Some(Place::Worn { owner, .. }) = self.places.get(item).copied() {
            let equipment = self.equipment.get_mut(owner).expect("a wearer");
            // One copy's modifiers go, even when the item carries the same affix twice.
            let mut left = catalogue.affix(affix).modifiers.len();
            remove(
                equipment,
                owner,
                |worn| {
                    let matches =
                        left > 0 && worn.item == item && worn.source == Source::Affix(affix);
                    left -= matches as usize;
                    matches
                },
                wearers,
            );
        }
        Ok(())
    }

    /// Spoils every item that spoils by `minutes` of game time, at its container's temperature,
    /// or at `outside` when it is loose or worn. Exposure adds up exactly, so the same minutes
    /// spoil the same whatever the steps they are cut into.
    pub fn spoil(
        &mut self,
        minutes: Fixed32,
        outside: i32,
        catalogue: &Catalogue,
        events: &mut Vec<InventoryEvent>,
    ) {
        let minutes = minutes.raw().max(0) as u64;
        for (handle, item) in self.items.iter_mut() {
            let Some(limit) = catalogue.kind(item.kind).spoils_after_minutes else {
                continue;
            };
            if item.spoiled {
                continue;
            }
            let temperature = match self.places.get(handle) {
                Some(Place::In(owner)) => self
                    .containers
                    .get(*owner)
                    .map_or(outside, |container| container.temperature),
                _ => outside,
            };
            let percent = catalogue.spoilage_percent(temperature) as u64;
            item.exposure = item.exposure.saturating_add(minutes * percent);
            if item.exposure >= limit as u64 * MINUTE_AT_FULL_RATE {
                item.spoiled = true;
                events.push(InventoryEvent::Spoiled { item: handle });
            }
        }
    }

    /// How fresh an item is, from 1 (new) to 0 (spoiled); `None` for an unknown item or a kind
    /// that never spoils.
    pub fn freshness(&self, item: Handle, catalogue: &Catalogue) -> Option<Fixed32> {
        let held = self.items.get(item)?;
        let limit =
            catalogue.kind(held.kind).spoils_after_minutes? as u128 * MINUTE_AT_FULL_RATE as u128;
        if limit == 0 {
            return Some(Fixed32::ZERO);
        }
        // Rounded so freshness is never reported higher than it is.
        let used = (held.exposure as u128 * 65_536).div_ceil(limit).min(65_536);
        Some(Fixed32::from_raw((65_536 - used) as i32))
    }
}

fn unit_weight(item: &Item, catalogue: &Catalogue) -> i64 {
    catalogue.kind(item.kind).weight.raw() as i64
}

fn held_affixes(items: &StableVector<Item>, item: Handle) -> Vec<AffixId> {
    items
        .get(item)
        .map_or_else(Vec::new, |held| held.affixes.clone())
}

fn apply(
    equipment: &mut Equipment,
    owner: Handle,
    item: Handle,
    source: Source,
    modifiers: &[(AttributeId, lockstep_attributes::Modifier)],
    wearers: &mut Wearers<'_>,
) {
    let Some(attributes) = wearers.attributes.get_mut(owner) else {
        return;
    };
    for (attribute, modifier) in modifiers {
        let handle = attributes.add_modifier(
            owner,
            *attribute,
            *modifier,
            wearers.registry,
            wearers.events,
        );
        equipment.modifiers.push(Worn {
            item,
            source,
            attribute: *attribute,
            modifier: handle,
        });
    }
}

fn remove(
    equipment: &mut Equipment,
    owner: Handle,
    mut which: impl FnMut(&Worn) -> bool,
    wearers: &mut Wearers<'_>,
) {
    let mut kept = Vec::with_capacity(equipment.modifiers.len());
    for worn in std::mem::take(&mut equipment.modifiers) {
        if which(&worn) {
            if let Some(attributes) = wearers.attributes.get_mut(owner) {
                attributes.remove_modifier(
                    owner,
                    worn.attribute,
                    worn.modifier,
                    wearers.registry,
                    wearers.events,
                );
            }
        } else {
            kept.push(worn);
        }
    }
    equipment.modifiers = kept;
}
