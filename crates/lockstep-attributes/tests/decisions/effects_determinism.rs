// SPDX-License-Identifier: Apache-2.0
//! Decision: attributes and effects are integer state machines, so a long run hashes the same
//! natively and under WebAssembly.
//! Alternative rejected: float attributes, whose rounding can differ between platforms.
//! Would change if: twelve attributes, five effects and 1,000 ticks hash to anything but the
//! committed value on any platform.

use lockstep_attributes::{
    Attributes, Effect, EffectContext, EffectTag, Effects, Modifier, Registry, Stacking,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Column, StableVector, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const TWELVE: &str = r#"{ "attributes": [
  { "name": "health", "minimum": 0, "maximum": 100, "starting": 100, "on_maximum_change": "scale_current",
    "thresholds": [ { "at": 50, "name": "hurt" }, { "at": 20, "name": "wounded" } ] },
  { "name": "hunger", "minimum": 0, "maximum": 100, "starting": 90,
    "thresholds": [ { "at": 60, "name": "peckish" }, { "at": 20, "name": "starving" } ] },
  { "name": "thirst", "minimum": 0, "maximum": 100, "starting": 90 },
  { "name": "sanity", "minimum": 0, "maximum": 100, "starting": 80, "thresholds": [ { "at": 30, "name": "shaken" } ] },
  { "name": "hope", "minimum": 0, "maximum": 100, "starting": 40 },
  { "name": "stamina", "minimum": 0, "maximum": 50, "starting": 50, "on_maximum_change": "scale_current" },
  { "name": "strength", "minimum": 0, "maximum": 20, "starting": 8 },
  { "name": "luck", "minimum": 0, "maximum": 20, "starting": 3 },
  { "name": "corruption", "minimum": 0, "maximum": 100, "starting": 0, "on_maximum_change": "ratchet",
    "thresholds": [ { "at": 25, "name": "tainted" }, { "at": 50, "name": "marked" } ] },
  { "name": "carry", "minimum": 0, "maximum": 400, "starting": 0,
    "derived": { "inputs": ["strength"], "curve": { "piecewise": { "knots": [[0, 20], [10, 120], [20, 160]] } } } },
  { "name": "critical_chance", "minimum": 0, "maximum": 100, "starting": 0,
    "derived": { "inputs": ["luck"], "curve": { "linear": { "per_point": "1.5", "offset": 5 } } } },
  { "name": "prayer_efficiency", "minimum": 0, "maximum": 100, "starting": 0,
    "derived": { "inputs": ["hope", "sanity"], "curve": "product" } }
] }"#;

fn run() -> u64 {
    let registry = Registry::from_json(TWELVE).unwrap();
    let id = |name: &str| registry.id(name).unwrap();
    let mut entities = StableVector::new();
    let who: Vec<_> = (0..3).map(|_| entities.insert(())).collect();
    let mut attributes = Column::new();
    for entity in &who {
        attributes.set(*entity, Attributes::from_registry(&registry));
    }
    let (mut attribute_events, mut effect_events) = (Vec::new(), Vec::new());
    let mut effects = Effects::new();
    let mut context = EffectContext {
        attributes: &mut attributes,
        registry: &registry,
        attribute_events: &mut attribute_events,
        effect_events: &mut effect_events,
    };
    let effect = |attribute,
                  modifier,
                  per_minute: Option<(i32, i32)>,
                  minutes: Option<i32>,
                  tag: &str,
                  stacking| Effect {
        attribute,
        modifier,
        per_minute: per_minute
            .map(|(numerator, denominator)| Fixed32::from_ratio(numerator, denominator)),
        remaining_minutes: minutes.map(Fixed32::from_int),
        tag: EffectTag::new(tag),
        stacking,
    };
    let five = [
        effect(
            id("hunger"),
            None,
            Some((-1, 6)),
            None,
            "hunger",
            Stacking::Independent,
        ),
        effect(
            id("thirst"),
            None,
            Some((-1, 4)),
            None,
            "thirst",
            Stacking::Independent,
        ),
        effect(
            id("health"),
            None,
            Some((-7, 3)),
            Some(30),
            "bleeding",
            Stacking::Independent,
        ),
        effect(
            id("luck"),
            Some(Modifier::Add(Fixed32::from_int(2))),
            None,
            Some(90),
            "relic",
            Stacking::RefreshDuration,
        ),
        effect(
            id("hope"),
            None,
            Some((3, 2)),
            Some(20),
            "prayer",
            Stacking::DailyBudget {
                cap_per_day: Fixed32::from_int(12),
            },
        ),
    ];
    let mut streams = Streams::new(77);
    for (index, entity) in who.iter().enumerate() {
        effects.add(*entity, five[index % 2].clone(), &mut context);
    }
    for tick in 0..1_000u32 {
        if streams.range("events", 0, 20) == 0 {
            let entity = who[streams.pick("who", who.len())];
            let chosen = five[streams.pick("which", five.len())].clone();
            effects.add(entity, chosen, &mut context);
        }
        if tick % 250 == 125 {
            effects.remove_by_tag(who[0], &EffectTag::new("bleeding"), &mut context);
        }
        let elapsed = Fixed32::from_raw(streams.range("elapsed", 1, 65_536));
        effects.tick(elapsed, tick / 400, &mut context);
    }
    hash_of(&(&attributes, &effects, &attribute_events, &effect_events))
}

#[test]
fn twelve_attributes_five_effects_and_1000_ticks_hash_to_the_committed_value() {
    let committed = include_str!("../../fixtures/effects.hash").trim();
    assert_eq!(format!("{:016x}", run()), committed);
}

/// Prints the value to commit. Run with
/// `cargo test -p lockstep-attributes print_effects_hash -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_effects_hash() {
    println!("{:016x}", run());
}
