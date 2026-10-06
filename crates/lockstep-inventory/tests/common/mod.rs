// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::{AttributeId, Registry};
use lockstep_core::math::Fixed32;
use lockstep_inventory::{Catalogue, KindId};

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
