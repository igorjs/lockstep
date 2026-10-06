// SPDX-License-Identifier: Apache-2.0
//! Decision: a `DailyBudget` caps the total change an entity gets from one tag in a game day.
//! Prayer restores hope, but praying all day gives no more than the cap until the next day.
//! Alternative rejected: diminishing each application by a factor, which still rewards spamming
//! and needs a tuned curve.
//! Would change if: a capped effect gives anything past its cap before the next game day, or the
//! next day's budget does not start fresh.

use crate::common::{column_with, fresh, id, whole};
use lockstep_attributes::{Effect, EffectContext, EffectTag, Effects, Stacking};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_capped_effect_gives_nothing_past_its_cap_until_the_next_game_day() {
    let (registry, _, who) = fresh();
    let hunger = id(&registry, "hunger");
    let mut attributes = column_with(&registry, who);
    let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
    let mut effects = Effects::new();
    let mut context = EffectContext {
        attributes: &mut attributes,
        registry: &registry,
        attribute_events: &mut attribute_events,
        effect_events: &mut effect_events,
    };
    let current =
        |context: &EffectContext<'_>| context.attributes.get(who).unwrap().get(hunger).current();
    // Start low, so the cap and not the maximum is what stops the gain.
    context.attributes.get_mut(who).unwrap().apply(
        who,
        hunger,
        whole(-70),
        &registry,
        &mut Vec::new(),
    );
    assert_eq!(current(&context), whole(10));
    let prayer = Effect {
        attribute: hunger,
        modifier: None,
        per_minute: Some(whole(2)),
        remaining_minutes: Some(whole(10)),
        tag: EffectTag::new("prayer"),
        stacking: Stacking::DailyBudget {
            cap_per_day: whole(15),
        },
    };
    // Three prayers on day 0 would give 60; the budget allows 15.
    for _ in 0..3 {
        effects.add(who, prayer.clone(), &mut context);
        effects.tick(whole(10), 0, &mut context);
    }
    assert_eq!(current(&context), whole(25));

    // The next day starts a fresh budget.
    effects.add(who, prayer.clone(), &mut context);
    effects.tick(whole(10), 1, &mut context);
    assert_eq!(current(&context), whole(40));

    // A drain under the same tag spends the same budget.
    let mut penance = prayer;
    penance.per_minute = Some(whole(-3));
    effects.add(who, penance, &mut context);
    effects.tick(whole(10), 1, &mut context);
    assert_eq!(current(&context), whole(40), "today's 15 are spent");
}

#[test]
fn a_gain_that_lands_on_a_full_attribute_spends_nothing() {
    let (registry, _, who) = fresh();
    let hunger = id(&registry, "hunger");
    let mut attributes = column_with(&registry, who);
    let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
    let mut effects = Effects::new();
    let mut context = EffectContext {
        attributes: &mut attributes,
        registry: &registry,
        attribute_events: &mut attribute_events,
        effect_events: &mut effect_events,
    };
    let current =
        |context: &EffectContext<'_>| context.attributes.get(who).unwrap().get(hunger).current();
    context.attributes.get_mut(who).unwrap().apply(
        who,
        hunger,
        whole(20),
        &registry,
        &mut Vec::new(),
    );
    assert_eq!(current(&context), whole(100), "full");
    let prayer = Effect {
        attribute: hunger,
        modifier: None,
        per_minute: Some(whole(2)),
        remaining_minutes: Some(whole(10)),
        tag: EffectTag::new("prayer"),
        stacking: Stacking::DailyBudget {
            cap_per_day: whole(15),
        },
    };
    effects.add(who, prayer.clone(), &mut context);
    effects.tick(whole(10), 0, &mut context);
    assert_eq!(current(&context), whole(100));
    // Later the same day the whole budget is still there.
    context.attributes.get_mut(who).unwrap().apply(
        who,
        hunger,
        whole(-50),
        &registry,
        &mut Vec::new(),
    );
    effects.add(who, prayer, &mut context);
    effects.tick(whole(10), 0, &mut context);
    assert_eq!(current(&context), whole(65));
}
