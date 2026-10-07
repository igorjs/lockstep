// SPDX-License-Identifier: Apache-2.0
use lockstep_inventory::{Catalogue, KindId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct RecipeId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct StationId(pub u16);

/// Some units of a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub kind: KindId,
    pub count: u16,
}

/// One row of an outcome table: its weight, and what it yields. A row that yields nothing, or
/// less than the recipe hoped, is a failure branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub name: String,
    pub weight: u32,
    pub outputs: Vec<Amount>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub station: StationId,
    pub inputs: Vec<Amount>,
    /// Whole game minutes the job takes.
    pub minutes: u32,
    pub outcomes: Vec<Outcome>,
}

impl Recipe {
    pub fn total_weight(&self) -> u32 {
        self.outcomes.iter().map(|outcome| outcome.weight).sum()
    }
}

/// Every station kind and recipe, in a stable order. Build it with `Recipes::from_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipes {
    pub stations: Vec<String>,
    pub recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipesError {
    Json(String),
    TooMany,
    DuplicateName(String),
    UnknownStation {
        recipe: String,
        station: String,
    },
    UnknownKind {
        recipe: String,
        kind: String,
    },
    /// An input or output of no units.
    NoUnits(String),
    /// More units of an output than one item of its kind holds.
    OverStack {
        recipe: String,
        kind: String,
    },
    /// No outcomes, or weights that add to nothing or past four billion.
    Weights(String),
}

impl std::fmt::Display for RecipesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RecipesError {}

impl Recipes {
    /// Reads the data form documented in `docs/lockstep-crafting.md`. Kinds name kinds in the
    /// inventory catalogue.
    pub fn from_json(text: &str, catalogue: &Catalogue) -> Result<Self, RecipesError> {
        let file: file::RecipesFile =
            serde_json::from_str(text).map_err(|error| RecipesError::Json(error.to_string()))?;
        file.into_recipes(catalogue)
    }

    pub fn recipe(&self, id: RecipeId) -> Option<&Recipe> {
        self.recipes.get(id.0 as usize)
    }

    pub fn recipe_id(&self, name: &str) -> Option<RecipeId> {
        self.recipes
            .iter()
            .position(|recipe| recipe.name == name)
            .map(|index| RecipeId(index as u16))
    }

    pub fn station_id(&self, name: &str) -> Option<StationId> {
        self.stations
            .iter()
            .position(|station| station == name)
            .map(|index| StationId(index as u16))
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RecipesFile {
        stations: Vec<String>,
        recipes: Vec<RecipeFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RecipeFile {
        name: String,
        station: String,
        inputs: Vec<AmountFile>,
        minutes: u32,
        outcomes: Vec<OutcomeFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AmountFile {
        kind: String,
        count: u16,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct OutcomeFile {
        name: String,
        weight: u32,
        #[serde(default)]
        outputs: Vec<AmountFile>,
    }

    fn unique<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), RecipesError> {
        let mut seen = BTreeSet::new();
        for name in names {
            if !seen.insert(name) {
                return Err(RecipesError::DuplicateName(name.to_string()));
            }
        }
        Ok(())
    }

    impl RecipesFile {
        pub(super) fn into_recipes(self, catalogue: &Catalogue) -> Result<Recipes, RecipesError> {
            if self.stations.len() > u16::MAX as usize || self.recipes.len() > u16::MAX as usize {
                return Err(RecipesError::TooMany);
            }
            unique(self.stations.iter().map(String::as_str))?;
            unique(self.recipes.iter().map(|recipe| recipe.name.as_str()))?;
            let stations = self.stations;
            let mut recipes = Vec::new();
            for file in self.recipes {
                let station = stations
                    .iter()
                    .position(|station| *station == file.station)
                    .map(|index| StationId(index as u16))
                    .ok_or_else(|| RecipesError::UnknownStation {
                        recipe: file.name.clone(),
                        station: file.station.clone(),
                    })?;
                let amount = |amount: &AmountFile, output: bool| {
                    let kind = catalogue.kind_id(&amount.kind).ok_or_else(|| {
                        RecipesError::UnknownKind {
                            recipe: file.name.clone(),
                            kind: amount.kind.clone(),
                        }
                    })?;
                    if amount.count == 0 {
                        return Err(RecipesError::NoUnits(file.name.clone()));
                    }
                    if output && amount.count > catalogue.kind(kind).stack {
                        return Err(RecipesError::OverStack {
                            recipe: file.name.clone(),
                            kind: amount.kind.clone(),
                        });
                    }
                    Ok(Amount {
                        kind,
                        count: amount.count,
                    })
                };
                let inputs = file
                    .inputs
                    .iter()
                    .map(|input| amount(input, false))
                    .collect::<Result<Vec<_>, _>>()?;
                let outcomes = file
                    .outcomes
                    .iter()
                    .map(|outcome| {
                        Ok(Outcome {
                            name: outcome.name.clone(),
                            weight: outcome.weight,
                            outputs: outcome
                                .outputs
                                .iter()
                                .map(|output| amount(output, true))
                                .collect::<Result<Vec<_>, _>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>, RecipesError>>()?;
                let total: u64 = outcomes.iter().map(|outcome| outcome.weight as u64).sum();
                if total == 0 || total > i32::MAX as u64 {
                    return Err(RecipesError::Weights(file.name));
                }
                recipes.push(Recipe {
                    name: file.name,
                    station,
                    inputs,
                    minutes: file.minutes,
                    outcomes,
                });
            }
            Ok(Recipes { stations, recipes })
        }
    }
}
