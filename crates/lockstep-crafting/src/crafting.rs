// SPDX-License-Identifier: Apache-2.0
use crate::recipes::{RecipeId, Recipes, StationId};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, Message, Streams};
use lockstep_inventory::{Catalogue, Inventory, KindId, Place};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The stream outcomes are drawn from.
pub const STREAM: &str = "crafting";

/// A job running at a station: the recipe, and the raw 16.16 game minutes still to go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub recipe: RecipeId,
    remaining: u64,
}

impl Job {
    /// Game minutes still to go.
    pub fn remaining_minutes(&self) -> Fixed32 {
        Fixed32::from_raw(self.remaining.min(i32::MAX as u64) as i32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    UnknownStation,
    UnknownRecipe,
    /// The recipe needs another kind of station.
    WrongStation,
    /// The station is already working.
    Busy,
    /// The source holds too few usable units of an input; nothing was consumed.
    Missing {
        kind: KindId,
        needed: u32,
        have: u32,
    },
    UnknownContainer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum CraftingEvent {
    Started {
        station: Handle,
        recipe: RecipeId,
    },
    /// The job ended with this row of its outcome table.
    Finished {
        station: Handle,
        recipe: RecipeId,
        outcome: u16,
    },
    /// An output went into the station's container.
    Produced {
        station: Handle,
        item: Handle,
    },
    /// The station's container had no room: the output is left loose.
    Overflowed {
        station: Handle,
        item: Handle,
    },
}

/// Stations and the jobs running at them. A station is an entity with a station kind and a
/// container in the inventory, which receives its outputs; one job runs at a time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Crafting {
    stations: Column<StationId>,
    jobs: Column<Job>,
}

impl Crafting {
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes an entity a station of a kind. It needs a container in the inventory for outputs.
    pub fn add_station(&mut self, station: Handle, kind: StationId) {
        self.stations.set(station, kind);
    }

    pub fn job(&self, station: Handle) -> Option<&Job> {
        self.jobs.get(station)
    }

    /// Usable units of a kind in a container: every unit of every item of the kind that has not
    /// spoiled.
    pub fn available(inventory: &Inventory, container: Handle, kind: KindId) -> u32 {
        inventory.container(container).map_or(0, |held| {
            held.items()
                .iter()
                .filter_map(|item| inventory.item(*item))
                .filter(|item| item.kind == kind && !item.spoiled)
                .map(|item| item.count as u32)
                .sum()
        })
    }

    /// Starts a recipe at a station, taking its inputs from a container, all or nothing: when any
    /// input is short (spoiled units do not count), nothing is consumed and the first short input
    /// is named. Inputs are taken from the container's items in order.
    pub fn start(
        &mut self,
        inventory: &mut Inventory,
        recipes: &Recipes,
        station: Handle,
        recipe: RecipeId,
        source: Handle,
        events: &mut Vec<CraftingEvent>,
    ) -> Result<(), Refusal> {
        let kind = *self.stations.get(station).ok_or(Refusal::UnknownStation)?;
        let definition = recipes.recipe(recipe).ok_or(Refusal::UnknownRecipe)?;
        if definition.station != kind {
            return Err(Refusal::WrongStation);
        }
        if self.jobs.has(station) {
            return Err(Refusal::Busy);
        }
        let Some(container) = inventory.container(source) else {
            return Err(Refusal::UnknownContainer);
        };
        // Needs by kind, in the order kinds first appear, so a kind listed twice is counted once.
        let mut needs: Vec<(KindId, u32)> = Vec::new();
        for input in &definition.inputs {
            match needs.iter_mut().find(|(kind, _)| *kind == input.kind) {
                Some((_, needed)) => *needed += input.count as u32,
                None => needs.push((input.kind, input.count as u32)),
            }
        }
        for (kind, needed) in &needs {
            let have = Self::available(inventory, source, *kind);
            if have < *needed {
                return Err(Refusal::Missing {
                    kind: *kind,
                    needed: *needed,
                    have,
                });
            }
        }
        // Every input is there: now take them.
        let items: Vec<Handle> = container.items().to_vec();
        let mut left: BTreeMap<KindId, u32> = needs.into_iter().collect();
        for item in items {
            let Some(held) = inventory.item(item) else {
                continue;
            };
            if held.spoiled {
                continue;
            }
            let Some(needed) = left.get_mut(&held.kind) else {
                continue;
            };
            let take = (*needed).min(held.count as u32) as u16;
            if take > 0 {
                inventory.consume(item, take).expect("checked available");
                *needed -= take as u32;
            }
        }
        self.jobs.set(
            station,
            Job {
                recipe,
                remaining: definition.minutes as u64 * 65_536,
            },
        );
        events.push(CraftingEvent::Started { station, recipe });
        Ok(())
    }

    /// Lets `minutes` of game time pass. Each job that runs out ends, in station handle order:
    /// it draws its outcome from the `"crafting"` stream by weight, and puts the outcome's
    /// outputs into the station's container, or leaves them loose when it has no room.
    pub fn tick(
        &mut self,
        inventory: &mut Inventory,
        catalogue: &Catalogue,
        recipes: &Recipes,
        minutes: Fixed32,
        streams: &mut Streams,
        events: &mut Vec<CraftingEvent>,
    ) {
        let minutes = minutes.raw().max(0) as u64;
        let mut done = Vec::new();
        for (station, job) in self.jobs.iter_mut() {
            job.remaining = job.remaining.saturating_sub(minutes);
            if job.remaining == 0 {
                done.push((station, job.recipe));
            }
        }
        for (station, recipe) in done {
            self.jobs.unset(station);
            let Some(definition) = recipes.recipe(recipe) else {
                continue;
            };
            let outcome = roll(
                definition.outcomes.iter().map(|outcome| outcome.weight),
                streams,
            );
            events.push(CraftingEvent::Finished {
                station,
                recipe,
                outcome: outcome as u16,
            });
            for output in &definition.outcomes[outcome].outputs {
                let item = inventory.create(catalogue, output.kind, output.count);
                match inventory.put(item, station, catalogue) {
                    Ok(into) => events.push(CraftingEvent::Produced {
                        station,
                        item: into,
                    }),
                    Err(_) => {
                        debug_assert_eq!(inventory.place(item), Some(Place::Loose));
                        events.push(CraftingEvent::Overflowed { station, item });
                    }
                }
            }
        }
    }
}

/// Draws a row by weight: one number below the total, then the row whose running total passes it.
/// A row of weight zero is never drawn.
pub fn roll(weights: impl Iterator<Item = u32> + Clone, streams: &mut Streams) -> usize {
    let total: u64 = weights.clone().map(u64::from).sum();
    let draw = streams.range(STREAM, 0, total.min(i32::MAX as u64) as i32) as u64;
    let mut running = 0;
    for (index, weight) in weights.enumerate() {
        running += weight as u64;
        if draw < running {
            return index;
        }
    }
    unreachable!("the draw is below the total")
}
