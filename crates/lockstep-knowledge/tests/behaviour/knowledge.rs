// SPDX-License-Identifier: Apache-2.0
use crate::common::{catalogue, fact, fragment, knower, question, rule};
use lockstep_core::{Column, Handle, Message};
use lockstep_knowledge::{
    enrol, evaluate, notice, receive, FactId, Fragment, KnowledgeEvent, RuleKind,
};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// A simulation's events, for the history: kind 0 is a transfer, 1 a flag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
enum Ledger {
    Transfer { account: Handle },
    Flagged { account: Handle },
}

#[test]
fn a_question_opens_after_two_fragments_across_its_facts() {
    let catalogue = catalogue();
    let who = knower();
    let mut knowledge = Column::new();
    lockstep_knowledge::enrol(&mut knowledge, who);
    let mut events = Vec::new();
    receive(
        &mut knowledge,
        who,
        fragment(&catalogue, "round_amounts", 1),
        &catalogue,
        &mut events,
    );
    assert!(events.is_empty(), "one fragment opens nothing");
    receive(
        &mut knowledge,
        who,
        fragment(&catalogue, "night_transfers", 2),
        &catalogue,
        &mut events,
    );
    assert_eq!(
        events,
        vec![KnowledgeEvent::Opened {
            knower: who,
            question: question(&catalogue, "who_is_behind_it")
        }]
    );
    // Neither fact is known yet: each needs two of its own.
    let mind = knowledge.get(who).unwrap();
    assert!(!mind.knows(fact(&catalogue, "round_amounts")));
}

#[test]
fn a_rule_naming_an_earlier_rule_fires_in_the_same_pass() {
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
        ("shared_address", 5),
    ] {
        receive(
            &mut knowledge,
            who,
            fragment(&catalogue, name, source),
            &catalogue,
            &mut events,
        );
    }
    events.clear();
    evaluate(&mut knowledge, &catalogue, &mut events);
    assert_eq!(
        events,
        vec![
            KnowledgeEvent::Fired {
                knower: who,
                rule: rule(&catalogue, "suspicion"),
                kind: RuleKind::Revelation
            },
            KnowledgeEvent::Fired {
                knower: who,
                rule: rule(&catalogue, "ring"),
                kind: RuleKind::Secret
            },
        ]
    );
}

#[test]
fn not_and_any_hold_for_a_knower_with_nothing_and_stop_once_evidence_arrives() {
    let catalogue = catalogue();
    let who = knower();
    let mut knowledge = Column::new();
    enrol(&mut knowledge, who);
    let mut events = Vec::new();
    evaluate(&mut knowledge, &catalogue, &mut events);
    assert_eq!(
        events,
        vec![KnowledgeEvent::Fired {
            knower: who,
            rule: rule(&catalogue, "clean"),
            kind: RuleKind::Achievement
        }]
    );
    // One fragment of round amounts and it would no longer hold, but it has already fired.
    receive(
        &mut knowledge,
        who,
        fragment(&catalogue, "round_amounts", 1),
        &catalogue,
        &mut events,
    );
    events.clear();
    evaluate(&mut knowledge, &catalogue, &mut events);
    assert!(events.is_empty());
}

#[test]
fn happened_counts_the_events_that_mentioned_the_knower() {
    let catalogue = catalogue();
    let mut store = lockstep_core::StableVector::new();
    let (account, bystander, stranger) = (store.insert(()), store.insert(()), store.insert(()));
    let mut knowledge = Column::new();
    enrol(&mut knowledge, account);
    enrol(&mut knowledge, bystander);
    notice(
        &mut knowledge,
        &[
            Ledger::Transfer { account },
            Ledger::Transfer { account },
            Ledger::Flagged { account },
            Ledger::Transfer { account: bystander },
            Ledger::Transfer { account: stranger },
        ],
    );
    assert_eq!(knowledge.get(account).unwrap().noticed(0), 2);
    assert!(
        knowledge.get(stranger).is_none(),
        "not enrolled, not counted"
    );
    let busy = rule(&catalogue, "busy_account");
    let busy_fired = |events: &[KnowledgeEvent]| {
        events
            .iter()
            .filter(|event| matches!(event, KnowledgeEvent::Fired { rule, .. } if *rule == busy))
            .cloned()
            .collect::<Vec<_>>()
    };
    let mut events = Vec::new();
    evaluate(&mut knowledge, &catalogue, &mut events);
    assert!(busy_fired(&events).is_empty(), "two transfers, not three");
    notice(&mut knowledge, &[Ledger::Transfer { account }]);
    // The counts live in the knowledge: through a save and load, a third transfer fires it.
    let saved = lockstep_core::encode(&knowledge);
    let mut knowledge: Column<lockstep_knowledge::Knowledge> =
        lockstep_core::decode(&saved).unwrap();
    events.clear();
    evaluate(&mut knowledge, &catalogue, &mut events);
    assert_eq!(
        busy_fired(&events),
        vec![KnowledgeEvent::Fired {
            knower: account,
            rule: busy,
            kind: RuleKind::Achievement
        }]
    );
}

#[test]
fn a_knower_not_enrolled_or_a_fact_the_catalogue_lacks_is_ignored() {
    let catalogue = catalogue();
    let mut store = lockstep_core::StableVector::new();
    let who = store.insert(());
    let mut knowledge = Column::new();
    let mut events = Vec::new();
    receive(
        &mut knowledge,
        who,
        fragment(&catalogue, "round_amounts", 1),
        &catalogue,
        &mut events,
    );
    assert!(knowledge.get(who).is_none(), "not enrolled");
    enrol(&mut knowledge, who);
    receive(
        &mut knowledge,
        who,
        Fragment {
            fact: FactId(99),
            source: 1,
        },
        &catalogue,
        &mut events,
    );
    assert!(events.is_empty());
    // A stale handle to the same slot changes nothing for the entity living there now.
    store.remove(who);
    let living = store.insert(());
    let mut living_knowledge = Column::new();
    enrol(&mut living_knowledge, living);
    receive(
        &mut living_knowledge,
        living,
        fragment(&catalogue, "shared_address", 1),
        &catalogue,
        &mut events,
    );
    let before = living_knowledge.get(living).cloned();
    receive(
        &mut living_knowledge,
        who,
        fragment(&catalogue, "round_amounts", 2),
        &catalogue,
        &mut events,
    );
    assert_eq!(living_knowledge.get(living).cloned(), before);
}
