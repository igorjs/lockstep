// SPDX-License-Identifier: Apache-2.0
//! Decision: an effect's drain depends only on the total game minutes it has run. Each tick
//! applies the change in the running total (`per_minute × minutes`), so splitting the same minutes
//! into more ticks changes nothing.
//! Alternative rejected: applying `per_minute × elapsed` per tick, which rounds once per tick and
//! drifts with the frame rate.
//! Would change if: one minute in one tick and in 1,800 ticks give different health or events
//! (the number to beat is zero differences).

use crate::common::{bleeding, column_with, fresh, id, whole};
use lockstep_attributes::{EffectContext, Effects};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn one_minute_in_one_tick_and_in_1800_ticks_give_the_same_health_and_events() {
    let run = |ticks: i32| {
        let (registry, _, who) = fresh();
        let mut attributes = column_with(&registry, who);
        let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
        let mut effects = Effects::new();
        let mut context = EffectContext {
            attributes: &mut attributes,
            registry: &registry,
            attribute_events: &mut attribute_events,
            effect_events: &mut effect_events,
        };
        // Bleeding at 2 a minute, odd fractions so every tick rounds; it runs out after 20 minutes.
        let mut effect = bleeding(&registry, Some(20));
        effect.per_minute = Some(Fixed32::from_ratio(-13, 7));
        effects.add(who, effect, &mut context);
        // 25 minutes, split into `ticks` pieces whose raw sizes sum exactly to the total.
        let total = whole(25).raw();
        for index in 0..ticks {
            let piece = total / ticks + i32::from(index < total % ticks);
            effects.tick(Fixed32::from_raw(piece), 0, &mut context);
        }
        let health = context
            .attributes
            .get(who)
            .unwrap()
            .get(id(&registry, "health"))
            .current();
        (health, attribute_events, effect_events)
    };
    let once = run(1);
    // The drain stops at 20 minutes: exactly 20 times the rate's raw value.
    let rate = Fixed32::from_ratio(-13, 7);
    assert_eq!(once.0, whole(50) + Fixed32::from_raw(rate.raw() * 20));
    assert_eq!(run(1_800), once);
    assert_eq!(run(25 * 1_800), once);
}
