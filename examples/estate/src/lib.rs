// SPDX-License-Identifier: Apache-2.0
//! The estate: a consumer scenario for `lockstep-structures` and `lockstep-procgen` that is not
//! a game.
//!
//! A garden estate of 6 by 4 chunks is rolled from the templates in `data/templates.json`: the
//! cloister chunks on the middle row are fixed, the rest are drawn from the `garden` pool. A flood
//! barrier is raised across open ground cell by cell; each flood surge wears every barrier down,
//! and a barrier that gives way opens its cells again. A survey asks whether a walker can still
//! get from the west cloister to the east one.

use lockstep_core::{
    record_session, ClockConfiguration, Context, Handle, Message, Recording, Runner, Session,
    Simulation, StableVector, StepConfiguration, Streams,
};
use lockstep_procgen::{Layout, Reservation, Templates};
use lockstep_spatial::{Cell, GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Square4};
use lockstep_structures::{Refusal, StructureEvent, Structures};
use serde::{Deserialize, Serialize};

pub const TEMPLATES_JSON: &str = include_str!("../data/templates.json");
/// Templates with edge sockets: paths and fences that meet across chunk borders.
pub const JOINED_JSON: &str = include_str!("../data/joined.json");

/// The estate, in chunks of 8 by 8 cells.
pub const CHUNKS_WIDE: u32 = 6;
pub const CHUNKS_HIGH: u32 = 4;
/// The durability of a new barrier, per cell it covers.
pub const DURABILITY_PER_CELL: u32 = 4;
/// The chunks on the middle row fixed to the cloister template: both ends and the two in the middle.
pub const CLOISTERS: [(u32, u32); 4] = [(0, 1), (3, 1), (4, 1), (5, 1)];

pub fn templates() -> Templates {
    Templates::from_json(TEMPLATES_JSON).expect("the committed templates are valid")
}

pub fn joined_templates() -> Templates {
    Templates::from_json(JOINED_JSON).expect("the committed templates are valid")
}

pub fn reservations(templates: &Templates) -> Vec<Reservation> {
    let template = templates.template_id("cloister").expect("defined");
    CLOISTERS
        .iter()
        .map(|(x, y)| Reservation {
            x: *x,
            y: *y,
            template,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// What the estate is called, for hosts.
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// Raise a barrier along a row of cells, from a column and row, to the east.
    Raise { x: u32, y: u32, length: u32 },
    /// A flood surge of this force wears every barrier down.
    Surge { force: u32 },
    /// Mend every barrier.
    Mend { amount: u32 },
    /// Ask whether a walker can still get from the west cloister to the east one.
    Survey,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Structure(StructureEvent),
    Refused {
        reason: Refusal,
    },
    Surveyed {
        open: bool,
    },
    /// Chunks of the map whose cells changed, for a host to redraw.
    Redrawn {
        chunks: Vec<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub layout: Layout,
    pub map: GridMap<Square4>,
    pub barriers: StableVector<()>,
    pub structures: Structures,
}

pub struct Estate {
    world: World,
    templates: Templates,
}

impl Estate {
    fn with_world(world: World) -> Self {
        Estate {
            world,
            templates: templates(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn templates(&self) -> &Templates {
        &self.templates
    }

    /// The cell at a column and row.
    pub fn cell(&self, x: u32, y: u32) -> Cell {
        self.world.map.index(x, y)
    }

    /// Whether a walker can get from the west cloister to the east one.
    pub fn is_open(&self) -> bool {
        let map = &self.world.map;
        let (from, to) = (map.index(2, 12), map.index(45, 12));
        let mut path = Vec::new();
        let result = Pathfinder::new(map).find(
            map,
            &Occupancy::new(map),
            from,
            to,
            PathOptions::default(),
            &mut path,
        );
        matches!(result, PathResult::Found { .. })
    }
}

impl Simulation for Estate {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(_configuration: Configuration, randomness: &mut Streams) -> Self {
        let templates = templates();
        let seed = ((randomness.range("terrain", 0, i32::MAX) as u64) << 32)
            | (randomness.range("terrain", 0, i32::MAX) as u64);
        let pool = templates.pool_id("garden").expect("defined");
        let layout = templates
            .roll(
                seed,
                CHUNKS_WIDE,
                CHUNKS_HIGH,
                pool,
                &reservations(&templates),
            )
            .expect("the estate fits its reservations");
        let side = templates.side();
        let mut map = GridMap::new(CHUNKS_WIDE * side, CHUNKS_HIGH * side);
        layout.apply(&templates, &mut map).expect("the map fits");
        // The host draws a new world whole: only later changes are reported.
        map.take_dirty_chunks();
        Estate {
            world: World {
                layout,
                map,
                barriers: StableVector::new(),
                structures: Structures::new(),
            },
            templates,
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        let mut structure_events = Vec::new();
        for intent in intents {
            let world = &mut self.world;
            match *intent {
                Intent::Raise { x, y, length } => {
                    // A barrier longer than the estate is wide cannot fit: refuse it before any
                    // cell is listed, so an order cannot ask for an endless row.
                    if length > world.map.width() {
                        events.push(Event::Refused {
                            reason: Refusal::OffMap(Cell(u32::MAX)),
                        });
                        continue;
                    }
                    let cells: Vec<Cell> = (x..x.saturating_add(length))
                        .map(|column| {
                            if column < world.map.width() && y < world.map.height() {
                                world.map.index(column, y)
                            } else {
                                Cell(u32::MAX)
                            }
                        })
                        .collect();
                    let barrier = world.barriers.insert(());
                    let built = world.structures.build(
                        &mut world.map,
                        barrier,
                        &cells,
                        length * DURABILITY_PER_CELL,
                        &mut structure_events,
                    );
                    if let Err(reason) = built {
                        world.barriers.remove(barrier);
                        events.push(Event::Refused { reason });
                    }
                }
                Intent::Surge { force } => {
                    let standing: Vec<Handle> = world.barriers.handles();
                    for barrier in standing {
                        if world
                            .structures
                            .damage(&mut world.map, barrier, force, &mut structure_events)
                            .is_ok()
                            && world.structures.get(barrier).is_none()
                        {
                            world.barriers.remove(barrier);
                        }
                    }
                }
                Intent::Mend { amount } => {
                    for barrier in world.barriers.handles() {
                        let _ = world
                            .structures
                            .repair(barrier, amount, &mut structure_events);
                    }
                }
                Intent::Survey => {
                    events.push(Event::Surveyed {
                        open: self.is_open(),
                    });
                }
            }
        }
        let mut all: Vec<Event> = structure_events.into_iter().map(Event::Structure).collect();
        all.extend(events);
        let chunks = self.world.map.take_dirty_chunks();
        if !chunks.is_empty() {
            all.push(Event::Redrawn { chunks });
        }
        context.events.extend(all);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Estate::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_016;
pub const DEFAULT_STEPS: u64 = 3_000;

pub fn grounds() -> Configuration {
    Configuration {
        name: "estate".to_string(),
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Estate> {
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

/// The scripted session the fixture hash covers: barriers raised in random places, some on walls
/// or on each other and refused, flood surges, mending, and surveys.
pub fn script(_estate: &Estate, script: &mut Streams) -> Vec<Intent> {
    match script.range("act", 0, 100) {
        0..=7 => vec![Intent::Raise {
            x: script.range("place", 0, 48) as u32,
            y: script.range("place", 0, 32) as u32,
            length: script.range("place", 2, 6) as u32,
        }],
        8..=19 => vec![Intent::Surge {
            force: script.range("force", 1, 5) as u32,
        }],
        20..=25 => vec![Intent::Mend { amount: 2 }],
        26..=30 => vec![Intent::Survey],
        _ => Vec::new(),
    }
}

fn drive(session: &mut impl Session<Estate>, seed: u64, steps: u64) {
    let mut streams = Streams::new(seed ^ 0x0000_6573_7461_7465);
    for _ in 0..steps {
        let intents = script(session.simulation(), &mut streams);
        session.step_once(&intents);
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Estate> {
    let mut runner = runner(grounds(), seed);
    drive(&mut runner, seed, steps);
    runner
}

/// The name a recording of this simulation carries.
pub const SIMULATION_ID: &str = "estate";

/// The same session, recorded with a checkpoint every `checkpoint_every` steps, keeping the
/// snapshot at each checkpoint when `keep_snapshots` is set.
pub fn record_fixture(
    seed: u64,
    steps: u64,
    checkpoint_every: u64,
    keep_snapshots: bool,
) -> Recording<Intent> {
    let configuration = grounds();
    record_session(
        SIMULATION_ID,
        &configuration,
        seed,
        runner(configuration.clone(), seed),
        checkpoint_every,
        keep_snapshots,
        |recorder| drive(recorder, seed, steps),
    )
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
