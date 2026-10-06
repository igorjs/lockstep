//! The crowd: the consumer scenario for the path batch.
//!
//! A crowd of bodies on a walled map walks to one goal. Every step, the bodies still walking ask for a
//! path in one batch (`find_paths`), with the other bodies treated as walls, and then step one cell
//! along their answer in handle order, through occupancy. A body that reaches the goal leaves the
//! map. With the `parallel` feature the batch runs on a thread pool; the fixture hash is the same
//! either way, which is what this scenario proves.

use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_spatial::{
    find_paths, Cell, GridMap, Occupancy, PathOptions, PathRequest, PathResult, Square8,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub width: u32,
    pub height: u32,
    pub wall_percent: u32,
    pub bodies: u32,
    pub goal: (u32, u32),
}

impl Message for Configuration {
    const VERSION: u32 = 1;
}

/// The crowd takes no input after it starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Intent {}

impl Message for Intent {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Arrived {
        body: Handle,
        step: u64,
    },
    /// No path from this body to the goal, with the other bodies in the way.
    Waiting {
        body: Handle,
    },
}

impl Message for Event {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub map: GridMap<Square8>,
    pub goal: Cell,
    pub bodies: StableVector<()>,
    pub positions: Column<Cell>,
}

impl Message for World {
    const VERSION: u32 = 1;
}

pub struct Crowd {
    world: World,
    occupancy: Occupancy,
}

impl Crowd {
    fn with_world(world: World) -> Self {
        let mut occupancy = Occupancy::new(&world.map);
        for (body, cell) in world.positions.iter() {
            occupancy
                .place(*cell, body)
                .expect("a saved world has distinct positions");
        }
        Crowd { world, occupancy }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// Bodies still on the map.
    pub fn walking(&self) -> usize {
        self.world.bodies.len()
    }
}

impl Simulation for Crowd {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, randomness: &mut Streams) -> Self {
        let mut map: GridMap<Square8> = GridMap::new(configuration.width, configuration.height);
        let goal = map.index(configuration.goal.0, configuration.goal.1);
        for index in 0..map.cell_count() as u32 {
            if Cell(index) != goal
                && randomness.range("walls", 0, 100) < configuration.wall_percent as i32
            {
                map.set_passable(Cell(index), false);
            }
        }
        let mut world = World {
            map,
            goal,
            bodies: StableVector::new(),
            positions: Column::new(),
        };
        let mut occupancy = Occupancy::new(&world.map);
        let cells = world.map.cell_count() as i32;
        for _ in 0..configuration.bodies {
            // Draw until the cell is open ground, not the goal, and free.
            for _ in 0..1_000 {
                let cell = Cell(randomness.range("spawn", 0, cells) as u32);
                if world.map.is_passable(cell) && cell != goal && occupancy.at(cell).is_none() {
                    let body = world.bodies.insert(());
                    occupancy.place(cell, body).expect("the cell was free");
                    world.positions.set(body, cell);
                    break;
                }
            }
        }
        Crowd { world, occupancy }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, _intents: &[Intent]) {
        // One batch for every body still walking, in handle order.
        let walkers = self.world.bodies.handles();
        let requests: Vec<PathRequest> = walkers
            .iter()
            .map(|body| PathRequest {
                from: *self.world.positions.get(*body).expect("placed"),
                to: self.world.goal,
            })
            .collect();
        let options = PathOptions {
            treat_occupants_as_walls: true,
            maximum_expansions: 4_000,
        };
        let answers = find_paths(&self.world.map, &self.occupancy, &requests, options);

        // Apply the answers in request order, which is handle order: a body sees the moves of lower
        // handles in this step.
        for (body, (result, path)) in walkers.iter().zip(answers) {
            let Some(next) = path.first().copied() else {
                if !matches!(result, PathResult::Found { .. }) {
                    context.events.push(Event::Waiting { body: *body });
                }
                continue;
            };
            if self.occupancy.move_to(*body, next).is_err() {
                continue; // a lower handle took the cell this step
            }
            self.world.positions.set(*body, next);
            if next == self.world.goal {
                self.occupancy.vacate(*body);
                self.world.positions.unset(*body);
                self.world.bodies.remove(*body);
                context.events.push(Event::Arrived {
                    body: *body,
                    step: context.step_number,
                });
            }
        }
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Crowd::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_260_925;
pub const DEFAULT_STEPS: u64 = 160;

pub fn configuration() -> Configuration {
    Configuration {
        width: 48,
        height: 48,
        wall_percent: 18,
        bodies: 60,
        goal: (24, 24),
    }
}

pub fn runner(seed: u64) -> Runner<Crowd> {
    Runner::new(
        configuration(),
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 1440.0,
            sunrise_minute: 0,
            sunset_minute: 1440,
            starting_minute: 0,
            starting_day: 0,
        },
    )
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Crowd> {
    let mut runner = runner(seed);
    for _ in 0..steps {
        runner.step_once(&[]);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
