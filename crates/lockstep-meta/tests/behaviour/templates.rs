// SPDX-License-Identifier: Apache-2.0
use crate::common::templates;
use lockstep_meta::{TemplateError, Templates};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn pairs(values: &[(&str, i32)]) -> Vec<(String, i32)> {
    values
        .iter()
        .map(|(name, value)| (name.to_string(), *value))
        .collect()
}

#[test]
fn an_archetype_without_purchases_is_its_base() {
    assert_eq!(
        templates().build("baker", &[]),
        Ok(pairs(&[("stamina", 4), ("judgement", 2)]))
    );
}

#[test]
fn purchases_raise_the_base_and_repeats_add_up() {
    assert_eq!(
        templates().build(
            "cashier",
            &[("stamina", 2), ("stamina", 1), ("judgement", 1)]
        ),
        Ok(pairs(&[("stamina", 5), ("judgement", 5)]))
    );
}

#[test]
fn the_budget_is_exact() {
    // Four stamina points and one judgement point cost 4 + 2 = 6, the whole budget.
    assert!(templates()
        .build("baker", &[("stamina", 4), ("judgement", 1)])
        .is_ok());
    assert_eq!(
        templates().build("baker", &[("stamina", 5), ("judgement", 1)]),
        Err(TemplateError::AboveMaximum("stamina".into()))
    );
    assert_eq!(
        templates().build("baker", &[("stamina", 3), ("judgement", 2)]),
        Err(TemplateError::OverBudget { cost: 7, budget: 6 })
    );
}

#[test]
fn nothing_passes_its_maximum() {
    assert_eq!(
        templates().build("baker", &[("stamina", 5)]),
        Err(TemplateError::AboveMaximum("stamina".into()))
    );
}

#[test]
fn unknown_names_are_refused() {
    assert_eq!(
        templates().build("chef", &[]),
        Err(TemplateError::UnknownArchetype("chef".into()))
    );
    assert_eq!(
        templates().build("baker", &[("luck", 1)]),
        Err(TemplateError::UnknownAttribute("luck".into()))
    );
}

const STAMINA: &str = r#"{ "name": "stamina", "minimum": 1, "maximum": 8, "cost_per_point": 1 }"#;

fn read(attributes: &str, archetypes: &str) -> Result<Templates, TemplateError> {
    Templates::from_json(&format!(
        r#"{{ "budget": 3, "attributes": [ {attributes} ], "archetypes": [ {archetypes} ] }}"#
    ))
}

#[test]
fn bad_data_is_refused_with_a_reason() {
    assert!(matches!(
        Templates::from_json("nope"),
        Err(TemplateError::Json(_))
    ));
    assert_eq!(
        read(&format!("{STAMINA}, {STAMINA}"), ""),
        Err(TemplateError::DuplicateName("stamina".into()))
    );
    for rule in [
        r#"{ "name": "x", "minimum": 5, "maximum": 1, "cost_per_point": 1 }"#,
        r#"{ "name": "x", "minimum": 1, "maximum": 5, "cost_per_point": 0 }"#,
    ] {
        assert_eq!(read(rule, ""), Err(TemplateError::Rule("x".into())));
    }
    for archetype in [
        r#"{ "name": "a", "base": { "stamina": 9 } }"#,
        r#"{ "name": "a", "base": {} }"#,
        r#"{ "name": "a", "base": { "luck": 1 } }"#,
    ] {
        assert!(matches!(
            read(STAMINA, archetype),
            Err(TemplateError::Archetype(_))
        ));
    }
    let twice =
        r#"{ "name": "a", "base": { "stamina": 1 } }, { "name": "a", "base": { "stamina": 2 } }"#;
    assert_eq!(
        read(STAMINA, twice),
        Err(TemplateError::DuplicateName("a".into()))
    );
}
