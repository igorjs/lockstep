// SPDX-License-Identifier: Apache-2.0
//! Decision: a derived attribute is data (inputs and a curve), recomputed in registry order after
//! every change.
//! Alternative rejected: code that updates critical chance wherever Luck changes, which misses the
//! one change site nobody remembered.
//! Would change if: critical chance stops tracking Luck through a change, a relic's +2, and the
//! relic's removal.

use crate::common::{fresh, id, whole};
use lockstep_attributes::Modifier;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn critical_chance_tracks_luck_a_relic_and_its_removal() {
    let (registry, mut attributes, who) = fresh();
    let (luck, critical) = (id(&registry, "luck"), id(&registry, "critical_chance"));
    let mut events = Vec::new();
    // 1 percent per point of Luck, plus 5.
    assert_eq!(attributes.get(critical).current(), whole(8));
    attributes.apply(who, luck, whole(2), &registry, &mut events);
    assert_eq!(attributes.get(critical).current(), whole(10));

    // Luck scales with its maximum, so a relic adding 2 points of maximum to a full 20 of 20 is
    // felt in full.
    attributes.apply(who, luck, whole(15), &registry, &mut events);
    assert_eq!(attributes.get(luck).current(), whole(20));
    let relic = attributes.add_modifier(who, luck, Modifier::Add(whole(2)), &registry, &mut events);
    assert_eq!(attributes.get(luck).current(), whole(22));
    assert_eq!(attributes.get(critical).current(), whole(27));

    attributes.remove_modifier(who, luck, relic, &registry, &mut events);
    assert_eq!(attributes.get(critical).current(), whole(25));
}
