// SPDX-License-Identifier: Apache-2.0
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt;

/// Slot index plus generation. A stale handle is detected, never dereferenced.
///
/// The low 32 bits are the slot index and the high 32 bits are the generation. Generations start
/// at one, so the all-zero handle (`Handle::from_raw(0)`) never refers to anything.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Handle(u64);

impl Handle {
    fn new(generation: u32, index: u32) -> Self {
        Handle(((generation as u64) << 32) | index as u64)
    }

    pub fn raw(self) -> u64 {
        self.0
    }

    pub fn from_raw(raw: u64) -> Self {
        Handle(raw)
    }

    /// The slot index, the position that iteration order follows.
    pub fn slot_index(self) -> u32 {
        (self.0 & 0xffff_ffff) as u32
    }

    pub fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }
}

/// The next generation for a slot. Zero is skipped so a zero handle stays invalid.
fn next_generation(generation: u32) -> u32 {
    match generation.wrapping_add(1) {
        0 => 1,
        next => next,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Slot<T> {
    /// The generation of the current occupant, or of the next one when the slot is vacant.
    generation: u32,
    value: Option<T>,
}

/// The entity registry. Iteration is by slot index on every platform.
///
/// Inserting always reuses the lowest vacant slot, so which slot an entity gets depends only on
/// which slots are occupied, never on the order earlier entities were removed. The vacant set is
/// derived from the slots when a store is loaded, so a restored store continues exactly as the
/// original would have.
#[derive(Clone)]
pub struct StableVector<T> {
    slots: Vec<Slot<T>>,
    vacant: BTreeSet<u32>,
    occupied: usize,
}

impl<T> Default for StableVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for StableVector<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for StableVector<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(left, right)| left == right)
    }
}

impl<T: Serialize> Serialize for StableVector<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.slots.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for StableVector<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let slots = Vec::<Slot<T>>::deserialize(deserializer)?;
        if slots.len() > u32::MAX as usize {
            return Err(D::Error::custom("too many slots"));
        }
        let mut vacant = BTreeSet::new();
        let mut occupied = 0;
        for (index, slot) in slots.iter().enumerate() {
            if slot.generation == 0 {
                return Err(D::Error::custom("a slot generation is zero"));
            }
            if slot.value.is_some() {
                occupied += 1;
            } else {
                vacant.insert(index as u32);
            }
        }
        Ok(Self {
            slots,
            vacant,
            occupied,
        })
    }
}

impl<T> StableVector<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            vacant: BTreeSet::new(),
            occupied: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> Handle {
        self.occupied += 1;
        if let Some(index) = self.vacant.pop_first() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            return Handle::new(slot.generation, index);
        }
        assert!(self.slots.len() < u32::MAX as usize, "the store is full");
        let index = self.slots.len() as u32;
        self.slots.push(Slot {
            generation: 1,
            value: Some(value),
        });
        Handle::new(1, index)
    }

    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        let slot = self.slots.get_mut(handle.slot_index() as usize)?;
        if slot.generation != handle.generation() {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = next_generation(slot.generation);
        self.vacant.insert(handle.slot_index());
        self.occupied -= 1;
        Some(value)
    }

    pub fn contains(&self, handle: Handle) -> bool {
        self.get(handle).is_some()
    }

    pub fn get(&self, handle: Handle) -> Option<&T> {
        let slot = self.slots.get(handle.slot_index() as usize)?;
        if slot.generation != handle.generation() {
            return None;
        }
        slot.value.as_ref()
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.slot_index() as usize)?;
        if slot.generation != handle.generation() {
            return None;
        }
        slot.value.as_mut()
    }

    pub fn len(&self) -> usize {
        self.occupied
    }

    pub fn is_empty(&self) -> bool {
        self.occupied == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|value| (Handle::new(slot.generation, index as u32), value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let generation = slot.generation;
                slot.value
                    .as_mut()
                    .map(|value| (Handle::new(generation, index as u32), value))
            })
    }

    pub fn handles(&self) -> Vec<Handle> {
        self.iter().map(|(handle, _)| handle).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Entry<T> {
    generation: u32,
    value: T,
}

/// A component column keyed by the same handles. Absent means the entity lacks it.
///
/// An entry remembers the generation of the handle it was set for, so a stale handle finds
/// nothing. Trailing empty slots are trimmed, so two columns holding the same entries serialize
/// and hash the same whatever history built them.
#[derive(Clone)]
pub struct Column<T> {
    entries: Vec<Option<Entry<T>>>,
    count: usize,
}

impl<T> Default for Column<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for Column<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for Column<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(left, right)| left == right)
    }
}

impl<T: Serialize> Serialize for Column<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.entries.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Column<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut entries = Vec::<Option<Entry<T>>>::deserialize(deserializer)?;
        if entries.len() > u32::MAX as usize {
            return Err(D::Error::custom("too many entries"));
        }
        if entries.iter().flatten().any(|entry| entry.generation == 0) {
            return Err(D::Error::custom("an entry generation is zero"));
        }
        while matches!(entries.last(), Some(None)) {
            entries.pop();
        }
        let count = entries.iter().flatten().count();
        Ok(Self { entries, count })
    }
}

impl<T> Column<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            count: 0,
        }
    }

    /// Sets the value for a handle. Returns the previous value for the same handle, if any.
    pub fn set(&mut self, handle: Handle, value: T) -> Option<T> {
        let index = handle.slot_index() as usize;
        if self.entries.len() <= index {
            self.entries.resize_with(index + 1, || None);
        }
        let previous = self.entries[index].replace(Entry {
            generation: handle.generation(),
            value,
        });
        match previous {
            None => {
                self.count += 1;
                None
            }
            Some(entry) if entry.generation == handle.generation() => Some(entry.value),
            Some(_) => None,
        }
    }

    pub fn unset(&mut self, handle: Handle) -> Option<T> {
        let index = handle.slot_index() as usize;
        match self.entries.get(index)? {
            Some(entry) if entry.generation == handle.generation() => {}
            _ => return None,
        }
        let entry = self.entries[index].take()?;
        self.count -= 1;
        while matches!(self.entries.last(), Some(None)) {
            self.entries.pop();
        }
        Some(entry.value)
    }

    pub fn has(&self, handle: Handle) -> bool {
        self.get(handle).is_some()
    }

    pub fn get(&self, handle: Handle) -> Option<&T> {
        match self.entries.get(handle.slot_index() as usize)? {
            Some(entry) if entry.generation == handle.generation() => Some(&entry.value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        match self.entries.get_mut(handle.slot_index() as usize)? {
            Some(entry) if entry.generation == handle.generation() => Some(&mut entry.value),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle, &T)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .as_ref()
                    .map(|entry| (Handle::new(entry.generation, index as u32), &entry.value))
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, &mut T)> {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry.as_mut().map(|entry| {
                    (
                        Handle::new(entry.generation, index as u32),
                        &mut entry.value,
                    )
                })
            })
    }

    pub fn handles(&self) -> Vec<Handle> {
        self.iter().map(|(handle, _)| handle).collect()
    }
}
