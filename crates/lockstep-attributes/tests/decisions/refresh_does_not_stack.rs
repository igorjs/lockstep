// SPDX-License-Identifier: Apache-2.0
//! Decision: a `RefreshDuration` effect applied again restarts its timer and keeps its one
//! modifier.
//! Alternative rejected: stacking each application, which turns a repeated blessing into an
//! unbounded bonus.
//! Would change if: a refreshed effect applies its modifier more than once, or expires on its old
//! timer.

use crate::common::{column_with, fresh, id, whole};
use lockstep_attributes::{
    Effect, EffectContext, EffectEvent, EffectTag, Effects, Modifier, Stacking,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_refreshed_effect_applies_its_modifier_once_and_restarts_its_timer() {
    let (registry, _, who) = fresh();
    let health = id(&registry, "health");
    let mut attributes = column_with(&registry, who);
    let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
    let mut effects = Effects::new();
    let mut context = EffectContext {
        attributes: &mut attributes,
        registry: &registry,
        attribute_events: &mut attribute_events,
        effect_events: &mut effect_events,
    };
    let blessed = Effect {
        attribute: health,
        modifier: Some(Modifier::Add(whole(20))),
        per_minute: None,
        remaining_minutes: Some(whole(10)),
        tag: EffectTag::new("blessed"),
        stacking: Stacking::RefreshDuration,
    };
    let first = effects.add(who, blessed.clone(), &mut context);
    effects.tick(whole(8), 0, &mut context);
    let second = effects.add(who, blessed.clone(), &mut context);
    let third = effects.add(who, blessed, &mut context);
    assert_eq!((first, second), (first, third), "one effect, refreshed");
    let maximum =
        |context: &EffectContext<'_>| context.attributes.get(who).unwrap().get(health).maximum();
    assert_eq!(maximum(&context), whole(120), "the modifier applies once");

    // The old timer would have run out at 10 minutes; the refreshed one runs 10 from minute 8.
    effects.tick(whole(9), 0, &mut context);
    assert_eq!(maximum(&context), whole(120));
    effects.tick(whole(1), 0, &mut context);
    assert_eq!(maximum(&context), whole(100));
    let tag = EffectTag::new("blessed");
    assert_eq!(
        context.effect_events.as_slice(),
        [
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Expired { who, tag },
        ]
    );
}

#[test]
fn a_refreshed_drain_starts_counting_again_and_never_heals() {
    let (registry, _, who) = fresh();
    let health = id(&registry, "health");
    let mut attributes = column_with(&registry, who);
    let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
    let mut effects = Effects::new();
    let mut context = EffectContext {
        attributes: &mut attributes,
        registry: &registry,
        attribute_events: &mut attribute_events,
        effect_events: &mut effect_events,
    };
    let poison = Effect {
        attribute: health,
        modifier: None,
        per_minute: Some(whole(-2)),
        remaining_minutes: Some(whole(10)),
        tag: EffectTag::new("poison"),
        stacking: Stacking::RefreshDuration,
    };
    let current =
        |context: &EffectContext<'_>| context.attributes.get(who).unwrap().get(health).current();
    effects.add(who, poison.clone(), &mut context);
    effects.tick(whole(8), 0, &mut context);
    assert_eq!(current(&context), whole(34));
    effects.add(who, poison, &mut context);
    effects.tick(whole(1), 0, &mut context);
    assert_eq!(
        current(&context),
        whole(32),
        "one more minute at 2, not a heal"
    );
    effects.tick(whole(20), 0, &mut context);
    assert_eq!(current(&context), whole(14), "ten minutes from the refresh");
}
