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
    let mut bakery = Bakery::new(4);
    bakery.stock("water", 5, bakery.pantry);
    bakery.stock("flour", 1, bakery.pantry);
    let before = bakery.inventory.clone();
    let dough = bakery.recipes.recipe_id("dough").unwrap();
    let flour = bakery.catalogue.kind_id("flour").unwrap();
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
            kind: flour,
            needed: 2,
            have: 1
        })
    );
    assert_eq!(bakery.inventory, before, "nothing taken");
    // A second one-unit stack of flour: now both are taken, with one water.
    bakery.stock("flour", 1, bakery.pantry);
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
