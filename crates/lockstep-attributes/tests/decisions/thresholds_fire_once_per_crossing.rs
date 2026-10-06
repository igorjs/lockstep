// SPDX-License-Identifier: Apache-2.0
//! Decision: a threshold fires once each time the current value passes it, in the order the value
//! passes them, and never while the value stays on one side.
//! Alternative rejected: firing while below a threshold, every change, which floods a status
//! display with repeats.
//! Would change if: hunger falling by 70 from 80 through 60, 40 and 20 emits anything but those
//! three events in that order, or small changes on one side of a threshold emit any.

use crate::common::{crossings, fresh, id, whole};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_drop_of_70_through_60_40_and_20_emits_exactly_three_ordered_events() {
    let (registry, mut attributes, who) = fresh();
    let hunger = id(&registry, "hunger");
    let mut events = Vec::new();
    attributes.apply(who, hunger, whole(-70), &registry, &mut events);
    assert_eq!(attributes.get(hunger).current(), whole(10));
    let expected = [("peckish", false), ("hungry", false), ("starving", false)];
    assert_eq!(
        crossings(&events),
        expected.map(|(name, upward)| (name.to_string(), upward))
    );

    // Hovering below a threshold says nothing more; climbing back fires it once, upward.
    events.clear();
    for _ in 0..5 {
        attributes.apply(who, hunger, whole(1), &registry, &mut events);
        attributes.apply(who, hunger, whole(-1), &registry, &mut events);
    }
    assert!(crossings(&events).is_empty());
    attributes.apply(who, hunger, whole(15), &registry, &mut events);
    assert_eq!(crossings(&events), [("starving".to_string(), true)]);
}
