// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_core::math::Fixed32;
use lockstep_core::{Handle, StableVector};
use lockstep_relations::Kinds;

pub const KINDS: &str = r#"{
  "relations": [
    { "name": "trust", "minimum": -100, "maximum": 100, "starting": 0, "decay_per_day": 10,
      "thresholds": [ { "at": 50, "name": "trusted" }, { "at": -20, "name": "wary" },
                      { "at": -50, "name": "hostile" } ] },
    { "name": "reputation", "minimum": 0, "maximum": 100, "starting": 50, "rest": 40,
      "decay_per_day": "2.5" }
  ],
  "groups": ["night_shift", "day_shift"]
}"#;

pub fn kinds() -> Kinds {
    Kinds::from_json(KINDS).expect("the test kinds are valid")
}

pub fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

/// Three entities.
pub fn three() -> (Handle, Handle, Handle) {
    let mut store = StableVector::new();
    (store.insert(()), store.insert(()), store.insert(()))
}
