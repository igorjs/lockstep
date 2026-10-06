// SPDX-License-Identifier: Apache-2.0
//! The capsule as the consumer of lockstep-attributes: needs, health and sanity come from
//! `data/attributes.json`, and decay, bleeding, starvation and prayer are effects.

use capsule::{registry, runner, Capsule, Event, Intent, ATTRIBUTES_JSON};
use lockstep_attributes::EffectTag;
use lockstep_core::math::Fixed32;
use lockstep_core::Runner;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

/// Runs `steps` steps, sending `intents` at the first.
fn run_with(runner: &mut Runner<Capsule>, intents: &[Intent], steps: u64) -> Vec<Event> {
    let mut events = runner.step_once(intents).events;
    for _ in 1..steps {
        events.extend(runner.step_once(&[]).events);
    }
    events
}

#[test]
fn the_attributes_come_from_the_committed_json() {
    let registry = registry();
    for name in ["health", "hunger", "thirst", "sanity"] {
        assert!(registry.id(name).is_some(), "{name}");
    }
    assert!(ATTRIBUTES_JSON.contains("\"hungry\""));
    let capsule = runner(1);
    let survivor = capsule.simulation().survivor().unwrap();
    let ids = capsule.simulation().ids();
    let world = capsule.simulation().world();
    assert_eq!(world.value(survivor, ids.health), Some(whole(100)));
    let decay: Vec<String> = world
        .effects
        .on(survivor)
        .map(|(_, effect, _)| effect.tag.0.clone())
        .collect();
    assert_eq!(decay, ["hunger", "thirst", "dread"]);
}

#[test]
fn a_wound_bleeds_by_a_roll_and_a_bandage_stops_every_bleed() {
    // Find a seed where the first wound bleeds; the roll is the capsule's only use of "wounds".
    let (mut session, survivor) = (1..200)
        .map(|seed| {
            let mut session = runner(seed);
            let survivor = session.simulation().survivor().unwrap();
            let events = run_with(&mut session, &[Intent::Wound { entity: survivor }], 1);
            (session, survivor, events)
        })
        .find(|(_, survivor, events)| events.contains(&Event::Bleeding { entity: *survivor }))
        .map(|(session, survivor, _)| (session, survivor))
        .expect("a 60 percent roll bleeds within 200 seeds");
    let health = session.simulation().ids().health;
    let bleeding = EffectTag::new("bleeding");
    assert!(session
        .simulation()
        .world()
        .effects
        .has(survivor, &bleeding));

    // Bleeding costs a point of health a game minute: 1,800 steps at multiplier 1 are 12 minutes.
    run_with(&mut session, &[], 1_800);
    let after_bleeding = session
        .simulation()
        .world()
        .value(survivor, health)
        .unwrap();
    assert!(
        after_bleeding <= whole(89) && after_bleeding > whole(87),
        "{after_bleeding:?}"
    );

    let events = run_with(&mut session, &[Intent::Bandage { entity: survivor }], 1_800);
    assert!(events.contains(&Event::Bandaged {
        entity: survivor,
        wounds: 1
    }));
    assert!(!session
        .simulation()
        .world()
        .effects
        .has(survivor, &bleeding));
    let after_bandage = session
        .simulation()
        .world()
        .value(survivor, health)
        .unwrap();
    assert!(
        after_bleeding - after_bandage < Fixed32::from_ratio(1, 50),
        "no more bleeding"
    );
}

#[test]
fn wounds_bleed_about_sixty_percent_of_the_time() {
    let mut bled = 0;
    for seed in 0..500 {
        let mut session = runner(seed);
        let survivor = session.simulation().survivor().unwrap();
        let events = run_with(&mut session, &[Intent::Wound { entity: survivor }], 1);
        if events.contains(&Event::Bleeding { entity: survivor }) {
            bled += 1;
        } else {
            assert!(events.contains(&Event::Grazed { entity: survivor }));
        }
    }
    assert!((260..=340).contains(&bled), "{bled} of 500");
}

#[test]
fn prayer_restores_sanity_but_no_more_than_ten_a_day() {
    // Two identical sessions at twenty times: one prays three times, one does not. Ten game hours
    // of dread first, so sanity has room to rise.
    let sessions: Vec<_> = [true, false]
        .into_iter()
        .map(|praying| {
            let mut session = runner(3);
            let survivor = session.simulation().survivor().unwrap();
            session.set_clock_multiplier(20.0);
            run_with(&mut session, &[], 4_500);
            let day = session.clock().day();
            // Three prayers of twenty game minutes each would restore 60.
            for _ in 0..3 {
                let intents = if praying {
                    vec![Intent::Pray { entity: survivor }]
                } else {
                    vec![]
                };
                let events = run_with(&mut session, &intents, 150);
                assert_eq!(
                    events.contains(&Event::Prayed { entity: survivor }),
                    praying
                );
            }
            assert_eq!(session.clock().day(), day, "still the same game day");
            let sanity = session.simulation().ids().sanity;
            let value = session
                .simulation()
                .world()
                .value(survivor, sanity)
                .unwrap();
            assert!(
                value < whole(100),
                "below the maximum, so the clamp hides nothing: {value:?}"
            );
            value
        })
        .collect();
    assert_eq!(
        sessions[0] - sessions[1],
        whole(10),
        "exactly the daily budget"
    );
}

#[test]
fn intents_for_someone_who_is_gone_are_rejected() {
    let mut session = runner(4);
    let survivor = session.simulation().survivor().unwrap();
    let mut store = lockstep_core::StableVector::new();
    store.insert(());
    let stranger = store.insert(());
    assert_ne!(stranger, survivor);
    for intent in [
        Intent::Wound { entity: stranger },
        Intent::Bandage { entity: stranger },
        Intent::Pray { entity: stranger },
    ] {
        let events = run_with(&mut session, &[intent], 1);
        assert_eq!(events, [Event::Rejected { entity: stranger }]);
    }
}
