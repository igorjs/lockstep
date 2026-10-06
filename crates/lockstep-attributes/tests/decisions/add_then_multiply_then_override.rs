// SPDX-License-Identifier: Apache-2.0
//! Decision: the maximum is `(base + every Add) × every Multiply`, then the last Override wins.
//! Alternative rejected: applying modifiers in the order they were added, where the same items
//! equipped in a different order give a different maximum.
//! Would change if: base 100 with +20 and ×1.5 gives anything but 180, in either order.

use crate::common::{fresh, id, whole};
use lockstep_attributes::Modifier;
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn base_100_plus_20_times_one_and_a_half_is_180_not_170_in_either_order() {
    let one_and_a_half = Fixed32::from_ratio(3, 2);
    for multiply_first in [false, true] {
        let (registry, mut attributes, who) = fresh();
        let hunger = id(&registry, "hunger");
        let mut events = Vec::new();
        let mut add =
            |modifier| attributes.add_modifier(who, hunger, modifier, &registry, &mut events);
        if multiply_first {
            add(Modifier::Multiply(one_and_a_half));
            add(Modifier::Add(whole(20)));
        } else {
            add(Modifier::Add(whole(20)));
            add(Modifier::Multiply(one_and_a_half));
        }
        assert_eq!(attributes.get(hunger).maximum(), whole(180));
    }
}

#[test]
fn the_last_override_wins_and_removing_it_restores_the_one_before() {
    let (registry, mut attributes, who) = fresh();
    let hunger = id(&registry, "hunger");
    let mut events = Vec::new();
    attributes.add_modifier(
        who,
        hunger,
        Modifier::Add(whole(20)),
        &registry,
        &mut events,
    );
    attributes.add_modifier(
        who,
        hunger,
        Modifier::Override(whole(30)),
        &registry,
        &mut events,
    );
    let last = attributes.add_modifier(
        who,
        hunger,
        Modifier::Override(whole(40)),
        &registry,
        &mut events,
    );
    assert_eq!(attributes.get(hunger).maximum(), whole(40));
    assert!(attributes.remove_modifier(who, hunger, last, &registry, &mut events));
    assert_eq!(attributes.get(hunger).maximum(), whole(30));
}
