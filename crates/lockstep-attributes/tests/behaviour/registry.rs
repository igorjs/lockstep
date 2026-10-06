use crate::common::{registry, REGISTRY};
use lockstep_attributes::{Curve, MaximumPolicy, Registry, RegistryError};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn one(attribute: &str) -> String {
    format!(r#"{{ "attributes": [ {attribute} ] }}"#)
}

#[test]
fn the_json_form_reads_names_numbers_policies_and_curves() {
    let registry = registry();
    assert_eq!(registry.len(), 5);
    let health = registry.id("health").unwrap();
    assert_eq!(registry.name(health), "health");
    let definition = registry.definition(health);
    assert_eq!(definition.maximum, Fixed32::from_int(100));
    assert_eq!(definition.on_maximum_change, MaximumPolicy::ScaleCurrent);
    assert_eq!(
        registry
            .definition(registry.id("hunger").unwrap())
            .on_maximum_change,
        MaximumPolicy::Clamp
    );
    let critical = registry.definition(registry.id("critical_chance").unwrap());
    let derived = critical.derived.as_ref().unwrap();
    assert_eq!(derived.inputs, [registry.id("luck").unwrap()]);
    assert_eq!(
        derived.curve,
        Curve::Linear {
            per_point: Fixed32::ONE,
            offset: Fixed32::from_int(5)
        }
    );
    assert_eq!(registry.id("stamina"), None);
    assert!(REGISTRY.contains("\"1\""), "decimal strings are accepted");
}

#[test]
fn decimal_strings_are_exact_and_json_fractions_are_refused() {
    let exact = Registry::from_json(&one(
        r#"{ "name": "speed", "minimum": "-1.25", "maximum": "2.5", "starting": "0.125" }"#,
    ))
    .unwrap();
    let speed = exact.definition(exact.id("speed").unwrap());
    assert_eq!(
        (speed.minimum, speed.maximum, speed.starting),
        (
            Fixed32::from_ratio(-5, 4),
            Fixed32::from_ratio(5, 2),
            Fixed32::from_ratio(1, 8)
        )
    );
    let fraction = Registry::from_json(&one(
        r#"{ "name": "speed", "minimum": 0, "maximum": 2.5, "starting": 0 }"#,
    ));
    assert!(matches!(fraction, Err(RegistryError::Json(_))));
    let huge = Registry::from_json(&one(
        r#"{ "name": "speed", "minimum": 0, "maximum": 40000, "starting": 0 }"#,
    ));
    assert!(matches!(huge, Err(RegistryError::Json(_))));
}

#[test]
fn every_invalid_registry_is_refused_with_its_reason() {
    let check = |attributes: &str, expected: RegistryError| {
        let text = format!(r#"{{ "attributes": [ {attributes} ] }}"#);
        assert_eq!(Registry::from_json(&text), Err(expected), "{attributes}");
    };
    let plain = |name: &str| {
        format!(r#"{{ "name": "{name}", "minimum": 0, "maximum": 10, "starting": 5 }}"#)
    };
    check(
        &format!("{}, {}", plain("a"), plain("a")),
        RegistryError::DuplicateName("a".into()),
    );
    check(&plain(""), RegistryError::EmptyName);
    check(
        r#"{ "name": "a", "minimum": 0, "maximum": 10, "starting": 11 }"#,
        RegistryError::Bounds("a".into()),
    );
    check(
        r#"{ "name": "a", "minimum": 5, "maximum": 4, "starting": 5 }"#,
        RegistryError::Bounds("a".into()),
    );
    check(
        r#"{ "name": "a", "minimum": 0, "maximum": 10, "starting": 5, "thresholds": [ { "at": 11, "name": "over" } ] }"#,
        RegistryError::ThresholdOutside {
            attribute: "a".into(),
            threshold: "over".into(),
        },
    );
    check(
        r#"{ "name": "a", "minimum": 0, "maximum": 10, "starting": 0, "derived": { "inputs": ["b"], "curve": "sum" } }"#,
        RegistryError::UnknownInput {
            attribute: "a".into(),
            input: "b".into(),
        },
    );
    check(
        r#"{ "name": "a", "minimum": 0, "maximum": 10, "starting": 0, "derived": { "inputs": ["a"], "curve": "sum" } }"#,
        RegistryError::ReadsItself("a".into()),
    );
    check(
        &format!(
            r#"{}, {}, {{ "name": "c", "minimum": 0, "maximum": 10, "starting": 0, "derived": {{ "inputs": ["a", "b"], "curve": {{ "linear": {{ "per_point": 1, "offset": 0 }} }} }} }}"#,
            plain("a"),
            plain("b")
        ),
        RegistryError::InputCount("c".into()),
    );
    check(
        &format!(
            r#"{}, {{ "name": "c", "minimum": 0, "maximum": 10, "starting": 0, "derived": {{ "inputs": ["a"], "curve": {{ "piecewise": {{ "knots": [[5, 1], [5, 2]] }} }} }} }}"#,
            plain("a")
        ),
        RegistryError::Unsorted("c".into()),
    );
    check(
        &format!(
            r#"{}, {{ "name": "c", "minimum": 0, "maximum": 10, "starting": 0, "derived": {{ "inputs": ["a"], "curve": {{ "threshold": {{ "steps": [] }} }} }} }}"#,
            plain("a")
        ),
        RegistryError::Unsorted("c".into()),
    );
    assert!(matches!(
        Registry::from_json(r#"{ "attributes": [], "extra": 1 }"#),
        Err(RegistryError::Json(_))
    ));
}
