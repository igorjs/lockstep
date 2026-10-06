//! Mars Rovers: the grid-shaped consumer scenario.
//!
//! Rovers land on a plateau, turn left or right, and move forward one cell at a time. The classic
//! kata answers hold unchanged, and the same rules run on any topology: a heading is an index into the
//! topology's neighbour list, so a rover moves toward `neighbours(cell)[heading]`.
//!
//! The plateau is stored in a map with a ring of walls around it, so the edge of the plateau is just
//! a wall and every cell a rover can stand on has a full neighbour list. The kata numbers the plateau
//! with y growing northward, while the grid's north is a smaller y, so positions are converted at the
//! boundary and the classic answers hold unchanged.
//!
//! Rules, all resolved in handle order:
//! - A move into an edge, a rock, or a cell another rover holds does nothing.
//! - Two rovers that want the same cell: the lower handle gets it.
//! - A rover may move into a cell a lower handle vacated earlier in the same step, but not into one
//!   a higher handle will vacate later in the step.
//! - A rover that lands on an edge, a rock, or an occupied cell is wrecked: it never moves again and
//!   does not hold a cell.

use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_spatial::{Cell, GridMap, Occupancy, Square4, Square8, Topology};
use serde::{Deserialize, Serialize};

pub use lockstep_spatial::Hex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    Left,
    Right,
    Move,
}

/// Parses the kata's instruction letters: `L`, `R` and `M`.
pub fn instructions(letters: &str) -> Vec<Instruction> {
    letters
        .chars()
        .map(|letter| match letter {
            'L' => Instruction::Left,
            'R' => Instruction::Right,
            'M' => Instruction::Move,
            other => panic!("unknown instruction {other}"),
        })
        .collect()
}

/// The plateau is `width` by `height` cells, numbered from zero, with rocks on some cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub width: u32,
    pub height: u32,
    pub rocks: Vec<(u32, u32)>,
}

impl Message for Configuration {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    /// Lands the next rover at a plateau position facing a heading (an index into the neighbour list).
    Land { x: u32, y: u32, heading: u8 },
    /// Adds instructions to the end of a rover's program. A rover runs one instruction per step,
    /// starting with the step after this one.
    Program {
        rover: Handle,
        instructions: Vec<Instruction>,
    },
}

impl Message for Intent {
    const VERSION: u32 = 1;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Obstacle {
    Edge,
    Rock,
    Rover,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Landed { rover: Handle, x: u32, y: u32 },
    Wrecked { rover: Handle, obstacle: Obstacle },
    Moved { rover: Handle, x: u32, y: u32 },
    Blocked { rover: Handle, obstacle: Obstacle },
}

impl Message for Event {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct World<T: RoverTopology> {
    map: GridMap<T>,
    /// The plateau size, without the ring of wall cells around it.
    width: u32,
    height: u32,
    rovers: StableVector<()>,
    positions: Column<Cell>,
    headings: Column<u8>,
    programs: Column<Vec<Instruction>>,
    wrecked: Column<()>,
}

impl<T: RoverTopology> Message for World<T> {
    const VERSION: u32 = 1;
}

/// The Mars Rovers simulation on one topology.
pub struct MarsRovers<T: RoverTopology> {
    world: World<T>,
    occupancy: Occupancy,
}

/// A topology the rovers can run on: it can be compared and printed, as every square and hex marker is.
pub trait RoverTopology: Topology + PartialEq + std::fmt::Debug + 'static {}
impl<T> RoverTopology for T where T: Topology + PartialEq + std::fmt::Debug + 'static {}

impl<T: RoverTopology> MarsRovers<T> {
    fn from_world(world: World<T>) -> Self {
        let mut occupancy = Occupancy::new(&world.map);
        for (rover, cell) in world.positions.iter() {
            occupancy
                .place(*cell, rover)
                .expect("a saved world has distinct positions");
        }
        Self { world, occupancy }
    }

    pub fn world(&self) -> &World<T> {
        &self.world
    }

    /// The grid cell for a plateau position (x, y) with y growing northward.
    fn grid_cell(&self, x: u32, y: u32) -> Cell {
        self.world.map.index(x + 1, self.world.height - y)
    }

    /// The plateau position (x, y), with y growing northward, of a grid cell.
    fn plateau_position(&self, cell: Cell) -> (u32, u32) {
        let (x, y) = self.world.map.coordinates(cell);
        (x - 1, self.world.height - y)
    }

    /// Where a rover stands on the plateau, as (x, y), or `None` when it is wrecked or unknown.
    pub fn position_of(&self, rover: Handle) -> Option<(u32, u32)> {
        let cell = self.world.positions.get(rover)?;
        Some(self.plateau_position(*cell))
    }

    pub fn heading_of(&self, rover: Handle) -> Option<u8> {
        self.world.headings.get(rover).copied()
    }

    pub fn is_wrecked(&self, rover: Handle) -> bool {
        self.world.wrecked.has(rover)
    }

    /// Every rover, in handle order.
    pub fn rovers(&self) -> Vec<Handle> {
        self.world.rovers.handles()
    }

    fn obstacle_at(&self, cell: Cell) -> Option<Obstacle> {
        let (x, y) = self.world.map.coordinates(cell);
        let on_plateau =
            (1..=self.world.width).contains(&x) && (1..=self.world.height).contains(&y);
        if !on_plateau {
            return Some(Obstacle::Edge);
        }
        if !self.world.map.is_passable(cell) {
            return Some(Obstacle::Rock);
        }
        if self.occupancy.at(cell).is_some() {
            return Some(Obstacle::Rover);
        }
        None
    }

    fn land(&mut self, x: u32, y: u32, heading: u8, context: &mut Context<'_, Self>) {
        let rover = self.world.rovers.insert(());
        self.world
            .headings
            .set(rover, heading % T::NEIGHBOURS as u8);
        let on_plateau = x < self.world.width && y < self.world.height;
        let cell = if on_plateau {
            self.grid_cell(x, y)
        } else {
            Cell(0)
        };
        match if on_plateau {
            self.obstacle_at(cell)
        } else {
            Some(Obstacle::Edge)
        } {
            None => {
                self.occupancy
                    .place(cell, rover)
                    .expect("the cell was free");
                self.world.positions.set(rover, cell);
                context.events.push(Event::Landed { rover, x, y });
            }
            Some(obstacle) => {
                self.world.wrecked.set(rover, ());
                context.events.push(Event::Wrecked { rover, obstacle });
            }
        }
    }

    fn execute(
        &mut self,
        rover: Handle,
        instruction: Instruction,
        context: &mut Context<'_, Self>,
    ) {
        let quarter = if T::NEIGHBOURS == 8 { 2 } else { 1 };
        let count = T::NEIGHBOURS as u8;
        let heading = self.world.headings.get(rover).copied().unwrap_or(0);
        match instruction {
            Instruction::Left => {
                self.world
                    .headings
                    .set(rover, (heading + count - quarter) % count);
            }
            Instruction::Right => {
                self.world.headings.set(rover, (heading + quarter) % count);
            }
            Instruction::Move => {
                let Some(from) = self.world.positions.get(rover).copied() else {
                    return;
                };
                let mut neighbours = Vec::new();
                self.world.map.neighbours(from, &mut neighbours);
                let target = neighbours[heading as usize];
                match self.obstacle_at(target) {
                    None => {
                        self.occupancy
                            .move_to(rover, target)
                            .expect("the cell was free");
                        self.world.positions.set(rover, target);
                        let (x, y) = self.world.map.coordinates(target);
                        context.events.push(Event::Moved {
                            rover,
                            x: x - 1,
                            y: y - 1,
                        });
                    }
                    Some(obstacle) => context.events.push(Event::Blocked { rover, obstacle }),
                }
            }
        }
    }
}

impl<T: RoverTopology> Simulation for MarsRovers<T> {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World<T>;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        // A ring of wall cells around the plateau, so the edge is a wall.
        let mut map: GridMap<T> = GridMap::new(configuration.width + 2, configuration.height + 2);
        for y in 0..configuration.height + 2 {
            for x in 0..configuration.width + 2 {
                let on_plateau = (1..=configuration.width).contains(&x)
                    && (1..=configuration.height).contains(&y);
                if !on_plateau {
                    map.set_passable(map.index(x, y), false);
                }
            }
        }
        for (x, y) in &configuration.rocks {
            map.set_passable(map.index(x + 1, configuration.height - y), false);
        }
        Self::from_world(World {
            map,
            width: configuration.width,
            height: configuration.height,
            rovers: StableVector::new(),
            positions: Column::new(),
            headings: Column::new(),
            programs: Column::new(),
            wrecked: Column::new(),
        })
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        // One instruction per rover, in handle order. A rover sees the moves of lower handles in this
        // step, and not the moves of higher handles that have not happened yet. A program added in
        // this step starts running in the next one.
        for rover in self.world.rovers.handles() {
            if self.world.wrecked.has(rover) {
                continue;
            }
            let next = match self.world.programs.get_mut(rover) {
                Some(program) if !program.is_empty() => Some(program.remove(0)),
                _ => None,
            };
            if let Some(instruction) = next {
                self.execute(rover, instruction, context);
            }
        }
        for intent in intents {
            match intent {
                Intent::Land { x, y, heading } => self.land(*x, *y, *heading, context),
                Intent::Program {
                    rover,
                    instructions,
                } => {
                    if self.world.rovers.contains(*rover) {
                        let mut program =
                            self.world.programs.get(*rover).cloned().unwrap_or_default();
                        program.extend(instructions.iter().copied());
                        self.world.programs.set(*rover, program);
                    }
                }
            }
        }
    }

    fn snapshot(&self) -> World<T> {
        self.world.clone()
    }

    fn restore(snapshot: World<T>) -> Self {
        Self::from_world(snapshot)
    }
}

/// Runs a rover simulation on a topology.
pub fn runner<T: RoverTopology>(configuration: Configuration, seed: u64) -> Runner<MarsRovers<T>> {
    Runner::new(
        configuration,
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

pub const DEFAULT_SEED: u64 = 20_260_925;
pub const DEFAULT_STEPS: u64 = 120;

/// The compass names for a square topology's headings, for readable scenarios and tests.
pub mod compass {
    pub const NORTH: u8 = 0;
    pub fn east<T: lockstep_spatial::Topology>() -> u8 {
        if T::NEIGHBOURS == 8 {
            2
        } else {
            1
        }
    }
    pub fn south<T: lockstep_spatial::Topology>() -> u8 {
        if T::NEIGHBOURS == 8 {
            4
        } else {
            2
        }
    }
    pub fn west<T: lockstep_spatial::Topology>() -> u8 {
        if T::NEIGHBOURS == 8 {
            6
        } else {
            3
        }
    }
}

/// The opposite heading: half way around the neighbour list.
pub fn opposite<T: Topology>(heading: u8) -> u8 {
    (heading + T::NEIGHBOURS as u8 / 2) % T::NEIGHBOURS as u8
}

/// Lands rovers and runs a script of (rover index, instructions) programs to completion, then returns
/// the runner for inspection. Rover indexes count landings in order.
pub fn play<T: RoverTopology>(
    configuration: Configuration,
    landings: &[(u32, u32, u8)],
    programs: &[(usize, &str)],
    steps: u64,
) -> Runner<MarsRovers<T>> {
    let mut runner = runner::<T>(configuration, DEFAULT_SEED);
    let lands: Vec<Intent> = landings
        .iter()
        .map(|(x, y, heading)| Intent::Land {
            x: *x,
            y: *y,
            heading: *heading,
        })
        .collect();
    runner.step_once(&lands);
    let rovers = runner.simulation().rovers();
    let program_intents: Vec<Intent> = programs
        .iter()
        .map(|(index, letters)| Intent::Program {
            rover: rovers[*index],
            instructions: instructions(letters),
        })
        .collect();
    runner.step_once(&program_intents);
    for _ in 0..steps {
        runner.step_once(&[]);
    }
    runner
}

/// One scripted run of every scenario on every topology; the hash covers all of them.
pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    fn scenario_set<T: RoverTopology>(seed: u64, steps: u64) -> Vec<u64> {
        let plateau = Configuration {
            width: 6,
            height: 6,
            rocks: vec![(3, 4)],
        };
        let east = compass::east::<T>();
        let west = compass::west::<T>();
        let mut hashes = Vec::new();
        let mut run = |landings: &[(u32, u32, u8)], programs: &[(usize, &str)]| {
            let mut runner = runner::<T>(plateau.clone(), seed);
            let lands: Vec<Intent> = landings
                .iter()
                .map(|(x, y, heading)| Intent::Land {
                    x: *x,
                    y: *y,
                    heading: *heading,
                })
                .collect();
            runner.step_once(&lands);
            let rovers = runner.simulation().rovers();
            let program_intents: Vec<Intent> = programs
                .iter()
                .map(|(index, letters)| Intent::Program {
                    rover: rovers[*index],
                    instructions: instructions(letters),
                })
                .collect();
            runner.step_once(&program_intents);
            for _ in 0..steps {
                runner.step_once(&[]);
            }
            hashes.push(runner.hash());
        };
        // The classic kata, then edge, rock, head-on, contested cell, following, and a wrecked landing.
        run(
            &[(1, 2, compass::NORTH), (3, 3, east)],
            &[(0, "LMLMLMLMM"), (1, "MMRMMRMRRM")],
        );
        run(&[(5, 2, east)], &[(0, "MMMM")]);
        run(&[(1, 4, east)], &[(0, "MMMMM")]);
        run(&[(1, 1, east), (2, 1, west)], &[(0, "MMM"), (1, "MMM")]);
        run(&[(1, 1, east), (3, 1, west)], &[(0, "MM"), (1, "MM")]);
        run(&[(1, 1, east), (2, 1, east)], &[(0, "MMM"), (1, "MMM")]);
        run(
            &[(3, 4, east), (1, 1, east), (1, 1, west)],
            &[(1, "M"), (2, "M")],
        );
        hashes
    }
    let mut hashes = scenario_set::<Square4>(seed, steps);
    hashes.extend(scenario_set::<Square8>(seed, steps));
    hashes.extend(scenario_set::<Hex>(seed, steps));
    lockstep_core::hash_of(&hashes)
}
