// SPDX-License-Identifier: Apache-2.0
//! Decision: each rule fires at most once per knower, the first time its predicate holds, and
//! the fired set is part of the saved state, so evaluating again, or after a save and load, never
//! fires it twice. The same holds for learning a fact and opening a question.
//! Alternative rejected: firing on each change into the true state, which fires again when a
//! condition drops and returns; and keeping fired rules outside the save, which fires them all
//! again after a load.
//! Would change if: a rule that holds for a hundred evaluations, then through a save and load,
//! fires more than once.

use crate::common::{catalogue, fragment, knower, rule};
use lockstep_core::Column;
use lockstep_knowledge::{evaluate, receive, Knowledge, KnowledgeEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_rule_that_keeps_holding_fires_once_even_across_a_save() {
    let catalogue = catalogue();
    let who = knower();
    let mut knowledge = Column::new();
    lockstep_knowledge::enrol(&mut knowledge, who);
    let mut events = Vec::new();
    for (name, source) in [
        ("round_amounts", 1),
        ("round_amounts", 2),
        ("night_transfers", 3),
        ("night_transfers", 4),
    ] {
        receive(
            &mut knowledge,
            who,
            fragment(&catalogue, name, source),
            &catalogue,
            &mut events,
        );
    }
    for _ in 0..100 {
        evaluate(&mut knowledge, &catalogue, &mut events);
    }
    let saved = lockstep_core::encode(&knowledge);
    let mut loaded: Column<Knowledge> = lockstep_core::decode(&saved).unwrap();
    evaluate(&mut loaded, &catalogue, &mut events);
    // A repeated fragment after the load opens and teaches nothing again.
    receive(
        &mut loaded,
        who,
        fragment(&catalogue, "round_amounts", 9),
        &catalogue,
        &mut events,
    );
    let suspicion = rule(&catalogue, "suspicion");
    let fired = events
        .iter()
        .filter(|event| matches!(event, KnowledgeEvent::Fired { rule, .. } if *rule == suspicion))
        .count();
    assert_eq!(fired, 1);
    let learned = events
        .iter()
        .filter(|event| matches!(event, KnowledgeEvent::Learned { .. }))
        .count();
    assert_eq!(learned, 2);
    let opened = events
        .iter()
        .filter(|event| matches!(event, KnowledgeEvent::Opened { .. }))
        .count();
    assert_eq!(opened, 1);
}
