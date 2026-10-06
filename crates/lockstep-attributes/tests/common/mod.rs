// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{Handle, StableVector};

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
      "thresholds": [ { "at": 10, "name": "keen" } ],
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

/// A live handle to stand for the entity.
pub fn someone() -> Handle {
    StableVector::new().insert(())
}

pub fn fresh() -> (Registry, Attributes, Handle) {
    let registry = registry();
    let attributes = Attributes::from_registry(&registry);
    (registry, attributes, someone())
}

pub fn crossings(events: &[AttributeEvent]) -> Vec<(String, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            AttributeEvent::Crossed {
                threshold, upward, ..
            } => Some((threshold.clone(), *upward)),
            _ => None,
        })
        .collect()
}

/// One entity's attributes in a column, ready for effects.
pub fn column_with(registry: &Registry, who: Handle) -> lockstep_core::Column<Attributes> {
    let mut column = lockstep_core::Column::new();
    column.set(who, Attributes::from_registry(registry));
    column
}

pub fn bleeding(registry: &Registry, minutes: Option<i32>) -> lockstep_attributes::Effect {
    lockstep_attributes::Effect {
        attribute: id(registry, "health"),
        modifier: None,
        per_minute: Some(whole(-2)),
        remaining_minutes: minutes.map(whole),
        tag: lockstep_attributes::EffectTag::new("bleeding"),
        stacking: lockstep_attributes::Stacking::Independent,
    }
}
