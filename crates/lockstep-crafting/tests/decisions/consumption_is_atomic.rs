// SPDX-License-Identifier: Apache-2.0
//! Decision: a recipe takes all its inputs or none. Every input is counted first, spoiled units
//! not counting, across as many stacks as hold the kind; only when all are there is anything
//! consumed, and a short input is named.
//! Alternative rejected: consuming input by input and stopping at the first short one, which
//! loses the inputs already taken.
//! Would change if: a mixer short of flour takes any water, or two flour from two one-unit stacks
//! are not taken.

use crate::common::Bakery;
use lockstep_crafting::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_short_input_consumes_nothing_and_a_full_set_comes_from_any_stacks() {
    // Flour is listed first and is all there; water is short. Taking inputs one by one would eat
    // the flour before finding the water missing.
    let mut bakery = Bakery::new(4);
    bakery.stock("flour", 2, bakery.pantry);
    let before = bakery.inventory.clone();
    let dough = bakery.recipes.recipe_id("dough").unwrap();
    let water = bakery.catalogue.kind_id("water").unwrap();
    let mut events = Vec::new();
    assert_eq!(
        bakery.crafting.start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.mixer,
            dough,
            bakery.pantry,
            &mut events
        ),
        Err(Refusal::Missing {
            kind: water,
            needed: 1,
            have: 0
        })
    );
    assert_eq!(bakery.inventory, before, "nothing taken");
    // With water there, both one-unit stacks of flour are taken, and one water.
    bakery.stock("water", 5, bakery.pantry);
    bakery
        .crafting
        .start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.mixer,
            dough,
            bakery.pantry,
            &mut events,
        )
        .unwrap();
    assert_eq!(bakery.available("flour", bakery.pantry), 0);
    assert_eq!(bakery.available("water", bakery.pantry), 4);
}

#[test]
fn a_kind_listed_twice_needs_both_counts() {
    // The double-flour recipe lists flour 1 twice: one flour is not enough.
    let mut bakery = Bakery::new(4);
    bakery.stock("flour", 1, bakery.pantry);
    let double = bakery.recipes.recipe_id("double").unwrap();
    let flour = bakery.catalogue.kind_id("flour").unwrap();
    assert_eq!(
        bakery.crafting.start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.mixer,
            double,
            bakery.pantry,
            &mut Vec::new()
        ),
        Err(Refusal::Missing {
            kind: flour,
            needed: 2,
            have: 1
        })
    );
}

#[test]
fn spoiled_units_do_not_count() {
    let mut bakery = Bakery::new(4);
    bakery.stock("water", 1, bakery.pantry);
    bakery.stock("flour", 2, bakery.pantry);
    // Flour spoils after 600 minutes.
    bakery.inventory.spoil(
        lockstep_core::math::Fixed32::from_int(600),
        18,
        &bakery.catalogue,
        &mut Vec::new(),
    );
    let dough = bakery.recipes.recipe_id("dough").unwrap();
    let flour = bakery.catalogue.kind_id("flour").unwrap();
    assert_eq!(
        bakery.crafting.start(
            &mut bakery.inventory,
            &bakery.recipes,
            bakery.mixer,
            dough,
            bakery.pantry,
            &mut Vec::new()
        ),
        Err(Refusal::Missing {
            kind: flour,
            needed: 2,
            have: 0
        })
    );
}
