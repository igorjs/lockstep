// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, StableVector};
use lockstep_inventory::{
    AffixId, Catalogue, Container, Inventory, KindId, Refusal, SlotId, Wearers,
};

pub const REGISTRY: &str = r#"{
  "attributes": [
    { "name": "oxygen", "minimum": 0, "maximum": 100, "starting": 100 },
    { "name": "focus", "minimum": 0, "maximum": 100, "starting": 50 }
  ]
}"#;

pub const CATALOGUE: &str = r#"{
  "slots": ["suit", "hand"],
  "spoilage": [
    { "below": 5, "percent": 25 },
    { "below": 30, "percent": 100 },
    { "percent": 300 }
  ],
  "affixes": [
    { "name": "faulty_seal", "binds": true,
      "modifiers": [ { "attribute": "oxygen", "modifier": { "add": -20 } } ] },
    { "name": "steady_grip",
      "modifiers": [ { "attribute": "focus", "modifier": { "add": 10 } } ] }
  ],
  "kinds": [
    { "name": "ration", "tags": ["food", "crew"], "weight": "0.5", "stack": 6,
      "spoils_after_minutes": 1440 },
    { "name": "water", "tags": ["crew", "drink"], "weight": 1, "stack": 4 },
    { "name": "suit", "tags": ["gear"], "weight": 12, "slot": "suit", "affix_slots": 2,
      "modifiers": [ { "attribute": "oxygen", "modifier": { "add": 50 } } ] },
    { "name": "wrench", "tags": ["tool"], "weight": "1.5", "slot": "hand", "affix_slots": 1 }
  ]
}"#;

pub fn registry() -> Registry {
    Registry::from_json(REGISTRY).expect("the test registry is valid")
}

pub fn catalogue(registry: &Registry) -> Catalogue {
    Catalogue::from_json(CATALOGUE, registry).expect("the test catalogue is valid")
}

pub fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

pub fn kind(catalogue: &Catalogue, name: &str) -> KindId {
    catalogue.kind_id(name).expect("known kind")
}

pub fn attribute(registry: &Registry, name: &str) -> AttributeId {
    registry.id(name).expect("known attribute")
}

/// A bay: a rack, a cold store, and one crew member who wears things.
pub struct Bay {
    pub registry: Registry,
    pub catalogue: Catalogue,
    pub inventory: Inventory,
    pub attributes: Column<Attributes>,
    pub events: Vec<AttributeEvent>,
    pub rack: Handle,
    pub cold: Handle,
    pub crew: Handle,
}

impl Bay {
    pub fn new() -> Self {
        let registry = registry();
        let catalogue = catalogue(&registry);
        let mut entities = StableVector::new();
        let (rack, cold, crew) = (
            entities.insert(()),
            entities.insert(()),
            entities.insert(()),
        );
        let mut inventory = Inventory::new();
        inventory
            .add_container(rack, Container::new(4, Some(whole(30)), 18))
            .unwrap();
        inventory
            .add_container(cold, Container::new(2, None, 2))
            .unwrap();
        inventory.add_wearer(crew, &catalogue);
        let mut attributes = Column::new();
        attributes.set(crew, Attributes::from_registry(&registry));
        Bay {
            registry,
            catalogue,
            inventory,
            attributes,
            events: Vec::new(),
            rack,
            cold,
            crew,
        }
    }

    pub fn make(&mut self, name: &str, count: u16) -> Handle {
        let kind = kind(&self.catalogue, name);
        self.inventory.create(&self.catalogue, kind, count)
    }

    /// Makes an item and puts it in a container.
    pub fn stock(&mut self, name: &str, count: u16, into: Handle) -> Handle {
        let item = self.make(name, count);
        self.inventory.put(item, into, &self.catalogue).unwrap()
    }

    pub fn equip(&mut self, item: Handle) -> Result<SlotId, Refusal> {
        let mut wearers = Wearers {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.events,
        };
        self.inventory
            .equip(self.crew, item, &self.catalogue, &mut wearers)
    }

    pub fn unequip(&mut self, slot: &str) -> Result<Handle, Refusal> {
        let slot = self.catalogue.slot_id(slot).expect("known slot");
        let mut wearers = Wearers {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.events,
        };
        self.inventory
            .unequip(self.crew, slot, &self.catalogue, &mut wearers)
    }

    pub fn affix(&self, name: &str) -> AffixId {
        self.catalogue.affix_id(name).expect("known affix")
    }

    pub fn add_affix(&mut self, item: Handle, name: &str) -> Result<(), Refusal> {
        let affix = self.affix(name);
        let mut wearers = Wearers {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.events,
        };
        self.inventory
            .add_affix(item, affix, &self.catalogue, &mut wearers)
    }

    pub fn remove_affix(&mut self, item: Handle, name: &str) -> Result<(), Refusal> {
        let affix = self.affix(name);
        let mut wearers = Wearers {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.events,
        };
        self.inventory
            .remove_affix(item, affix, &self.catalogue, &mut wearers)
    }

    pub fn value(&self, name: &str) -> (Fixed32, Fixed32) {
        let value = self
            .attributes
            .get(self.crew)
            .unwrap()
            .get(attribute(&self.registry, name));
        (value.current(), value.maximum())
    }
}
