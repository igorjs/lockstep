// SPDX-License-Identifier: Apache-2.0
//! The capsule: a game-shaped consumer scenario.
//!
//! One survivor lives in a small room with a wall across the middle. The host clicks a cell and the
//! survivor walks there along the cheapest path around the wall, one cell every few steps.
//!
//! Health, hunger, thirst and sanity are attributes read from `data/attributes.json`. Their decay is
//! three effects that drain per game minute, with no system code. A wound may bleed (a roll), a
//! bandage removes every bleed, and a prayer restores sanity up to a daily budget. When hunger or
//! thirst runs out, starvation drains health, and at zero the survivor dies. Everything the
//! survivor is lives in columns keyed by a handle.

pub mod script;

use lockstep_attributes::{
    AttributeEvent, AttributeId, Attributes, Effect, EffectContext, EffectTag, Effects, Registry,
    Stacking,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{
    hash_of, Chance, ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation,
    StableVector, StepConfiguration, Streams,
};
use lockstep_spatial::{
    Cell as GridCell, GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Square8,
};
use serde::{Deserialize, Serialize};

pub const ROOM_SIZE: i32 = 16;
pub const STEPS_PER_CELL_WALKING: u32 = 6;
pub const STEPS_PER_CELL_RUNNING: u32 = 3;
/// The wall across the room: the column `WALL_COLUMN`, from `WALL_FIRST_ROW` to `WALL_LAST_ROW`.
/// There is a gap above it and a gap below it.
pub const WALL_COLUMN: i32 = 8;
pub const WALL_FIRST_ROW: i32 = 2;
pub const WALL_LAST_ROW: i32 = 12;

/// The attributes, as data. A save holds the values; this file says what they are.
pub const ATTRIBUTES_JSON: &str = include_str!("../data/attributes.json");

/// The chance that a wound bleeds.
pub const BLEED_CHANCE: Chance = Chance(6_000);

pub fn registry() -> Registry {
    Registry::from_json(ATTRIBUTES_JSON).expect("the committed attributes are valid")
}

/// The attribute ids the capsule uses, looked up once by name.
#[derive(Clone, Copy, Debug)]
pub struct Ids {
    pub health: AttributeId,
    pub hunger: AttributeId,
    pub thirst: AttributeId,
    pub sanity: AttributeId,
}

impl Ids {
    pub fn of(registry: &Registry) -> Self {
        let id = |name: &str| {
            registry
                .id(name)
                .expect("the capsule's attributes are defined")
        };
        Ids {
            health: id("health"),
            hunger: id("hunger"),
            thirst: id("thirst"),
            sanity: id("sanity"),
        }
    }
}

fn drain(attribute: AttributeId, tag: &str, numerator: i32, denominator: i32) -> Effect {
    Effect {
        attribute,
        modifier: None,
        per_minute: Some(Fixed32::from_ratio(numerator, denominator)),
        remaining_minutes: None,
        tag: EffectTag::new(tag),
        stacking: Stacking::Independent,
    }
}

/// Needs decay: a survivor with full thirst runs dry in about eleven game hours, and with full
/// hunger starts starving in about seventeen.
pub fn decay(ids: Ids) -> [Effect; 3] {
    [
        drain(ids.hunger, "hunger", -1, 10),
        drain(ids.thirst, "thirst", -3, 20),
        drain(ids.sanity, "dread", -1, 50),
    ]
}

/// Health lost while hunger or thirst is empty, until the survivor dies.
pub fn starvation(ids: Ids) -> Effect {
    Effect {
        stacking: Stacking::RefreshDuration,
        ..drain(ids.health, "starvation", -1, 2)
    }
}

/// One bleeding wound: a point of health a minute for half an hour. Wounds stack.
pub fn bleeding(ids: Ids) -> Effect {
    Effect {
        remaining_minutes: Some(Fixed32::from_int(30)),
        ..drain(ids.health, "bleeding", -1, 1)
    }
}

/// A prayer restores a point of sanity a minute for twenty minutes, but no more than ten a day.
pub fn prayer(ids: Ids) -> Effect {
    Effect {
        remaining_minutes: Some(Fixed32::from_int(20)),
        stacking: Stacking::DailyBudget {
            cap_per_day: Fixed32::from_int(10),
        },
        ..drain(ids.sanity, "prayer", 1, 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Walk {
    pub destination: Cell,
    pub run: bool,
    pub steps_until_next_cell: u32,
    /// The cells still to walk, in order, as indexes into the room map. The first is next.
    pub path: Vec<u32>,
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
    /// A wound, which bleeds or not by a roll.
    Wound {
        entity: Handle,
    },
    /// Removes every bleeding wound.
    Bandage {
        entity: Handle,
    },
    Pray {
        entity: Handle,
    },
}

impl Message for Intent {
    const VERSION: u32 = 2;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Arrived {
        entity: Handle,
        cell: Cell,
    },
    Hungry {
        entity: Handle,
    },
    Starving {
        entity: Handle,
    },
    Died {
        entity: Handle,
        day: u32,
    },
    Rejected {
        entity: Handle,
    },
    /// A wound that bleeds.
    Bleeding {
        entity: Handle,
    },
    /// A wound that does not.
    Grazed {
        entity: Handle,
    },
    Bandaged {
        entity: Handle,
        wounds: usize,
    },
    Prayed {
        entity: Handle,
    },
}

impl Message for Event {
    const VERSION: u32 = 2;
}

/// The whole world. It is also the snapshot, so a save is exactly this value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    /// The room: sixteen by sixteen cells with a wall across the middle.
    pub room: GridMap<Square8>,
    pub entities: StableVector<String>,
    pub positions: Column<Cell>,
    pub walks: Column<Walk>,
    pub attributes: Column<Attributes>,
    pub effects: Effects,
}

impl Message for World {
    const VERSION: u32 = 2;
}

impl World {
    /// Despawning removes the entity and unsets every column.
    pub fn despawn(&mut self, entity: Handle) {
        self.entities.remove(entity);
        self.positions.unset(entity);
        self.walks.unset(entity);
        self.attributes.unset(entity);
        self.effects.remove_entity(entity);
    }

    /// Every column entry must belong to a living entity.
    pub fn orphans(&self) -> Vec<Handle> {
        let mut found = Vec::new();
        let columns: [Vec<Handle>; 3] = [
            self.positions.handles(),
            self.walks.handles(),
            self.attributes.handles(),
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

    /// A survivor's attribute value, if alive.
    pub fn value(&self, entity: Handle, attribute: AttributeId) -> Option<Fixed32> {
        Some(self.attributes.get(entity)?.get(attribute).current())
    }
}

/// The room for a new world.
pub fn build_room() -> GridMap<Square8> {
    let mut room: GridMap<Square8> = GridMap::new(ROOM_SIZE as u32, ROOM_SIZE as u32);
    for y in WALL_FIRST_ROW..=WALL_LAST_ROW {
        room.set_passable(room.index(WALL_COLUMN as u32, y as u32), false);
    }
    room
}

pub struct Capsule {
    world: World,
    /// Configuration data, loaded again on restore; not part of the state.
    registry: Registry,
    ids: Ids,
    /// Search buffers, reused for every move. They are not part of the state.
    pathfinder: Pathfinder,
    path: Vec<GridCell>,
    /// No bodies block each other yet, so this is empty; the pathfinder needs one.
    occupancy: Occupancy,
}

impl Capsule {
    fn with_world(world: World) -> Self {
        let pathfinder = Pathfinder::new(&world.room);
        let occupancy = Occupancy::new(&world.room);
        let registry = registry();
        let ids = Ids::of(&registry);
        Capsule {
            world,
            registry,
            ids,
            pathfinder,
            path: Vec::new(),
            occupancy,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn ids(&self) -> Ids {
        self.ids
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
            room: build_room(),
            entities: StableVector::new(),
            positions: Column::new(),
            walks: Column::new(),
            attributes: Column::new(),
            effects: Effects::new(),
        };
        let survivor = world.entities.insert(configuration.survivor_name);
        // Spawn on open ground: draw again if the draw lands on the wall.
        let spawn = loop {
            let cell = Cell {
                x: randomness.range("spawn", 0, ROOM_SIZE),
                y: randomness.range("spawn", 0, ROOM_SIZE),
            };
            if world.room.is_passable(room_cell(&world.room, cell)) {
                break cell;
            }
        };
        world.positions.set(survivor, spawn);
        let mut capsule = Capsule::with_world(world);
        let world = &mut capsule.world;
        world
            .attributes
            .set(survivor, Attributes::from_registry(&capsule.registry));
        let mut effect_context = EffectContext {
            attributes: &mut world.attributes,
            registry: &capsule.registry,
            attribute_events: &mut Vec::new(),
            effect_events: &mut Vec::new(),
        };
        for effect in decay(capsule.ids) {
            world.effects.add(survivor, effect, &mut effect_context);
        }
        capsule
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut attribute_events = Vec::new();
        apply_intents(self, intents, context, &mut attribute_events);
        walking_system(&mut self.world, context);
        effects_system(self, context, &mut attribute_events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Capsule::with_world(snapshot)
    }
}

fn inside_room(cell: Cell) -> bool {
    (0..ROOM_SIZE).contains(&cell.x) && (0..ROOM_SIZE).contains(&cell.y)
}

/// The room map's cell for a position inside the room.
fn room_cell(room: &GridMap<Square8>, cell: Cell) -> GridCell {
    room.index(cell.x as u32, cell.y as u32)
}

fn position_of_room_cell(room: &GridMap<Square8>, cell: GridCell) -> Cell {
    let (x, y) = room.coordinates(cell);
    Cell {
        x: x as i32,
        y: y as i32,
    }
}

fn apply_intents(
    capsule: &mut Capsule,
    intents: &[Intent],
    context: &mut Context<'_, Capsule>,
    attribute_events: &mut Vec<AttributeEvent>,
) {
    let Capsule {
        world,
        registry,
        ids,
        pathfinder,
        path,
        occupancy,
    } = capsule;
    for intent in intents {
        match intent {
            Intent::MoveTo { entity, cell, run } => {
                let from = world.positions.get(*entity).copied();
                let reachable = match from {
                    Some(from) if world.entities.contains(*entity) && inside_room(*cell) => {
                        let (from, to) =
                            (room_cell(&world.room, from), room_cell(&world.room, *cell));
                        world.room.is_passable(to)
                            && matches!(
                                pathfinder.find(
                                    &world.room,
                                    occupancy,
                                    from,
                                    to,
                                    PathOptions::default(),
                                    path
                                ),
                                PathResult::Found { .. }
                            )
                    }
                    _ => false,
                };
                if !reachable {
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
                        path: path.iter().map(|step| step.0).collect(),
                    },
                );
            }
            Intent::Stop { entity } => {
                world.walks.unset(*entity);
            }
            Intent::Wound { entity } | Intent::Bandage { entity } | Intent::Pray { entity } => {
                if !world.entities.contains(*entity) {
                    context.events.push(Event::Rejected { entity: *entity });
                    continue;
                }
                let mut effect_context = EffectContext {
                    attributes: &mut world.attributes,
                    registry,
                    attribute_events: &mut *attribute_events,
                    effect_events: &mut Vec::new(),
                };
                let event = match intent {
                    Intent::Wound { .. } => {
                        if context.randomness.roll("wounds", BLEED_CHANCE) {
                            world
                                .effects
                                .add(*entity, bleeding(*ids), &mut effect_context);
                            Event::Bleeding { entity: *entity }
                        } else {
                            Event::Grazed { entity: *entity }
                        }
                    }
                    Intent::Bandage { .. } => Event::Bandaged {
                        entity: *entity,
                        wounds: world.effects.remove_by_tag(
                            *entity,
                            &EffectTag::new("bleeding"),
                            &mut effect_context,
                        ),
                    },
                    _ => {
                        world
                            .effects
                            .add(*entity, prayer(*ids), &mut effect_context);
                        Event::Prayed { entity: *entity }
                    }
                };
                context.events.push(event);
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
        if !walk.path.is_empty() {
            *position = position_of_room_cell(&world.room, GridCell(walk.path.remove(0)));
        }
        if walk.path.is_empty() {
            arrived.push((entity, *position));
        }
    }
    for (entity, cell) in arrived {
        world.walks.unset(entity);
        context.events.push(Event::Arrived { entity, cell });
    }
}

/// Runs every effect for the step's game minutes, then turns what the attributes report into the
/// capsule's events: the hunger warning, starvation when hunger or thirst runs out, and death.
fn effects_system(
    capsule: &mut Capsule,
    context: &mut Context<'_, Capsule>,
    attribute_events: &mut Vec<AttributeEvent>,
) {
    let Capsule {
        world,
        registry,
        ids,
        ..
    } = capsule;
    let mut effect_context = EffectContext {
        attributes: &mut world.attributes,
        registry,
        attribute_events: &mut *attribute_events,
        effect_events: &mut Vec::new(),
    };
    world.effects.tick(
        context.elapsed_minutes,
        context.clock.day(),
        &mut effect_context,
    );
    let mut dead = Vec::new();
    for event in attribute_events.drain(..) {
        match event {
            AttributeEvent::Crossed {
                who,
                threshold,
                upward: false,
                ..
            } if threshold == "hungry" => context.events.push(Event::Hungry { entity: who }),
            AttributeEvent::Emptied { who, attribute }
                if attribute == ids.hunger || attribute == ids.thirst =>
            {
                let starving = EffectTag::new("starvation");
                if !world.effects.has(who, &starving) && world.entities.contains(who) {
                    let mut effect_context = EffectContext {
                        attributes: &mut world.attributes,
                        registry,
                        attribute_events: &mut Vec::new(),
                        effect_events: &mut Vec::new(),
                    };
                    world
                        .effects
                        .add(who, starvation(*ids), &mut effect_context);
                    context.events.push(Event::Starving { entity: who });
                }
            }
            AttributeEvent::Emptied { who, attribute }
                if attribute == ids.health && !dead.contains(&who) =>
            {
                dead.push(who);
            }
            _ => {}
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
