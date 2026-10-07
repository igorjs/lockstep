// SPDX-License-Identifier: Apache-2.0
//! Decision: a gate is an attribute at or above a value, checked when the node is taken; an
//! attribute that later falls takes nothing away.
//! Alternative rejected: gates checked continuously, which would strip a node and its modifiers
//! the moment experience dips, and a rule's effects would flicker.
//! Would change if: senior (experience at least 50) can be taken at 49, cannot be taken at 50,
//! or is lost when experience falls back to 10.

use crate::common::Crew;
use lockstep_progression::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn senior_needs_fifty_experience_when_taken_and_keeps_it_after() {
    let mut crew = Crew::new(20);
    crew.unlock("basics").unwrap();
    crew.unlock("diagnostics").unwrap();
    let experience = crew.registry.id("experience").unwrap();
    crew.set("experience", 49);
    assert_eq!(crew.unlock("senior"), Err(Refusal::Gate(experience)));
    crew.set("experience", 50);
    crew.unlock("senior").unwrap();
    crew.set("experience", 10);
    assert!(crew
        .progress
        .get(crew.who)
        .unwrap()
        .has_taken(crew.node("senior")));
}
