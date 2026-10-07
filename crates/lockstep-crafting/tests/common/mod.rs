// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::Registry;
use lockstep_core::{Handle, StableVector};
use lockstep_crafting::{Crafting, Recipes};
use lockstep_inventory::{Catalogue, Container, Inventory};

pub const CATALOGUE: &str = r#"{
  "spoilage": [ { "percent": 100 } ],
  "kinds": [
    { "name": "flour", "weight": 1, "stack": 1, "spoils_after_minutes": 600 },
    { "name": "water", "weight": 1, "stack": 10 },
    { "name": "dough", "weight": 1, "stack": 4 },
    { "name": "bread", "weight": 1, "stack": 8 },
    { "name": "charcoal", "weight": 1, "stack": 8 }
  ]
}"#;

pub const RECIPES: &str = r#"{
  "stations": ["mixer", "oven"],
  "recipes": [
    { "name": "dough", "station": "mixer", "minutes": 10,
      "inputs": [ { "kind": "flour", "count": 2 }, { "kind": "water", "count": 1 } ],
      "outcomes": [ { "name": "mixed", "weight": 1, "outputs": [ { "kind": "dough", "count": 1 } ] } ] },
    { "name": "bread", "station": "oven", "minutes": 30,
      "inputs": [ { "kind": "dough", "count": 1 } ],
      "outcomes": [
        { "name": "good", "weight": 85, "outputs": [ { "kind": "bread", "count": 2 } ] },
        { "name": "burnt", "weight": 10, "outputs": [ { "kind": "charcoal", "count": 1 } ] },
        { "name": "collapsed", "weight": 5 } ] },
    { "name": "double", "station": "mixer", "minutes": 5,
      "inputs": [ { "kind": "flour", "count": 1 }, { "kind": "flour", "count": 1 } ],
      "outcomes": [ { "name": "mixed", "weight": 1, "outputs": [ { "kind": "dough", "count": 1 } ] } ] }
  ]
}"#;

/// A pantry, a mixer and an oven, each with a container.
pub struct Bakery {
    pub catalogue: Catalogue,
    pub recipes: Recipes,
    pub inventory: Inventory,
    pub crafting: Crafting,
    pub pantry: Handle,
    pub mixer: Handle,
    pub oven: Handle,
}

impl Bakery {
    pub fn new(oven_slots: u16) -> Self {
        let registry = Registry::from_json(r#"{ "attributes": [] }"#).unwrap();
        let catalogue = Catalogue::from_json(CATALOGUE, &registry).unwrap();
        let recipes = Recipes::from_json(RECIPES, &catalogue).unwrap();
        let mut entities = StableVector::new();
        let (pantry, mixer, oven) = (
            entities.insert(()),
            entities.insert(()),
            entities.insert(()),
        );
        let mut inventory = Inventory::new();
        inventory
            .add_container(pantry, Container::new(20, None, 18))
            .unwrap();
        inventory
            .add_container(mixer, Container::new(4, None, 18))
            .unwrap();
        inventory
            .add_container(oven, Container::new(oven_slots, None, 18))
            .unwrap();
        let mut crafting = Crafting::new();
        crafting.add_station(mixer, recipes.station_id("mixer").unwrap());
        crafting.add_station(oven, recipes.station_id("oven").unwrap());
        Bakery {
            catalogue,
            recipes,
            inventory,
            crafting,
            pantry,
            mixer,
            oven,
        }
    }

    /// Puts units of a kind into a container, one item per stack.
    pub fn stock(&mut self, name: &str, count: u16, into: Handle) {
        let kind = self.catalogue.kind_id(name).unwrap();
        let stack = self.catalogue.kind(kind).stack;
        let mut left = count;
        while left > 0 {
            let units = left.min(stack);
            let item = self.inventory.create(&self.catalogue, kind, units);
            self.inventory.put(item, into, &self.catalogue).unwrap();
            left -= units;
        }
    }

    pub fn available(&self, name: &str, container: Handle) -> u32 {
        Crafting::available(
            &self.inventory,
            container,
            self.catalogue.kind_id(name).unwrap(),
        )
    }
}
