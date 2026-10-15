// SPDX-License-Identifier: Apache-2.0
use crate::common::{calm, hours, id, regions, sure};
use lockstep_core::math::Fixed32;
use lockstep_core::{decode, encode, Streams};
use lockstep_regions::{Condition, Spread, SpreadEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_sure_spread_reaches_one_more_link_each_hour() {
    let regions = regions();
    let rules = sure();
    let mut spread = Spread::new(&regions);
    let mut streams = Streams::new(1);
    spread.start(id(&regions, "a"), &rules, &mut Vec::new());
    let mut reached = Vec::new();
    for _ in 0..3 {
        hours(&mut spread, &regions, &rules, calm(), 100, 1, &mut streams);
        reached.push(
            ["a", "b", "c", "d"]
                .into_iter()
                .filter(|name| spread.condition(id(&regions, name)) != Some(Condition::Clear))
                .count(),
        );
    }
    // a; then b joins; the next hour adds c; then d. (Branch is half receptive, so left out.)
    assert_eq!(reached, vec![2, 3, 4]);
    assert_eq!(
        spread.condition(id(&regions, "island")),
        Some(Condition::Clear)
    );
}

#[test]
fn a_region_is_active_for_its_intervals_then_spent_and_never_starts_again() {
    let regions = regions();
    let rules = sure();
    let mut spread = Spread::new(&regions);
    let mut streams = Streams::new(1);
    let a = id(&regions, "a");
    spread.start(a, &rules, &mut Vec::new());
    let first = hours(&mut spread, &regions, &rules, calm(), 100, 2, &mut streams);
    assert_eq!(
        spread.condition(a),
        Some(Condition::Active { intervals_left: 1 })
    );
    assert!(!first.contains(&SpreadEvent::Spent { region: a }));
    let third = hours(&mut spread, &regions, &rules, calm(), 100, 1, &mut streams);
    assert_eq!(spread.condition(a), Some(Condition::Spent));
    assert!(third.contains(&SpreadEvent::Spent { region: a }));
    // The neighbour b is spent later and cannot restart a.
    hours(&mut spread, &regions, &rules, calm(), 100, 12, &mut streams);
    assert_eq!(spread.condition(a), Some(Condition::Spent));
    assert_eq!(spread.active_count(), 0);
    assert!(spread.spent_count() >= 4);
}

#[test]
fn starting_names_the_source_and_only_a_clear_known_region_starts() {
    let regions = regions();
    let rules = sure();
    let mut spread = Spread::new(&regions);
    let mut events = Vec::new();
    let a = id(&regions, "a");
    assert!(spread.start(a, &rules, &mut events));
    assert!(!spread.start(a, &rules, &mut events));
    assert!(!spread.start(lockstep_regions::RegionId(99), &rules, &mut events));
    let after = hours(
        &mut spread,
        &regions,
        &rules,
        calm(),
        100,
        1,
        &mut Streams::new(1),
    );
    let b = id(&regions, "b");
    assert_eq!(
        events,
        vec![SpreadEvent::Started {
            region: a,
            from: None
        }]
    );
    assert!(after.contains(&SpreadEvent::Started {
        region: b,
        from: Some(a)
    }));
}

#[test]
fn a_pace_of_nothing_stops_the_spread_and_part_of_a_receptive_region_slows_it() {
    let regions = regions();
    let rules = sure();
    let mut spread = Spread::new(&regions);
    spread.start(id(&regions, "a"), &rules, &mut Vec::new());
    hours(
        &mut spread,
        &regions,
        &rules,
        calm(),
        0,
        2,
        &mut Streams::new(1),
    );
    assert_eq!(spread.active_count(), 1);
}

#[test]
fn a_partial_hour_is_carried_to_the_next_step() {
    let regions = regions();
    let rules = sure();
    let mut spread = Spread::new(&regions);
    let mut streams = Streams::new(1);
    spread.start(id(&regions, "a"), &rules, &mut Vec::new());
    let mut events = Vec::new();
    for _ in 0..3 {
        spread.advance(
            &regions,
            &rules,
            calm(),
            100,
            Fixed32::from_int(20),
            &mut streams,
            &mut events,
        );
    }
    assert!(spread.is_active(id(&regions, "b")));
}

#[test]
fn a_saved_spread_continues_exactly() {
    let regions = regions();
    let rules = crate::common::sure();
    let rules = lockstep_regions::SpreadRules {
        base_permille: 400,
        ..rules
    };
    let mut spread = Spread::new(&regions);
    let mut streams = Streams::new(5);
    spread.start(id(&regions, "a"), &rules, &mut Vec::new());
    hours(&mut spread, &regions, &rules, calm(), 100, 3, &mut streams);
    let mut restored: Spread = decode(&encode(&spread)).unwrap();
    assert_eq!(restored, spread);
    let mut streams_after = streams.clone();
    let live = hours(&mut spread, &regions, &rules, calm(), 100, 6, &mut streams);
    let again = hours(
        &mut restored,
        &regions,
        &rules,
        calm(),
        100,
        6,
        &mut streams_after,
    );
    assert_eq!(restored, spread);
    assert_eq!(live, again);
}

#[test]
fn a_spread_made_for_other_regions_does_nothing_instead_of_panicking() {
    let regions = regions();
    let rules = sure();
    let small =
        lockstep_regions::Regions::from_json(r#"{"regions":[{"name":"only","x":0,"y":0}]}"#)
            .unwrap();
    let mut spread = Spread::new(&small);
    spread.start(lockstep_regions::RegionId(0), &rules, &mut Vec::new());
    let events = hours(
        &mut spread,
        &regions,
        &rules,
        calm(),
        100,
        3,
        &mut Streams::new(1),
    );
    assert!(events.is_empty());
    assert_eq!(spread.active_count(), 1);
}
