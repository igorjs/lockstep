// SPDX-License-Identifier: Apache-2.0
//! Decision: a fact is known after fragments from distinct sources; the same source again counts
//! once, so repeating one rumour never proves anything.
//! Alternative rejected: counting every fragment, which lets one source repeated make anything
//! known.
//! Would change if: two fragments of a two-fragment fact from one source make it known.

use crate::common::{catalogue, fact, fragment, knower};
use lockstep_core::Column;
use lockstep_knowledge::receive;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_same_source_twice_is_one_fragment() {
    let catalogue = catalogue();
    let who = knower();
    let mut knowledge = Column::new();
    let mut events = Vec::new();
    let once = fragment(&catalogue, "round_amounts", 7);
    receive(&mut knowledge, who, once, &catalogue, &mut events);
    receive(&mut knowledge, who, once, &catalogue, &mut events);
    let mind = knowledge.get(who).unwrap();
    assert_eq!(mind.fragments_of(fact(&catalogue, "round_amounts")), 1);
    assert!(!mind.knows(fact(&catalogue, "round_amounts")));
    assert!(events.is_empty());
    receive(
        &mut knowledge,
        who,
        fragment(&catalogue, "round_amounts", 8),
        &catalogue,
        &mut events,
    );
    assert!(knowledge
        .get(who)
        .unwrap()
        .knows(fact(&catalogue, "round_amounts")));
}
