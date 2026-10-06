// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_core::{Handle, StableVector};
use lockstep_knowledge::{Catalogue, FactId, Fragment, QuestionId, RuleId};

/// Event kinds a simulation would name for `happened`.
pub const EVENT_KINDS: &[(&str, u16)] = &[("transfer", 0), ("flagged", 1)];

pub const CATALOGUE: &str = r#"{
  "facts": [
    { "name": "round_amounts", "fragments": 2 },
    { "name": "night_transfers", "fragments": 2 },
    { "name": "shared_address", "fragments": 1 }
  ],
  "questions": [
    { "name": "who_is_behind_it", "facts": ["round_amounts", "night_transfers"], "fragments": 2 }
  ],
  "rules": [
    { "name": "suspicion", "kind": "revelation",
      "when": { "all": [ { "known": "round_amounts" }, { "known": "night_transfers" } ] } },
    { "name": "ring", "kind": "secret",
      "when": { "all": [ { "fired": "suspicion" }, { "known": "shared_address" } ] } },
    { "name": "busy_account", "kind": "achievement",
      "when": { "happened": { "event": "transfer", "at_least": 3 } } },
    { "name": "clean", "kind": "achievement",
      "when": { "not": { "any": [ { "fragments": { "fact": "round_amounts", "at_least": 1 } },
                                  { "opened": "who_is_behind_it" } ] } } }
  ]
}"#;

pub fn catalogue() -> Catalogue {
    Catalogue::from_json(CATALOGUE, EVENT_KINDS).expect("the test catalogue is valid")
}

pub fn knower() -> Handle {
    StableVector::new().insert(())
}

pub fn fact(catalogue: &Catalogue, name: &str) -> FactId {
    catalogue.fact_id(name).expect("known fact")
}

pub fn question(catalogue: &Catalogue, name: &str) -> QuestionId {
    catalogue.question_id(name).expect("known question")
}

pub fn rule(catalogue: &Catalogue, name: &str) -> RuleId {
    catalogue.rule_id(name).expect("known rule")
}

pub fn fragment(catalogue: &Catalogue, name: &str, source: u32) -> Fragment {
    Fragment {
        fact: fact(catalogue, name),
        source,
    }
}
