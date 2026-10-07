// SPDX-License-Identifier: Apache-2.0
use bakery::{
    catalogue, fixture_hash, recipes, runner, script, shop, Bakery, Event, Intent, Place,
    DEFAULT_SEED, DEFAULT_STEPS, ROOM,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Runner, Simulation, StableVector, Streams};
use lockstep_crafting::{Crafting, CraftingEvent};
use lockstep_inventory::{Container, Inventory};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn script_streams() -> Streams {
    Streams::new(DEFAULT_SEED ^ 0x0062_616b_6572)
}

/// Every unit of a kind in a place, spoiled or not.
fn all_units(bakery: &Bakery, name: &str, at: Place) -> u32 {
    let inventory = &bakery.world().inventory;
    let kind = bakery.catalogue().kind_id(name).unwrap();
    inventory
        .container(bakery.place(at))
        .unwrap()
        .items()
        .iter()
        .filter_map(|item| inventory.item(*item))
        .filter(|item| item.kind == kind)
        .map(|item| item.count as u32)
        .sum()
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/bakery.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate, atomic consumption: across the session the pantry's flour, water and yeast change
/// by exactly what is delivered, less one dough recipe's inputs for each mix that starts. A mix
/// refused for a missing input takes nothing.
#[test]
fn the_pantry_changes_only_by_deliveries_and_whole_recipes() {
    let mut runner = runner(shop(), DEFAULT_SEED);
    let mut streams = script_streams();
    let catalogue = catalogue();
    let dough = recipes(&catalogue).recipe_id("dough").unwrap();
    let mut refused = 0;
    for step in 0..DEFAULT_STEPS {
        let intents = script(runner.simulation(), &mut streams);
        let units = |runner: &Runner<Bakery>| {
            ["flour", "water", "yeast"]
                .map(|name| all_units(runner.simulation(), name, Place::Pantry))
        };
        let before = units(&runner);
        let events = runner.step_once(&intents).events;
        let after = units(&runner);
        let mut expected = before.map(|units| units as i64);
        for event in &events {
            match event {
                Event::Delivered { kind, count } => {
                    let name = catalogue.kind(*kind).name.as_str();
                    let index = ["flour", "water", "yeast"]
                        .iter()
                        .position(|known| *known == name)
                        .unwrap();
                    expected[index] += *count as i64;
                }
                Event::Crafting(CraftingEvent::Started { recipe, .. }) if *recipe == dough => {
                    expected[0] -= 2;
                    expected[1] -= 1;
                    expected[2] -= 1;
                }
                Event::Refused { .. } => refused += 1,
                _ => {}
            }
        }
        assert_eq!(after.map(|units| units as i64), expected, "step {step}");
    }
    assert!(refused > 0, "some jobs were refused");
}

/// The gate, convergence: 2,000 bakes with the committed bread recipe land within 3 percent of
/// its 85, 10 and 5.
#[test]
fn two_thousand_bakes_land_near_the_outcome_table() {
    let catalogue = catalogue();
    let recipes = recipes(&catalogue);
    let bread = recipes.recipe_id("bread").unwrap();
    let dough = catalogue.kind_id("dough").unwrap();
    let oven = StableVector::new().insert(());
    let mut inventory = Inventory::new();
    inventory
        .add_container(oven, Container::new(64, None, ROOM))
        .unwrap();
    let mut crafting = Crafting::new();
    crafting.add_station(oven, recipes.station_id("oven").unwrap());
    let mut streams = Streams::new(DEFAULT_SEED);
    let mut counts = [0u32; 3];
    for _ in 0..2_000 {
        let item = inventory.create(&catalogue, dough, 1);
        inventory.put(item, oven, &catalogue).unwrap();
        let mut events = Vec::new();
        crafting
            .start(&mut inventory, &recipes, oven, bread, oven, &mut events)
            .unwrap();
        crafting.tick(
            &mut inventory,
            &catalogue,
            &recipes,
            Fixed32::from_int(30),
            &mut streams,
            &mut events,
        );
        for event in events {
            if let CraftingEvent::Finished { outcome, .. } = event {
                counts[outcome as usize] += 1;
            }
            if let CraftingEvent::Produced { item, .. } = event {
                let _ = inventory.destroy(item);
            }
        }
    }
    for (count, weight) in counts.iter().zip([85u32, 10, 5]) {
        assert!(count.abs_diff(weight * 20) <= 60, "{counts:?}");
    }
}

#[test]
fn bread_comes_from_dough_from_the_pantry() {
    let mut runner = runner(shop(), 1);
    let kind = |name: &str| runner.simulation().catalogue().kind_id(name).unwrap();
    let (flour, water, yeast) = (kind("flour"), kind("water"), kind("yeast"));
    runner.step_once(&[Intent::Deliver {
        kind: flour,
        count: 2,
    }]);
    runner.step_once(&[Intent::Deliver {
        kind: water,
        count: 1,
    }]);
    runner.step_once(&[Intent::Deliver {
        kind: yeast,
        count: 1,
    }]);
    runner.step_once(&[Intent::Mix]);
    // Ten minutes of mixing at one game minute a real second.
    for _ in 0..300 {
        runner.step_once(&[]);
    }
    let made_dough = runner.simulation().units("dough", Place::Mixer);
    runner.step_once(&[Intent::Bake]);
    for _ in 0..900 {
        runner.step_once(&[]);
    }
    let bakery = runner.simulation();
    assert_eq!(bakery.units("flour", Place::Pantry), 0);
    if made_dough == 1 {
        let baked = bakery.units("bread", Place::Oven) + bakery.units("charcoal", Place::Oven);
        assert!(baked <= 2);
        assert_eq!(
            bakery.units("dough", Place::Mixer),
            0,
            "the dough went in the oven"
        );
    }
}

#[test]
fn selling_needs_a_loaf_and_a_bad_delivery_is_turned_away() {
    let mut runner = runner(shop(), 1);
    assert_eq!(
        runner.step_once(&[Intent::Sell]).events,
        vec![Event::NothingToSell]
    );
    let flour = runner.simulation().catalogue().kind_id("flour").unwrap();
    let events = runner
        .step_once(&[Intent::Deliver {
            kind: flour,
            count: 11,
        }])
        .events;
    assert_eq!(events, vec![Event::Turned { kind: flour }]);
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    bakery: &mut Bakery,
    clock: &mut Clock,
    randomness: &mut Streams,
    intents: &[Intent],
    number: u64,
) -> Vec<Event> {
    let (minutes, exact) = clock.advance_exactly(&mut Vec::new());
    let mut events = Vec::new();
    let mut context = Context {
        clock,
        elapsed_game_minutes: minutes,
        elapsed_minutes: exact,
        randomness,
        events: &mut events,
        step_number: number,
        step_seconds: 1.0 / 30.0,
    };
    bakery.step(&mut context, intents);
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let session = runner(shop(), DEFAULT_SEED);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Bakery::create(shop(), &mut Streams::new(5));
    let mut streams = Streams::new(77);
    for number in 0..15_000 {
        let intents = script(&kept, &mut streams);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = Bakery::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 15_000..30_000 {
        let intents = script(&kept, &mut streams);
        let a = step(&mut kept, &mut clock, &mut randomness, &intents, number);
        let b = step(
            &mut loaded,
            &mut loaded_clock,
            &mut loaded_randomness,
            &intents,
            number,
        );
        assert_eq!(a, b, "step {number}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
}
