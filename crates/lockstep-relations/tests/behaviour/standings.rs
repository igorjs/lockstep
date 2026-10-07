// SPDX-License-Identifier: Apache-2.0
use crate::common::{kinds, three, whole};
use lockstep_core::math::Fixed32;
use lockstep_relations::{RelationEvent, Relations, Target};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_change_is_held_within_bounds_and_crosses_thresholds_in_passing_order() {
    let kinds = kinds();
    let trust = kinds.relation_id("trust").unwrap();
    let (a, b, _) = three();
    let mut relations = Relations::new();
    let mut events = Vec::new();
    relations.change(&kinds, trust, a, Target::Entity(b), whole(-80), &mut events);
    let names: Vec<_> = events
        .iter()
        .map(
            |RelationEvent::Crossed {
                 threshold, upward, ..
             }| (threshold.as_str(), *upward),
        )
        .collect();
    assert_eq!(names, vec![("wary", false), ("hostile", false)]);
    relations.change(
        &kinds,
        trust,
        a,
        Target::Entity(b),
        whole(-80),
        &mut Vec::new(),
    );
    assert_eq!(
        relations.get(&kinds, trust, a, Target::Entity(b)),
        whole(-100)
    );
    events.clear();
    relations.change(&kinds, trust, a, Target::Entity(b), whole(160), &mut events);
    let names: Vec<_> = events
        .iter()
        .map(
            |RelationEvent::Crossed {
                 threshold, upward, ..
             }| (threshold.as_str(), *upward),
        )
        .collect();
    assert_eq!(
        names,
        vec![("hostile", true), ("wary", true), ("trusted", true)]
    );
}

#[test]
fn toward_adds_the_standing_toward_each_group_the_entity_belongs_to() {
    let kinds = kinds();
    let trust = kinds.relation_id("trust").unwrap();
    let night = kinds.group_id("night_shift").unwrap();
    let (dispatcher, driver, _) = three();
    let mut relations = Relations::new();
    let mut events = Vec::new();
    relations.change(
        &kinds,
        trust,
        dispatcher,
        Target::Entity(driver),
        whole(30),
        &mut events,
    );
    relations.change(
        &kinds,
        trust,
        dispatcher,
        Target::Group(night),
        whole(-10),
        &mut events,
    );
    assert_eq!(
        relations.toward(&kinds, trust, dispatcher, driver, &[night]),
        whole(20)
    );
    assert_eq!(
        relations.toward(&kinds, trust, dispatcher, driver, &[]),
        whole(30)
    );
    relations.change(
        &kinds,
        trust,
        dispatcher,
        Target::Group(night),
        whole(90),
        &mut events,
    );
    assert_eq!(
        relations.toward(&kinds, trust, dispatcher, driver, &[night]),
        whole(100),
        "held within bounds"
    );
}

#[test]
fn decay_stops_at_rest_from_either_side_and_a_change_restarts_it() {
    let kinds = kinds();
    let reputation = kinds.relation_id("reputation").unwrap();
    let (a, b, c) = three();
    let mut relations = Relations::new();
    let mut events = Vec::new();
    relations.change(
        &kinds,
        reputation,
        a,
        Target::Entity(b),
        whole(10),
        &mut events,
    );
    relations.change(
        &kinds,
        reputation,
        a,
        Target::Entity(c),
        whole(-40),
        &mut events,
    );
    // Reputation rests at 40, decaying 2.5 a day: from 60 and from 10, eight days.
    for _ in 0..8 {
        relations.tick(&kinds, whole(1_440), &mut events);
    }
    assert_eq!(
        relations.get(&kinds, reputation, a, Target::Entity(b)),
        whole(40)
    );
    assert_eq!(
        relations.get(&kinds, reputation, a, Target::Entity(c)),
        whole(30)
    );
    for _ in 0..8 {
        relations.tick(&kinds, whole(1_440), &mut events);
    }
    assert_eq!(
        relations.get(&kinds, reputation, a, Target::Entity(c)),
        whole(40)
    );
    relations.change(
        &kinds,
        reputation,
        a,
        Target::Entity(b),
        whole(20),
        &mut events,
    );
    relations.tick(&kinds, whole(720), &mut events);
    assert_eq!(
        relations.get(&kinds, reputation, a, Target::Entity(b)),
        whole(60) - Fixed32::from_ratio(5, 4)
    );
}

#[test]
fn forgetting_an_entity_drops_its_standings_both_ways() {
    let kinds = kinds();
    let trust = kinds.relation_id("trust").unwrap();
    let (a, b, c) = three();
    let mut relations = Relations::new();
    let mut events = Vec::new();
    relations.change(&kinds, trust, a, Target::Entity(b), whole(10), &mut events);
    relations.change(&kinds, trust, b, Target::Entity(c), whole(10), &mut events);
    relations.change(&kinds, trust, c, Target::Entity(a), whole(10), &mut events);
    relations.forget(b);
    assert_eq!(relations.get(&kinds, trust, a, Target::Entity(b)), whole(0));
    assert_eq!(relations.get(&kinds, trust, b, Target::Entity(c)), whole(0));
    assert_eq!(
        relations.get(&kinds, trust, c, Target::Entity(a)),
        whole(10)
    );
    let saved = lockstep_core::encode(&relations);
    let loaded: Relations = lockstep_core::decode(&saved).unwrap();
    assert_eq!(loaded, relations);
}
