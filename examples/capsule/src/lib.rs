//! The capsule: a game-shaped consumer scenario.
//!
//! One survivor lives in a small room. The host clicks a cell and the survivor walks there, one
//! cell every few steps. Needs drain with game time. When hunger reaches zero, health drains,
//! and the survivor starves. Everything the survivor is lives in columns keyed by a handle.

pub mod script;

use lockstep_core::{
    hash_of, ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation,
    StableVector, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

pub const ROOM_SIZE: i32 = 16;
pub const STEPS_PER_CELL_WALKING: u32 = 6;
pub const STEPS_PER_CELL_RUNNING: u32 = 3;

/// Drain per game minute. A survivor with full hunger starves in about sixteen game hours.
const HUNGER_PER_MINUTE: f32 = 0.1;
const THIRST_PER_MINUTE: f32 = 0.15;
const SANITY_PER_MINUTE: f32 = 0.02;
/// Health lost per game minute while hunger or thirst is empty.
const STARVATION_PER_MINUTE: f32 = 0.5;
const HUNGRY_BELOW: f32 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

/// Three numbers read together, so they live in one column.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub thirst: f32,
    pub sanity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Walk {
    pub destination: Cell,
    pub run: bool,
    pub steps_until_next_cell: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub survivor_name: String,
}

impl Message for Configuration {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    MoveTo {
        entity: Handle,
        cell: Cell,
        run: bool,
    },
    Stop {
        entity: Handle,
    },
}

impl Message for Intent {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Arrived { entity: Handle, cell: Cell },
    Hungry { entity: Handle },
    Starving { entity: Handle },
    Died { entity: Handle, day: u32 },
    Rejected { entity: Handle },
}

impl Message for Event {
    const VERSION: u32 = 1;
}

/// The whole world. It is also the snapshot, so a save is exactly this value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub entities: StableVector<String>,
    pub positions: Column<Cell>,
    pub walks: Column<Walk>,
    pub needs: Column<Needs>,
    pub health: Column<f32>,
    pub hungry: Column<()>,
    pub starving: Column<()>,
}

impl Message for World {
    const VERSION: u32 = 1;
}

impl World {
    /// Despawning removes the entity and unsets every column.
    pub fn despawn(&mut self, entity: Handle) {
        self.entities.remove(entity);
        self.positions.unset(entity);
        self.walks.unset(entity);
        self.needs.unset(entity);
        self.health.unset(entity);
        self.hungry.unset(entity);
        self.starving.unset(entity);
    }

    /// Every column entry must belong to a living entity.
    pub fn orphans(&self) -> Vec<Handle> {
        let mut found = Vec::new();
        let columns: [Vec<Handle>; 6] = [
            self.positions.handles(),
            self.walks.handles(),
            self.needs.handles(),
            self.health.handles(),
            self.hungry.handles(),
            self.starving.handles(),
        ];
        for handles in columns {
            for handle in handles {
                if !self.entities.contains(handle) {
                    found.push(handle);
                }
            }
        }
        found
    }
}

pub struct Capsule {
    world: World,
}

impl Capsule {
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The first (and only) survivor, if alive.
    pub fn survivor(&self) -> Option<Handle> {
        self.world.entities.handles().first().copied()
    }
}

impl Simulation for Capsule {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, randomness: &mut Streams) -> Self {
        let mut world = World {
            entities: StableVector::new(),
            positions: Column::new(),
            walks: Column::new(),
            needs: Column::new(),
            health: Column::new(),
            hungry: Column::new(),
            starving: Column::new(),
        };
        let survivor = world.entities.insert(configuration.survivor_name);
        world.positions.set(
            survivor,
            Cell {
                x: randomness.range("spawn", 0, ROOM_SIZE),
                y: randomness.range("spawn", 0, ROOM_SIZE),
            },
        );
        world.needs.set(
            survivor,
            Needs {
                hunger: 100.0,
                thirst: 100.0,
                sanity: 100.0,
            },
        );
        world.health.set(survivor, 100.0);
        Capsule { world }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        apply_intents(&mut self.world, intents, context);
        walking_system(&mut self.world, context);
        needs_system(&mut self.world, context);
        starvation_system(&mut self.world, context);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Capsule { world: snapshot }
    }
}

fn inside_room(cell: Cell) -> bool {
    (0..ROOM_SIZE).contains(&cell.x) && (0..ROOM_SIZE).contains(&cell.y)
}

fn apply_intents(world: &mut World, intents: &[Intent], context: &mut Context<'_, Capsule>) {
    for intent in intents {
        match intent {
            Intent::MoveTo { entity, cell, run } => {
                if !world.entities.contains(*entity) || !inside_room(*cell) {
                    context.events.push(Event::Rejected { entity: *entity });
                    continue;
                }
                let steps = if *run {
                    STEPS_PER_CELL_RUNNING
                } else {
                    STEPS_PER_CELL_WALKING
                };
                world.walks.set(
                    *entity,
                    Walk {
                        destination: *cell,
                        run: *run,
                        steps_until_next_cell: steps,
                    },
                );
            }
            Intent::Stop { entity } => {
                world.walks.unset(*entity);
            }
        }
    }
}

fn walking_system(world: &mut World, context: &mut Context<'_, Capsule>) {
    let mut arrived = Vec::new();
    for (entity, walk) in world.walks.iter_mut() {
        let Some(position) = world.positions.get_mut(entity) else {
            continue;
        };
        walk.steps_until_next_cell -= 1;
        if walk.steps_until_next_cell > 0 {
            continue;
        }
        walk.steps_until_next_cell = if walk.run {
            STEPS_PER_CELL_RUNNING
        } else {
            STEPS_PER_CELL_WALKING
        };
        // One axis at a time, horizontal first, so every platform picks the same path.
        if position.x != walk.destination.x {
            position.x += (walk.destination.x - position.x).signum();
        } else if position.y != walk.destination.y {
            position.y += (walk.destination.y - position.y).signum();
        }
        if *position == walk.destination {
            arrived.push((entity, *position));
        }
    }
    for (entity, cell) in arrived {
        world.walks.unset(entity);
        context.events.push(Event::Arrived { entity, cell });
    }
}

fn needs_system(world: &mut World, context: &mut Context<'_, Capsule>) {
    let minutes = context.elapsed_game_minutes;
    for (entity, needs) in world.needs.iter_mut() {
        needs.hunger = (needs.hunger - HUNGER_PER_MINUTE * minutes).max(0.0);
        needs.thirst = (needs.thirst - THIRST_PER_MINUTE * minutes).max(0.0);
        needs.sanity = (needs.sanity - SANITY_PER_MINUTE * minutes).max(0.0);
        if needs.hunger < HUNGRY_BELOW && !world.hungry.has(entity) {
            world.hungry.set(entity, ());
            context.events.push(Event::Hungry { entity });
        }
        if (needs.hunger <= 0.0 || needs.thirst <= 0.0) && !world.starving.has(entity) {
            world.starving.set(entity, ());
            context.events.push(Event::Starving { entity });
        }
    }
}

fn starvation_system(world: &mut World, context: &mut Context<'_, Capsule>) {
    let loss = STARVATION_PER_MINUTE * context.elapsed_game_minutes;
    let mut dead = Vec::new();
    for (entity, _) in world.starving.iter() {
        if let Some(health) = world.health.get_mut(entity) {
            *health -= loss;
            if *health <= 0.0 {
                dead.push(entity);
            }
        }
    }
    for entity in dead {
        context.events.push(Event::Died {
            entity,
            day: context.clock.day(),
        });
        world.despawn(entity);
    }
}

pub fn clock_configuration() -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes: 120.0,
        sunrise_minute: 360,
        sunset_minute: 1080,
        starting_minute: 8 * 60,
        starting_day: 1,
    }
}

pub fn runner(seed: u64) -> Runner<Capsule> {
    Runner::new(
        Configuration {
            survivor_name: "Mara".to_string(),
        },
        seed,
        StepConfiguration::default(),
        clock_configuration(),
    )
}

/// The committed scripted session: `fixtures/capsule.intents`.
pub const SCRIPT_TEXT: &str = include_str!("../fixtures/capsule.intents");

fn committed_script() -> script::Script {
    script::parse(SCRIPT_TEXT).expect("the committed script parses")
}

/// The seed in the committed script. The file is the single source of truth.
pub fn default_seed() -> u64 {
    committed_script().seed
}

/// The step count in the committed script.
pub fn default_steps() -> u64 {
    committed_script().steps
}

/// The session the fixture hash covers. The script is the same on every platform because it
/// depends only on the step number.
pub fn run_fixture(seed: u64, steps: u64) -> Runner<Capsule> {
    script::run(&committed_script(), Some(seed), Some(steps))
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}

/// Hash of the world alone, for tests that compare snapshots.
pub fn world_hash(world: &World) -> u64 {
    hash_of(world)
}
