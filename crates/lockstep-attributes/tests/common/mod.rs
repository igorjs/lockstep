// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::{AttributeId, Registry};
use lockstep_core::math::Fixed32;

pub const REGISTRY: &str = r#"{
  "attributes": [
    { "name": "health", "minimum": 0, "maximum": 100, "starting": 50,
      "on_maximum_change": "scale_current",
      "thresholds": [ { "at": 20, "name": "wounded" } ] },
    { "name": "hunger", "minimum": 0, "maximum": 100, "starting": 80,
      "thresholds": [ { "at": 60, "name": "peckish" }, { "at": 40, "name": "hungry" },
                      { "at": 20, "name": "starving" }, { "at": 5, "name": "dying" } ] },
    { "name": "luck", "minimum": 0, "maximum": 20, "starting": 3,
      "on_maximum_change": "scale_current" },
    { "name": "critical_chance", "minimum": 0, "maximum": 100, "starting": 0,
      "derived": { "inputs": ["luck"], "curve": { "linear": { "per_point": "1", "offset": "5" } } } },
    { "name": "corruption", "minimum": 0, "maximum": 100, "starting": 0,
      "on_maximum_change": "ratchet",
      "thresholds": [ { "at": 25, "name": "tainted" }, { "at": 50, "name": "marked" },
                      { "at": 75, "name": "lost" } ] }
  ]
}"#;

pub fn registry() -> Registry {
    Registry::from_json(REGISTRY).expect("the test registry is valid")
}

pub fn id(registry: &Registry, name: &str) -> AttributeId {
    registry.id(name).expect("known attribute")
}

pub fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}
