// SPDX-License-Identifier: Apache-2.0
use crate::common::{catalogue, fact, fragment, knower, question, rule};
use lockstep_core::{Clock, ClockConfiguration, Column, Handle, Message, Timeline};
use lockstep_knowledge::{
    evaluate, receive, FactId, Fragment, KnowledgeEvent, NoHistory, RuleKind,
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
    evaluate(&mut knowledge, &catalogue, &NoHistory, &mut events);
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
    knowledge.set(who, Default::default());
    let mut events = Vec::new();
    evaluate(&mut knowledge, &catalogue, &NoHistory, &mut events);
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
    evaluate(&mut knowledge, &catalogue, &NoHistory, &mut events);
    assert!(events.is_empty());
}

#[test]
fn happened_reads_the_timeline_for_the_knower() {
    let catalogue = catalogue();
    let mut store = lockstep_core::StableVector::new();
    let (account, bystander) = (store.insert(()), store.insert(()));
    let clock = Clock::new(
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 0,
            starting_day: 0,
        },
        1.0 / 30.0,
    );
    let mut timeline = Timeline::new();
    timeline.append(
        1,
        &clock,
        &[
            Ledger::Transfer { account },
            Ledger::Transfer { account },
            Ledger::Flagged { account },
            Ledger::Transfer { account: bystander },
        ],
    );
    let mut knowledge = Column::new();
    knowledge.set(account, Default::default());
    knowledge.set(bystander, Default::default());
    let busy = rule(&catalogue, "busy_account");
    let busy_fired = |events: &[KnowledgeEvent]| {
        events
            .iter()
            .any(|event| matches!(event, KnowledgeEvent::Fired { rule, .. } if *rule == busy))
    };
    let mut events = Vec::new();
    evaluate(&mut knowledge, &catalogue, &timeline, &mut events);
    assert!(!busy_fired(&events), "two transfers, not three");
    timeline.append(2, &clock, &[Ledger::Transfer { account }]);
    events.clear();
    evaluate(&mut knowledge, &catalogue, &timeline, &mut events);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, KnowledgeEvent::Fired { rule, .. } if *rule == busy))
            .collect::<Vec<_>>(),
        vec![&KnowledgeEvent::Fired {
            knower: account,
            rule: busy,
            kind: RuleKind::Achievement
        }]
    );
}

#[test]
fn a_fragment_of_a_fact_the_catalogue_lacks_is_ignored() {
    let catalogue = catalogue();
    let who = knower();
    let mut knowledge = Column::new();
    let mut events = Vec::new();
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
    assert!(knowledge.get(who).is_none());
}
