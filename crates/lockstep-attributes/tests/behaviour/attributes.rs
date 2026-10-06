// SPDX-License-Identifier: Apache-2.0
use crate::common::{crossings, fresh, id, someone, whole};
use lockstep_attributes::{AttributeEvent, Attributes, Modifier};
use lockstep_core::hash_of;
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn every_attribute_starts_at_its_starting_value() {
    let (registry, attributes, _) = fresh();
    for (name, starting) in [
        ("health", 50),
        ("hunger", 80),
        ("luck", 3),
        ("corruption", 0),
    ] {
        let attribute = attributes.get(id(&registry, name));
        assert_eq!(attribute.current(), whole(starting), "{name}");
        assert_eq!(attribute.minimum(), whole(0));
    }
    assert_eq!(
        attributes.get(id(&registry, "critical_chance")).current(),
        whole(8)
    );
}

#[test]
fn a_change_clamps_and_reports_emptied_and_filled_once() {
    let (registry, mut attributes, who) = fresh();
    let health = id(&registry, "health");
    let mut events = Vec::new();
    attributes.apply(who, health, whole(500), &registry, &mut events);
    assert_eq!(attributes.get(health).current(), whole(100));
    assert_eq!(
        events,
        [AttributeEvent::Filled {
            who,
            attribute: health
        }]
    );
    events.clear();
    attributes.apply(who, health, whole(5), &registry, &mut events);
    assert!(events.is_empty(), "already full");
    attributes.apply(who, health, whole(-1_000), &registry, &mut events);
    assert_eq!(attributes.get(health).current(), whole(0));
    assert_eq!(
        events,
        [
            AttributeEvent::Crossed {
                who,
                attribute: health,
                threshold: "wounded".into(),
                upward: false
            },
            AttributeEvent::Emptied {
                who,
                attribute: health
            },
        ]
    );
    events.clear();
    attributes.apply(who, health, whole(-1), &registry, &mut events);
    assert!(events.is_empty(), "already empty");
}

#[test]
fn a_lowered_maximum_that_meets_the_current_value_fills_it() {
    let (registry, mut attributes, who) = fresh();
    let hunger = id(&registry, "hunger");
    let mut events = Vec::new();
    attributes.add_modifier(
        who,
        hunger,
        Modifier::Override(whole(70)),
        &registry,
        &mut events,
    );
    assert_eq!(attributes.get(hunger).current(), whole(70));
    assert_eq!(
        events,
        [AttributeEvent::Filled {
            who,
            attribute: hunger
        }]
    );
}

#[test]
fn a_modifier_is_removed_only_by_its_own_handle_and_only_once() {
    let (registry, mut attributes, who) = fresh();
    let (hunger, health) = (id(&registry, "hunger"), id(&registry, "health"));
    let mut events = Vec::new();
    let first = attributes.add_modifier(
        who,
        hunger,
        Modifier::Add(whole(10)),
        &registry,
        &mut events,
    );
    let second = attributes.add_modifier(
        who,
        hunger,
        Modifier::Add(whole(10)),
        &registry,
        &mut events,
    );
    assert_ne!(first, second);
    assert!(
        !attributes.remove_modifier(who, health, first, &registry, &mut events),
        "wrong attribute"
    );
    assert!(attributes.remove_modifier(who, hunger, first, &registry, &mut events));
    assert!(
        !attributes.remove_modifier(who, hunger, first, &registry, &mut events),
        "already removed"
    );
    assert_eq!(attributes.get(hunger).maximum(), whole(110));
    let held: Vec<_> = attributes
        .get(hunger)
        .modifiers()
        .map(|(handle, _)| handle)
        .collect();
    assert_eq!(held, [second]);
}

#[test]
fn the_fraction_runs_from_the_minimum_to_the_maximum() {
    let (registry, mut attributes, who) = fresh();
    let health = id(&registry, "health");
    assert_eq!(attributes.get(health).fraction(), Fixed32::HALF);
    attributes.apply(who, health, whole(-25), &registry, &mut Vec::new());
    assert_eq!(attributes.get(health).fraction(), Fixed32::from_ratio(1, 4));
}

#[test]
fn a_derived_attribute_ignores_apply_and_reports_its_own_crossings() {
    let (registry, mut attributes, who) = fresh();
    let (luck, critical) = (id(&registry, "luck"), id(&registry, "critical_chance"));
    let mut events = Vec::new();
    attributes.apply(who, critical, whole(50), &registry, &mut events);
    assert_eq!(attributes.get(critical).current(), whole(8));
    assert!(events.is_empty());
    attributes.apply(who, luck, whole(2), &registry, &mut events);
    assert_eq!(
        events,
        [AttributeEvent::Crossed {
            who,
            attribute: critical,
            threshold: "keen".into(),
            upward: true
        }]
    );
}

#[test]
fn a_derived_attribute_with_scale_current_reports_nothing_for_a_modifier_it_does_not_feel() {
    let registry = lockstep_attributes::Registry::from_json(
        r#"{ "attributes": [
            { "name": "focus", "minimum": 0, "maximum": 100, "starting": 40 },
            { "name": "insight", "minimum": 0, "maximum": 100, "starting": 0,
              "on_maximum_change": "scale_current",
              "thresholds": [ { "at": 50, "name": "clear" } ],
              "derived": { "inputs": ["focus"], "curve": "sum" } }
        ] }"#,
    )
    .unwrap();
    let insight = registry.id("insight").unwrap();
    let mut attributes = Attributes::from_registry(&registry);
    let who = someone();
    let mut events = Vec::new();
    attributes.add_modifier(
        who,
        insight,
        Modifier::Multiply(whole(2)),
        &registry,
        &mut events,
    );
    assert_eq!(attributes.get(insight).current(), whole(40));
    assert_eq!(attributes.get(insight).maximum(), whole(200));
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn a_derived_value_is_clamped_to_its_own_bounds() {
    let (registry, mut attributes, who) = fresh();
    let (luck, critical) = (id(&registry, "luck"), id(&registry, "critical_chance"));
    attributes.add_modifier(
        who,
        luck,
        Modifier::Override(whole(500)),
        &registry,
        &mut Vec::new(),
    );
    attributes.apply(who, luck, whole(500), &registry, &mut Vec::new());
    assert_eq!(attributes.get(luck).current(), whole(500));
    assert_eq!(
        attributes.get(critical).current(),
        whole(100),
        "5 plus 500, held at 100"
    );
}

#[test]
fn events_name_the_entity_they_happened_to() {
    let (registry, _, _) = fresh();
    let mut entities = lockstep_core::StableVector::new();
    let (first, second) = (entities.insert(()), entities.insert(()));
    assert_ne!(first, second);
    let mut events = Vec::new();
    for who in [first, second] {
        let mut attributes = Attributes::from_registry(&registry);
        attributes.apply(
            who,
            id(&registry, "hunger"),
            whole(-30),
            &registry,
            &mut events,
        );
    }
    let named: Vec<_> = events
        .iter()
        .map(|event| match event {
            AttributeEvent::Crossed { who, .. } => *who,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(named, [first, second]);
    assert_eq!(crossings(&events).len(), 2);
}

#[test]
fn a_saved_set_of_attributes_continues_exactly_like_the_original() {
    let (registry, mut original, who) = fresh();
    let (hunger, luck) = (id(&registry, "hunger"), id(&registry, "luck"));
    let mut events = Vec::new();
    let relic = original.add_modifier(who, luck, Modifier::Add(whole(2)), &registry, &mut events);
    original.apply(who, hunger, whole(-33), &registry, &mut events);
    let mut restored: Attributes =
        bincode::deserialize(&bincode::serialize(&original).unwrap()).unwrap();
    assert_eq!(restored, original);
    let (mut original_events, mut restored_events) = (Vec::new(), Vec::new());
    for attributes_and_events in [
        (&mut original, &mut original_events),
        (&mut restored, &mut restored_events),
    ] {
        let (attributes, events) = attributes_and_events;
        attributes.remove_modifier(who, luck, relic, &registry, events);
        attributes.add_modifier(
            who,
            hunger,
            Modifier::Multiply(Fixed32::HALF),
            &registry,
            events,
        );
        attributes.apply(who, hunger, whole(-20), &registry, events);
    }
    assert_eq!(hash_of(&restored), hash_of(&original));
    assert_eq!(restored_events, original_events);
}
