// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_core::{Handle, StableVector};
use lockstep_meta::Templates;

pub const TEMPLATES: &str = r#"{
  "budget": 6,
  "attributes": [
    { "name": "stamina", "minimum": 1, "maximum": 8, "cost_per_point": 1 },
    { "name": "judgement", "minimum": 1, "maximum": 8, "cost_per_point": 2 }
  ],
  "archetypes": [
    { "name": "baker", "base": { "stamina": 4, "judgement": 2 } },
    { "name": "cashier", "base": { "stamina": 2, "judgement": 4 } }
  ]
}"#;

pub fn templates() -> Templates {
    Templates::from_json(TEMPLATES).expect("the test templates are valid")
}

/// Three entities.
pub fn three() -> (Handle, Handle, Handle) {
    let mut store = StableVector::new();
    (store.insert(()), store.insert(()), store.insert(()))
}
