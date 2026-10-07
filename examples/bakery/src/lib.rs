// SPDX-License-Identifier: Apache-2.0
//! The bakery: a consumer scenario for `lockstep-crafting` that is not a game.
//!
//! Flour, water and yeast are delivered to a pantry. A mixer turns two flour, a water and a yeast
//! into dough, which sometimes comes out flat; an oven turns a dough into two loaves, which are
//! sometimes burnt to charcoal or collapse. The recipes are in `data/recipes.json` and the kinds,
//! which spoil at room temperature, in `data/catalogue.json`. Bread is sold off the oven's rack.

use lockstep_attributes::Registry;
use lockstep_core::{
    ClockConfiguration, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_crafting::{Crafting, CraftingEvent, Recipes, Refusal};
use lockstep_inventory::{Catalogue, Container, Inventory, InventoryEvent, KindId};
use serde::{Deserialize, Serialize};

pub const CATALOGUE_JSON: &str = include_str!("../data/catalogue.json");
pub const RECIPES_JSON: &str = include_str!("../data/recipes.json");

/// The bakery's temperature, in whole degrees.
pub const ROOM: i32 = 22;

pub fn catalogue() -> Catalogue {
    let registry = Registry::from_json(r#"{ "attributes": [] }"#).expect("an empty registry");
    Catalogue::from_json(CATALOGUE_JSON, &registry).expect("the committed catalogue is valid")
}

pub fn recipes(catalogue: &Catalogue) -> Recipes {
    Recipes::from_json(RECIPES_JSON, catalogue).expect("the committed recipes are valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub pantry_slots: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// Units of a kind arrive at the pantry.
    Deliver { kind: KindId, count: u16 },
    /// The mixer makes dough from the pantry.
    Mix,
    /// The oven bakes the dough the mixer holds.
    Bake,
    /// One loaf leaves the oven's rack.
    Sell,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Delivered {
        kind: KindId,
        count: u16,
    },
    /// The pantry had no room for a delivery.
    Turned {
        kind: KindId,
    },
    Crafting(CraftingEvent),
    Refused {
        reason: Refusal,
    },
    Inventory(InventoryEvent),
    Sold,
    NothingToSell,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Place {
    Pantry,
    Mixer,
    Oven,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub places: StableVector<Place>,
    pub inventory: Inventory,
    pub crafting: Crafting,
    pub sold: u32,
}

pub struct Bakery {
    world: World,
    catalogue: Catalogue,
    recipes: Recipes,
}

impl Bakery {
    fn with_world(world: World) -> Self {
        let catalogue = catalogue();
        let recipes = recipes(&catalogue);
        Bakery {
            world,
            catalogue,
            recipes,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    pub fn recipes(&self) -> &Recipes {
        &self.recipes
    }

    pub fn place(&self, which: Place) -> Handle {
        self.world
            .places
            .iter()
            .find(|(_, place)| **place == which)
            .map(|(handle, _)| handle)
            .expect("every place exists")
    }

    /// Usable units of a kind somewhere.
    pub fn units(&self, name: &str, at: Place) -> u32 {
        Crafting::available(
            &self.world.inventory,
            self.place(at),
            self.catalogue.kind_id(name).expect("a known kind"),
        )
    }

    fn start(&mut self, station: Place, recipe: &str, source: Place, events: &mut Vec<Event>) {
        let (station, source) = (self.place(station), self.place(source));
        let recipe = self.recipes.recipe_id(recipe).expect("a known recipe");
        let mut crafting_events = Vec::new();
        let outcome = self.world.crafting.start(
            &mut self.world.inventory,
            &self.recipes,
            station,
            recipe,
            source,
            &mut crafting_events,
        );
        if let Err(reason) = outcome {
            events.push(Event::Refused { reason });
        }
        events.extend(crafting_events.into_iter().map(Event::Crafting));
    }
}

impl Simulation for Bakery {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut bakery = Bakery::with_world(World {
            places: StableVector::new(),
            inventory: Inventory::new(),
            crafting: Crafting::new(),
            sold: 0,
        });
        let world = &mut bakery.world;
        let pantry = world.places.insert(Place::Pantry);
        let mixer = world.places.insert(Place::Mixer);
        let oven = world.places.insert(Place::Oven);
        for (owner, slots) in [(pantry, configuration.pantry_slots), (mixer, 4), (oven, 8)] {
            world
                .inventory
                .add_container(owner, Container::new(slots, None, ROOM))
                .expect("a new container");
        }
        let (mixer_kind, oven_kind) = (
            bakery.recipes.station_id("mixer").expect("defined"),
            bakery.recipes.station_id("oven").expect("defined"),
        );
        bakery.world.crafting.add_station(mixer, mixer_kind);
        bakery.world.crafting.add_station(oven, oven_kind);
        bakery
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        for intent in intents {
            match *intent {
                Intent::Deliver { kind, count } => {
                    let valid = (kind.0 as usize) < self.catalogue.kind_count()
                        && count >= 1
                        && count <= self.catalogue.kind(kind).stack;
                    if !valid {
                        events.push(Event::Turned { kind });
                        continue;
                    }
                    let item = self.world.inventory.create(&self.catalogue, kind, count);
                    let pantry = self.place(Place::Pantry);
                    match self.world.inventory.put(item, pantry, &self.catalogue) {
                        Ok(_) => events.push(Event::Delivered { kind, count }),
                        Err(_) => {
                            self.world.inventory.destroy(item).expect("just made");
                            events.push(Event::Turned { kind });
                        }
                    }
                }
                Intent::Mix => self.start(Place::Mixer, "dough", Place::Pantry, &mut events),
                Intent::Bake => self.start(Place::Oven, "bread", Place::Mixer, &mut events),
                Intent::Sell => {
                    let oven = self.place(Place::Oven);
                    let loaf = self
                        .world
                        .inventory
                        .find_by_tag(oven, "baked", &self.catalogue)
                        .into_iter()
                        .find(|item| {
                            self.world
                                .inventory
                                .item(*item)
                                .is_some_and(|held| !held.spoiled)
                        });
                    match loaf {
                        Some(loaf) => {
                            self.world.inventory.consume(loaf, 1).expect("a loaf");
                            self.world.sold += 1;
                            events.push(Event::Sold);
                        }
                        None => events.push(Event::NothingToSell),
                    }
                }
            }
        }
        let mut spoiled = Vec::new();
        self.world
            .inventory
            .spoil(context.elapsed_minutes, ROOM, &self.catalogue, &mut spoiled);
        events.extend(spoiled.into_iter().map(Event::Inventory));
        let mut crafting_events = Vec::new();
        self.world.crafting.tick(
            &mut self.world.inventory,
            &self.catalogue,
            &self.recipes,
            context.elapsed_minutes,
            context.randomness,
            &mut crafting_events,
        );
        events.extend(crafting_events.into_iter().map(Event::Crafting));
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Bakery::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_013;
pub const DEFAULT_STEPS: u64 = 54_000;

pub fn shop() -> Configuration {
    Configuration { pantry_slots: 12 }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Bakery> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 300,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: deliveries now and then, the mixer and oven
/// asked to work whether or not they can, and a loaf sold when a customer comes in.
pub fn script(bakery: &Bakery, script: &mut Streams) -> Vec<Intent> {
    let kind = |name: &str| bakery.catalogue().kind_id(name).expect("defined");
    match script.range("act", 0, 600) {
        0 => vec![Intent::Deliver {
            kind: kind("flour"),
            count: 6,
        }],
        1 => vec![Intent::Deliver {
            kind: kind("water"),
            count: 4,
        }],
        2 => vec![Intent::Deliver {
            kind: kind("yeast"),
            count: 3,
        }],
        3..=6 => vec![Intent::Mix],
        7..=12 => vec![Intent::Bake],
        13..=16 => vec![Intent::Sell],
        _ => Vec::new(),
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Bakery> {
    let mut runner = runner(shop(), seed);
    let mut streams = Streams::new(seed ^ 0x0062_616b_6572);
    for _ in 0..steps {
        let intents = script(runner.simulation(), &mut streams);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
