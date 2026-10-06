// SPDX-License-Identifier: Apache-2.0
use crate::common::{catalogue, fact, question, EVENT_KINDS};
use lockstep_knowledge::{Catalogue, CatalogueError, Predicate, RuleKind};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_catalogue_reads_facts_questions_and_rules_as_predicates() {
    let catalogue = catalogue();
    assert_eq!(catalogue.facts.len(), 3);
    assert_eq!(
        catalogue.questions[0].facts,
        vec![
            fact(&catalogue, "round_amounts"),
            fact(&catalogue, "night_transfers")
        ]
    );
    assert_eq!(catalogue.rules[2].kind, RuleKind::Achievement);
    assert_eq!(
        catalogue.rules[2].when,
        Predicate::Happened {
            kind: 0,
            at_least: 3
        }
    );
    assert_eq!(
        catalogue.rules[3].when,
        Predicate::Not(Box::new(Predicate::Any(vec![
            Predicate::Fragments {
                fact: fact(&catalogue, "round_amounts"),
                at_least: 1
            },
            Predicate::Opened(question(&catalogue, "who_is_behind_it")),
        ])))
    );
}

#[test]
fn a_bad_catalogue_is_refused_with_its_reason() {
    let read = |text: &str| Catalogue::from_json(text, EVENT_KINDS);
    let cases = [
        (
            r#"{ "facts": [ { "name": "a", "fragments": 1 }, { "name": "a", "fragments": 1 } ] }"#,
            CatalogueError::DuplicateName("a".into()),
        ),
        (
            r#"{ "facts": [ { "name": "a", "fragments": 0 } ] }"#,
            CatalogueError::NoFragments("a".into()),
        ),
        (
            r#"{ "facts": [], "questions": [ { "name": "q", "facts": ["x"], "fragments": 1 } ] }"#,
            CatalogueError::UnknownFact {
                owner: "q".into(),
                fact: "x".into(),
            },
        ),
        (
            r#"{ "facts": [], "rules": [ { "name": "r", "kind": "secret", "when": { "opened": "q" } } ] }"#,
            CatalogueError::UnknownQuestion {
                owner: "r".into(),
                question: "q".into(),
            },
        ),
        (
            r#"{ "facts": [], "rules": [ { "name": "r", "kind": "secret", "when": { "happened": { "event": "landing", "at_least": 1 } } } ] }"#,
            CatalogueError::UnknownEvent {
                owner: "r".into(),
                event: "landing".into(),
            },
        ),
        (
            r#"{ "facts": [], "rules": [ { "name": "r", "kind": "secret", "when": { "fired": "s" } }, { "name": "s", "kind": "secret", "when": { "all": [] } } ] }"#,
            CatalogueError::RuleOrder {
                owner: "r".into(),
                rule: "s".into(),
            },
        ),
        (
            r#"{ "facts": [], "rules": [ { "name": "r", "kind": "secret", "when": { "fired": "nobody" } } ] }"#,
            CatalogueError::UnknownRule {
                owner: "r".into(),
                rule: "nobody".into(),
            },
        ),
    ];
    for (text, error) in cases {
        assert_eq!(read(text), Err(error), "{text}");
    }
    assert!(matches!(
        read("{ \"facts\": 3 }"),
        Err(CatalogueError::Json(_))
    ));
}
