// SPDX-License-Identifier: Apache-2.0
//! Decision: each attribute chooses what a new maximum does to its current value. Health scales
//! (50 of 100 becomes 75 of 150); hunger clamps (50 of 100 stays 50 of 150).
//! Alternative rejected: one rule for every attribute, which either heals on equip or leaves a
//! full stomach looking empty.
//! Would change if: the two cases below give anything but 75 of 150 and 50 of 150, or removing
//! the modifier does not return health to 50 of 100.

use crate::common::{fresh, id, whole};
use lockstep_attributes::Modifier;
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn health_scales_and_hunger_clamps() {
    let (registry, mut attributes, who) = fresh();
    let (health, hunger) = (id(&registry, "health"), id(&registry, "hunger"));
    let mut events = Vec::new();
    attributes.apply(who, hunger, whole(-30), &registry, &mut events);
    assert_eq!(attributes.get(health).current(), whole(50));
    assert_eq!(attributes.get(hunger).current(), whole(50));

    let half_again = Modifier::Multiply(Fixed32::from_ratio(3, 2));
    let on_health = attributes.add_modifier(who, health, half_again, &registry, &mut events);
    attributes.add_modifier(who, hunger, half_again, &registry, &mut events);
    assert_eq!(
        (
            attributes.get(health).current(),
            attributes.get(health).maximum()
        ),
        (whole(75), whole(150))
    );
    assert_eq!(
        (
            attributes.get(hunger).current(),
            attributes.get(hunger).maximum()
        ),
        (whole(50), whole(150))
    );

    attributes.remove_modifier(who, health, on_health, &registry, &mut events);
    assert_eq!(
        (
            attributes.get(health).current(),
            attributes.get(health).maximum()
        ),
        (whole(50), whole(100))
    );
}
