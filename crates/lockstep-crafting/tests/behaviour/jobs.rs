// SPDX-License-Identifier: Apache-2.0
use crate::common::Bakery;
use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_crafting::{CraftingEvent, Refusal};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn minutes(whole: i32) -> Fixed32 {
    Fixed32::from_int(whole)
}

#[test]
fn a_job_takes_its_minutes_and_puts_its_outcome_in_the_station() {
    let mut bakery = Bakery::new(4);
    bakery.stock("dough", 1, bakery.oven);
    let bread = bakery.recipes.recipe_id("bread").unwrap();
    let mut events = Vec::new();
    bakery
        .crafting
        .start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.oven,
            bread,
            bakery.oven,
            &mut events,
        )
        .unwrap();
    assert_eq!(bakery.available("dough", bakery.oven), 0);
    let mut streams = Streams::new(1);
    bakery.crafting.tick(
        &mut bakery.inventory,
        &bakery.catalogue,
        &bakery.recipes,
        minutes(29),
        &mut streams,
        &mut events,
    );
    assert!(bakery.crafting.job(bakery.oven).is_some(), "a minute to go");
    assert_eq!(
        bakery
            .crafting
            .job(bakery.oven)
            .unwrap()
            .remaining_minutes(),
        minutes(1)
    );
    bakery.crafting.tick(
        &mut bakery.inventory,
        &bakery.catalogue,
        &bakery.recipes,
        minutes(1),
        &mut streams,
        &mut events,
    );
    assert!(bakery.crafting.job(bakery.oven).is_none());
    let outcome = events.iter().find_map(|event| match event {
        CraftingEvent::Finished { outcome, .. } => Some(*outcome),
        _ => None,
    });
    let produced =
        bakery.available("bread", bakery.oven) + bakery.available("charcoal", bakery.oven);
    match outcome {
        Some(0) => assert_eq!(bakery.available("bread", bakery.oven), 2),
        Some(1) => assert_eq!(bakery.available("charcoal", bakery.oven), 1),
        Some(2) => assert_eq!(produced, 0, "collapsed: nothing"),
        other => panic!("no outcome: {other:?}"),
    }
}

#[test]
fn a_station_refuses_the_wrong_recipe_a_second_job_and_strangers() {
    let mut bakery = Bakery::new(4);
    bakery.stock("dough", 2, bakery.pantry);
    let (bread, dough) = (
        bakery.recipes.recipe_id("bread").unwrap(),
        bakery.recipes.recipe_id("dough").unwrap(),
    );
    let mut events = Vec::new();
    let (inventory, recipes) = (&mut bakery.inventory, &bakery.recipes);
    assert_eq!(
        bakery.crafting.start(
            inventory,
            recipes,
            bakery.oven,
            dough,
            bakery.pantry,
            &mut events
        ),
        Err(Refusal::WrongStation)
    );
    bakery
        .crafting
        .start(
            inventory,
            recipes,
            bakery.oven,
            bread,
            bakery.pantry,
            &mut events,
        )
        .unwrap();
    assert_eq!(
        bakery.crafting.start(
            inventory,
            recipes,
            bakery.oven,
            bread,
            bakery.pantry,
            &mut events
        ),
        Err(Refusal::Busy)
    );
    assert_eq!(
        bakery.crafting.start(
            inventory,
            recipes,
            bakery.pantry,
            bread,
            bakery.pantry,
            &mut events
        ),
        Err(Refusal::UnknownStation)
    );
}

#[test]
fn an_output_with_no_room_is_left_loose() {
    // An oven with no slots: the outputs cannot go in.
    let mut bakery = Bakery::new(0);
    bakery.stock("dough", 1, bakery.pantry);
    let bread = bakery.recipes.recipe_id("bread").unwrap();
    let mut events = Vec::new();
    bakery
        .crafting
        .start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.oven,
            bread,
            bakery.pantry,
            &mut events,
        )
        .unwrap();
    // Seed 3 draws the good outcome (checked below), two bread.
    let mut streams = Streams::new(3);
    bakery.crafting.tick(
        &mut bakery.inventory,
        &bakery.catalogue,
        &bakery.recipes,
        minutes(30),
        &mut streams,
        &mut events,
    );
    assert!(events.contains(&CraftingEvent::Finished {
        station: bakery.oven,
        recipe: bread,
        outcome: 0
    }));
    let overflowed = events
        .iter()
        .filter(|event| matches!(event, CraftingEvent::Overflowed { .. }))
        .count();
    assert_eq!(overflowed, 1);
}

#[test]
fn the_same_seed_bakes_the_same_and_a_workshop_saves_and_loads() {
    let bake = |seed| {
        let mut bakery = Bakery::new(8);
        let bread = bakery.recipes.recipe_id("bread").unwrap();
        let mut outcomes = Vec::new();
        let mut streams = Streams::new(seed);
        for _ in 0..20 {
            bakery.stock("dough", 1, bakery.pantry);
            let mut events = Vec::new();
            bakery
                .crafting
                .start(
                    &mut bakery.inventory,
                    &bakery.recipes,
                    bakery.oven,
                    bread,
                    bakery.pantry,
                    &mut events,
                )
                .unwrap();
            bakery.crafting.tick(
                &mut bakery.inventory,
                &bakery.catalogue,
                &bakery.recipes,
                minutes(30),
                &mut streams,
                &mut events,
            );
            outcomes.extend(events.iter().filter_map(|event| match event {
                CraftingEvent::Finished { outcome, .. } => Some(*outcome),
                _ => None,
            }));
        }
        (outcomes, bakery.crafting)
    };
    let (first, crafting) = bake(11);
    assert_eq!(first, bake(11).0);
    let saved = lockstep_core::encode(&crafting);
    let loaded: lockstep_crafting::Crafting = lockstep_core::decode(&saved).unwrap();
    assert_eq!(loaded, crafting);
}
