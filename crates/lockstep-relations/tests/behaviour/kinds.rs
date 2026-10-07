// SPDX-License-Identifier: Apache-2.0
use crate::common::{kinds, whole};
use lockstep_core::math::Fixed32;
use lockstep_relations::{Kinds, KindsError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn kinds_read_relations_groups_and_sorted_thresholds() {
    let kinds = kinds();
    let trust = kinds.relation(kinds.relation_id("trust").unwrap()).unwrap();
    assert_eq!(trust.rest, whole(0), "rest defaults to starting");
    let names: Vec<_> = trust
        .thresholds
        .iter()
        .map(|threshold| threshold.name.as_str())
        .collect();
    assert_eq!(names, vec!["hostile", "wary", "trusted"]);
    let reputation = kinds
        .relation(kinds.relation_id("reputation").unwrap())
        .unwrap();
    assert_eq!(reputation.rest, whole(40));
    assert_eq!(reputation.decay_per_day, Fixed32::from_ratio(5, 2));
    assert_eq!(kinds.group_id("day_shift").map(|group| group.0), Some(1));
}

#[test]
fn bad_kinds_are_refused_with_their_reason() {
    let one = |fields: &str| {
        Kinds::from_json(&format!(
            r#"{{ "relations": [ {{ "name": "r", {fields} }} ] }}"#
        ))
    };
    assert_eq!(
        one(r#""minimum": 5, "maximum": 1, "starting": 3"#),
        Err(KindsError::Bounds("r".into()))
    );
    assert_eq!(
        one(r#""minimum": 0, "maximum": 10, "starting": 11"#),
        Err(KindsError::Bounds("r".into()))
    );
    assert_eq!(
        one(r#""minimum": 0, "maximum": 10, "starting": 5, "rest": -1"#),
        Err(KindsError::Bounds("r".into()))
    );
    assert_eq!(
        one(r#""minimum": 0, "maximum": 10, "starting": 5, "decay_per_day": -1"#),
        Err(KindsError::NegativeDecay("r".into()))
    );
    assert_eq!(
        one(
            r#""minimum": 0, "maximum": 10, "starting": 5, "thresholds": [ { "at": 20, "name": "high" } ]"#
        ),
        Err(KindsError::ThresholdOutside {
            relation: "r".into(),
            threshold: "high".into()
        })
    );
    assert_eq!(
        Kinds::from_json(r#"{ "relations": [], "groups": ["a", "a"] }"#),
        Err(KindsError::DuplicateName("a".into()))
    );
    assert!(matches!(Kinds::from_json("{}"), Err(KindsError::Json(_))));
}
