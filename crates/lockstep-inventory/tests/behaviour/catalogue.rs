// SPDX-License-Identifier: Apache-2.0
use crate::common::{kind, registry, whole, CATALOGUE};
use lockstep_inventory::{Catalogue, CatalogueError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn read(text: &str) -> Result<Catalogue, CatalogueError> {
    Catalogue::from_json(text, &registry())
}

#[test]
fn a_catalogue_reads_kinds_slots_affixes_and_bands() {
    let catalogue = read(CATALOGUE).unwrap();
    let ration = catalogue.kind(kind(&catalogue, "ration"));
    assert_eq!(
        ration.weight,
        lockstep_core::math::Fixed32::from_ratio(1, 2)
    );
    assert_eq!(ration.stack, 6);
    assert_eq!(ration.tags, vec!["crew".to_string(), "food".to_string()]);
    assert!(ration.has_tag("food") && !ration.has_tag("tool"));
    let suit = catalogue.kind(kind(&catalogue, "suit"));
    assert_eq!(suit.slot, catalogue.slot_id("suit"));
    assert_eq!(
        suit.stack, 1,
        "a kind stacks to one unless it says otherwise"
    );
    assert_eq!(suit.weight, whole(12));
    assert!(
        catalogue
            .affix(catalogue.affix_id("faulty_seal").unwrap())
            .binds
    );
    assert_eq!(catalogue.spoilage_percent(-40), 25);
    assert_eq!(catalogue.spoilage_percent(4), 25);
    assert_eq!(catalogue.spoilage_percent(5), 100);
    assert_eq!(catalogue.spoilage_percent(29), 100);
    assert_eq!(catalogue.spoilage_percent(30), 300);
}

#[test]
fn a_bad_catalogue_is_refused_with_its_reason() {
    let band = r#""spoilage": [ { "percent": 100 } ]"#;
    let cases = [
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": 1 }}, {{ "name": "a", "weight": 1 }} ] }}"#),
            CatalogueError::DuplicateName("a".into()),
        ),
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": 1, "slot": "head" }} ] }}"#),
            CatalogueError::UnknownSlot {
                kind: "a".into(),
                slot: "head".into(),
            },
        ),
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": 1, "modifiers": [ {{ "attribute": "luck", "modifier": {{ "add": 1 }} }} ] }} ] }}"#),
            CatalogueError::UnknownAttribute {
                owner: "a".into(),
                attribute: "luck".into(),
            },
        ),
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": 1, "stack": 0 }} ] }}"#),
            CatalogueError::EmptyStack("a".into()),
        ),
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": "-0.5" }} ] }}"#),
            CatalogueError::NegativeWeight("a".into()),
        ),
        (
            format!(r#"{{ {band}, "kinds": [ {{ "name": "a", "weight": 1, "modifiers": [ {{ "attribute": "oxygen", "modifier": {{ "add": 1 }} }} ] }} ] }}"#),
            CatalogueError::ModifiersWithoutSlot("a".into()),
        ),
        (
            r#"{ "spoilage": [ { "below": 5, "percent": 25 } ], "kinds": [] }"#.to_string(),
            CatalogueError::Bands,
        ),
        (
            r#"{ "spoilage": [ { "below": 9, "percent": 25 }, { "below": 5, "percent": 50 }, { "percent": 100 } ], "kinds": [] }"#.to_string(),
            CatalogueError::Bands,
        ),
        (
            r#"{ "spoilage": [ { "percent": 100 }, { "percent": 50 } ], "kinds": [] }"#.to_string(),
            CatalogueError::Bands,
        ),
    ];
    for (text, error) in cases {
        assert_eq!(read(&text), Err(error), "{text}");
    }
    assert!(
        matches!(
            read(
                r#"{ "spoilage": [ { "percent": 100 } ], "kinds": [ { "name": "a", "weight": 0.5 } ] }"#
            ),
            Err(CatalogueError::Json(_))
        ),
        "JSON fractions are refused, as in the attribute registry"
    );
}
