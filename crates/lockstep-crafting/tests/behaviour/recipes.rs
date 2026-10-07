// SPDX-License-Identifier: Apache-2.0
use crate::common::{CATALOGUE, RECIPES};
use lockstep_attributes::Registry;
use lockstep_crafting::{Recipes, RecipesError};
use lockstep_inventory::Catalogue;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn catalogue() -> Catalogue {
    Catalogue::from_json(
        CATALOGUE,
        &Registry::from_json(r#"{ "attributes": [] }"#).unwrap(),
    )
    .unwrap()
}

#[test]
fn recipes_read_stations_inputs_and_outcome_tables() {
    let catalogue = catalogue();
    let recipes = Recipes::from_json(RECIPES, &catalogue).unwrap();
    let bread = recipes.recipe(recipes.recipe_id("bread").unwrap()).unwrap();
    assert_eq!(bread.station, recipes.station_id("oven").unwrap());
    assert_eq!(bread.minutes, 30);
    assert_eq!(bread.total_weight(), 100);
    assert!(
        bread.outcomes[2].outputs.is_empty(),
        "a failure branch yields nothing"
    );
}

#[test]
fn bad_recipes_are_refused_with_their_reason() {
    let catalogue = catalogue();
    let read = |recipe: &str| {
        Recipes::from_json(
            &format!(r#"{{ "stations": ["oven"], "recipes": [ {recipe} ] }}"#),
            &catalogue,
        )
    };
    let good_outcome = r#""outcomes": [ { "name": "ok", "weight": 1 } ]"#;
    let cases = [
        (format!(r#"{{ "name": "r", "station": "kiln", "minutes": 1, "inputs": [], {good_outcome} }}"#), RecipesError::UnknownStation { recipe: "r".into(), station: "kiln".into() }),
        (format!(r#"{{ "name": "r", "station": "oven", "minutes": 1, "inputs": [ {{ "kind": "salt", "count": 1 }} ], {good_outcome} }}"#), RecipesError::UnknownKind { recipe: "r".into(), kind: "salt".into() }),
        (format!(r#"{{ "name": "r", "station": "oven", "minutes": 1, "inputs": [ {{ "kind": "flour", "count": 0 }} ], {good_outcome} }}"#), RecipesError::NoUnits("r".into())),
        (r#"{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], "outcomes": [ { "name": "ok", "weight": 1, "outputs": [ { "kind": "dough", "count": 5 } ] } ] }"#.to_string(), RecipesError::OverStack { recipe: "r".into(), kind: "dough".into() }),
        (r#"{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], "outcomes": [] }"#.to_string(), RecipesError::Weights("r".into())),
        (r#"{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], "outcomes": [ { "name": "a", "weight": 0 } ] }"#.to_string(), RecipesError::Weights("r".into())),
        (format!(r#"{{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], {good_outcome} }}, {{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], {good_outcome} }}"#), RecipesError::DuplicateName("r".into())),
    ];
    let cases = cases.into_iter().chain([
        (r#"{ "name": "r", "station": "oven", "minutes": 1, "inputs": [], "outcomes": [ { "name": "a", "weight": 1000001 } ] }"#.to_string(), RecipesError::Weights("r".into())),
        (format!(r#"{{ "name": "r", "station": "oven", "minutes": 32768, "inputs": [], {good_outcome} }}"#), RecipesError::TooLong("r".into())),
    ]);
    for (recipe, error) in cases {
        assert_eq!(read(&recipe), Err(error), "{recipe}");
    }
}
