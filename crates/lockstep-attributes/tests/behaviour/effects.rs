// SPDX-License-Identifier: Apache-2.0
use crate::common::{bleeding, column_with, fresh, id, whole};
use lockstep_attributes::{
    Attributes, Effect, EffectContext, EffectEvent, EffectTag, Effects, Modifier, Registry,
    Stacking,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Column, Handle};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

struct World {
    registry: Registry,
    who: Handle,
    attributes: Column<Attributes>,
    effects: Effects,
    attribute_events: Vec<lockstep_attributes::AttributeEvent>,
    effect_events: Vec<EffectEvent>,
}

impl World {
    fn new() -> Self {
        let (registry, _, who) = fresh();
        let attributes = column_with(&registry, who);
        World {
            registry,
            who,
            attributes,
            effects: Effects::new(),
            attribute_events: Vec::new(),
            effect_events: Vec::new(),
        }
    }

    fn context(&mut self) -> (&mut Effects, EffectContext<'_>) {
        (
            &mut self.effects,
            EffectContext {
                attributes: &mut self.attributes,
                registry: &self.registry,
                attribute_events: &mut self.attribute_events,
                effect_events: &mut self.effect_events,
            },
        )
    }

    fn add(&mut self, effect: Effect) -> lockstep_attributes::EffectHandle {
        let who = self.who;
        let (effects, mut context) = self.context();
        effects.add(who, effect, &mut context)
    }

    fn tick(&mut self, minutes: i32) {
        let (effects, mut context) = self.context();
        effects.tick(whole(minutes), 0, &mut context);
    }

    fn value(&self, name: &str) -> (Fixed32, Fixed32) {
        let attribute = self
            .attributes
            .get(self.who)
            .unwrap()
            .get(id(&self.registry, name));
        (attribute.current(), attribute.maximum())
    }
}

#[test]
fn independent_effects_stack_and_each_one_expires_on_its_own() {
    let mut world = World::new();
    world.add(bleeding(&world.registry, Some(5)));
    world.add(bleeding(&world.registry, Some(10)));
    world.tick(5);
    assert_eq!(
        world.value("health").0,
        whole(50 - 20),
        "two bleeds drain twice"
    );
    world.tick(5);
    assert_eq!(world.value("health").0, whole(50 - 30));
    world.tick(5);
    assert_eq!(world.value("health").0, whole(50 - 30), "both have expired");
    let tag = EffectTag::new("bleeding");
    let who = world.who;
    assert_eq!(
        world.effect_events,
        [
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Expired {
                who,
                tag: tag.clone()
            },
            EffectEvent::Expired { who, tag },
        ]
    );
}

#[test]
fn replace_removes_the_old_effect_and_its_modifier_first() {
    let mut world = World::new();
    let hunger = world.registry.id("hunger").unwrap();
    let fever = |amount| Effect {
        attribute: hunger,
        modifier: Some(Modifier::Add(whole(amount))),
        per_minute: None,
        remaining_minutes: None,
        tag: EffectTag::new("fever"),
        stacking: Stacking::Replace,
    };
    let first = world.add(fever(10));
    let second = world.add(fever(30));
    assert_ne!(first, second);
    assert_eq!(world.value("hunger").1, whole(130));
    let tag = EffectTag::new("fever");
    let who = world.who;
    assert_eq!(
        world.effect_events,
        [
            EffectEvent::Applied {
                who,
                tag: tag.clone()
            },
            EffectEvent::Removed {
                who,
                tag: tag.clone()
            },
            EffectEvent::Applied { who, tag },
        ]
    );
}

#[test]
fn a_refresh_only_refreshes_an_effect_that_refreshes() {
    let mut world = World::new();
    let independent = bleeding(&world.registry, Some(10));
    let mut refreshing = independent.clone();
    refreshing.stacking = Stacking::RefreshDuration;
    let first = world.add(independent);
    let second = world.add(refreshing);
    assert_ne!(
        first, second,
        "a new effect, not a refresh of the independent one"
    );
    world.tick(10);
    assert_eq!(world.value("health").0, whole(10));
}

#[test]
fn removing_by_tag_takes_every_match_and_gives_back_their_modifiers() {
    let mut world = World::new();
    let mut held = bleeding(&world.registry, None);
    held.modifier = Some(Modifier::Add(whole(-10)));
    world.add(held.clone());
    world.add(held);
    assert_eq!(world.value("health").1, whole(80));
    let who = world.who;
    let (effects, mut context) = world.context();
    assert_eq!(
        effects.remove_by_tag(who, &EffectTag::new("bleeding"), &mut context),
        2
    );
    assert_eq!(
        effects.remove_by_tag(who, &EffectTag::new("bleeding"), &mut context),
        0
    );
    assert!(!effects.has(who, &EffectTag::new("bleeding")));
    assert_eq!(world.value("health").1, whole(100));
    world.tick(60);
    assert_eq!(world.value("health").0, whole(50), "no drain after removal");
}

#[test]
fn an_effect_without_a_duration_lasts_until_removed() {
    let mut world = World::new();
    let handle = world.add(bleeding(&world.registry, None));
    world.tick(10);
    assert_eq!(world.value("health").0, whole(30));
    let (_, effect, left) = world.effects.on(world.who).next().unwrap();
    assert_eq!((effect.tag.0.as_str(), left), ("bleeding", None));
    let (effects, mut context) = world.context();
    assert!(effects.remove(handle, &mut context));
    assert!(!effects.remove(handle, &mut context), "already removed");
    world.tick(10);
    assert_eq!(world.value("health").0, whole(30));
}

#[test]
fn the_minutes_left_count_down_and_a_drain_stops_exactly_at_the_end() {
    let mut world = World::new();
    world.add(bleeding(&world.registry, Some(10)));
    world.tick(4);
    let left: Vec<_> = world
        .effects
        .on(world.who)
        .map(|(_, _, left)| left)
        .collect();
    assert_eq!(left, [Some(whole(6))]);
    world.tick(100);
    assert_eq!(
        world.value("health").0,
        whole(30),
        "20 minutes of drain is 10 minutes at 2"
    );
    assert_eq!(world.effects.on(world.who).count(), 0);
}

#[test]
fn an_entity_without_attributes_or_one_that_is_gone_is_left_alone() {
    let mut world = World::new();
    let mut store = lockstep_core::StableVector::new();
    store.insert(());
    let stranger = store.insert(());
    let effect = bleeding(&world.registry, Some(10));
    let (effects, mut context) = world.context();
    effects.add(stranger, effect, &mut context);
    effects.tick(whole(5), 0, &mut context);
    assert_eq!(
        effects.on(stranger).count(),
        1,
        "kept, but nothing to drain"
    );
    effects.remove_entity(stranger);
    assert_eq!(effects.on(stranger).count(), 0);
    assert_eq!(world.value("health").0, whole(50));
}

#[test]
fn saved_effects_continue_exactly_like_the_originals() {
    let mut original = World::new();
    original.add(bleeding(&original.registry, Some(30)));
    let mut prayer = bleeding(&original.registry, Some(40));
    prayer.attribute = original.registry.id("hunger").unwrap();
    prayer.per_minute = Some(Fixed32::from_ratio(1, 3));
    prayer.tag = EffectTag::new("prayer");
    prayer.stacking = Stacking::DailyBudget {
        cap_per_day: whole(5),
    };
    original.add(prayer);
    original.tick(7);

    let mut restored = World::new();
    restored.attributes =
        bincode::deserialize(&bincode::serialize(&original.attributes).unwrap()).unwrap();
    restored.effects =
        bincode::deserialize(&bincode::serialize(&original.effects).unwrap()).unwrap();
    for world in [&mut original, &mut restored] {
        world.attribute_events.clear();
        world.effect_events.clear();
        for _ in 0..20 {
            world.tick(3);
        }
    }
    assert_eq!(
        hash_of(&(
            &restored.attributes,
            &restored.effects,
            &restored.attribute_events,
            &restored.effect_events
        )),
        hash_of(&(
            &original.attributes,
            &original.effects,
            &original.attribute_events,
            &original.effect_events
        ))
    );
    assert_eq!(restored.effects, original.effects);
}
