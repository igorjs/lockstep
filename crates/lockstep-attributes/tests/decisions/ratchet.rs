// SPDX-License-Identifier: Apache-2.0
//! Decision: a ratchet attribute turns each threshold its value rises through into its new
//! minimum, so corruption has points of no return.
//! Alternative rejected: a separate "highest value reached" field each consumer checks, which a
//! cleansing effect would have to know to respect.
//! Would change if: corruption's minimum ever falls once raised, by any change or modifier.

use crate::common::{fresh, id, whole};
use lockstep_attributes::Modifier;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn corruptions_minimum_never_falls_once_raised() {
    let (registry, mut attributes, who) = fresh();
    let corruption = id(&registry, "corruption");
    let mut events = Vec::new();
    let mut floor = whole(0);
    let steps = [10, 20, -25, 30, -100, 40, -5, 100, -100];
    for step in steps {
        attributes.apply(who, corruption, whole(step), &registry, &mut events);
        let attribute = attributes.get(corruption);
        assert!(
            attribute.minimum() >= floor,
            "the minimum fell after {step}"
        );
        assert!(attribute.current() >= attribute.minimum());
        floor = attribute.minimum();
    }
    assert_eq!(floor, whole(75), "rose through tainted, marked and lost");
    assert_eq!(attributes.get(corruption).current(), whole(75));

    // A smaller maximum cannot take the floor away either.
    attributes.add_modifier(
        who,
        corruption,
        Modifier::Override(whole(10)),
        &registry,
        &mut events,
    );
    assert_eq!(attributes.get(corruption).minimum(), whole(75));
    assert_eq!(attributes.get(corruption).maximum(), whole(75));
}
