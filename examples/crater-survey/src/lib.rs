// SPDX-License-Identifier: Apache-2.0
//! The crater survey: a consumer scenario for `lockstep-agents` that is not a game.
//!
//! Survey rovers work a plateau. A lander touching down is loud, and the wind carries the sound:
//! a rover that hears it grows curious and, when its utility says investigating beats holding,
//! drives toward where it heard it. A rover that sees the lander is alert, and the director lets
//! at most two be alert on one lander. Some rovers are leashed to their crater and never leave
//! it. An operator can ask a rover to survey a spot; the rover accepts, delays or refuses by its
//! battery and the distance, and says why.

use lockstep_agents::{
    choose, evaluate_task, hear, perceive, steer, think, Alertness, Choice, Cone, Director, Leash,
    Mind, MindEvent, MindRules, Noise, Reason, Senses, Stimulus, Surroundings, Task, TaskResponse,
    Weather, WeatherRules,
};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_spatial::{Cell, FlowField, GridMap, Occupancy, Square8};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Each cell is two metres across.
pub fn cell_metres() -> Fixed32 {
    Fixed32::from_int(2)
}

/// A landing carries 80 metres in still air.
pub const LANDING_LOUDNESS_METRES: i32 = 80;
/// Rovers move one cell every this many steps: two cells a second at 30 steps a second.
pub const STEPS_PER_MOVE: u64 = 15;
/// Each rover looks every this many steps, staggered.
pub const PERCEIVE_EVERY: u32 = 5;
pub const ALERT_BUDGET: u16 = 2;
/// Utility choices.
pub const HOLD: u16 = 0;
pub const INVESTIGATE: u16 = 1;
/// Why a survey request is refused.
pub const LOW_BATTERY: Reason = Reason(1);
pub const TOO_FAR: Reason = Reason(2);

fn mind_rules() -> MindRules {
    MindRules {
        forget_after_minutes: Fixed32::from_int(30),
        lose_sight_after_minutes: Fixed32::from_int(2),
        heard_confidence: Fixed32::HALF,
    }
}

fn senses() -> Senses {
    Senses {
        sight: Cone {
            half_angle: 10_923,
            range_metres: Fixed32::from_int(40),
            around_metres: Fixed32::from_int(6),
        },
        hearing_range_metres: Fixed32::from_int(200),
        eye_height: 1,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoverStart {
    pub x: u32,
    pub y: u32,
    /// Leashed to its starting cell, this many metres; `None` roams freely.
    pub leash_metres: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub width: u32,
    pub height: u32,
    /// Ridges rovers cannot cross.
    pub ridges: Vec<(u32, u32)>,
    pub rovers: Vec<RoverStart>,
    pub weather: WeatherRules,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Land {
        x: u32,
        y: u32,
    },
    Depart {
        lander: Handle,
    },
    /// The operator asks a rover to survey a spot.
    Request {
        rover: Handle,
        x: u32,
        y: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Landed {
        lander: Handle,
        at: Cell,
    },
    Departed {
        lander: Handle,
    },
    /// The landing site was blocked or off the map.
    LandingRefused,
    Heard {
        rover: Handle,
        at: Cell,
    },
    Answered {
        rover: Handle,
        response: TaskResponse,
    },
    Surveyed {
        rover: Handle,
        at: Cell,
    },
    Mind(MindEvent),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    Rover,
    Lander,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub map: GridMap<Square8>,
    pub occupancy: Occupancy,
    pub entities: StableVector<Entity>,
    pub senses: Column<Senses>,
    pub minds: Column<Mind>,
    pub leashes: Column<Leash>,
    pub facings: Column<Turn>,
    /// Percent, 0 to 100.
    pub battery: Column<u32>,
    /// Where an accepted survey request sends a rover.
    pub orders: Column<Cell>,
    pub weather: Weather,
    pub weather_rules: WeatherRules,
}

pub struct Survey {
    world: World,
    /// Flow fields by goal cell, rebuilt as needed; not state.
    fields: BTreeMap<Cell, FlowField>,
}

impl Survey {
    fn with_world(world: World) -> Self {
        Survey {
            world,
            fields: BTreeMap::new(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn rovers(&self) -> Vec<Handle> {
        self.of(Entity::Rover)
    }

    pub fn landers(&self) -> Vec<Handle> {
        self.of(Entity::Lander)
    }

    fn of(&self, kind: Entity) -> Vec<Handle> {
        self.world
            .entities
            .iter()
            .filter(|(_, entity)| **entity == kind)
            .map(|(handle, _)| handle)
            .collect()
    }

    pub fn cell_of(&self, who: Handle) -> Option<Cell> {
        self.world.occupancy.cell_of(who)
    }

    fn field(&mut self, goal: Cell) -> &FlowField {
        let map = &self.world.map;
        self.fields
            .entry(goal)
            .or_insert_with(|| FlowField::build(map, &[goal], u32::MAX))
    }

    fn land(
        &mut self,
        x: u32,
        y: u32,
        stimuli: &mut Vec<(Handle, Stimulus)>,
        events: &mut Vec<Event>,
    ) {
        let world = &mut self.world;
        if !world.map.contains(x, y) {
            events.push(Event::LandingRefused);
            return;
        }
        let at = world.map.index(x, y);
        if !world.map.is_passable(at) || world.occupancy.at(at).is_some() {
            events.push(Event::LandingRefused);
            return;
        }
        let lander = world.entities.insert(Entity::Lander);
        world.occupancy.place(at, lander).expect("checked free");
        events.push(Event::Landed { lander, at });
        let noise = Noise {
            at,
            loudness_metres: Fixed32::from_int(LANDING_LOUDNESS_METRES),
            source: Some(lander),
            psychic: false,
        };
        let mut heard = Vec::new();
        hear(
            &world.map,
            &world.occupancy,
            &world.senses,
            &noise,
            world.weather.wind(),
            cell_metres(),
            &mut heard,
        );
        for heard in heard {
            stimuli.push((
                heard.listener,
                Stimulus::Heard {
                    at,
                    source: Some(lander),
                },
            ));
            events.push(Event::Heard {
                rover: heard.listener,
                at,
            });
        }
    }

    fn request(&mut self, rover: Handle, x: u32, y: u32, events: &mut Vec<Event>) {
        let world = &self.world;
        let (Some(from), true) = (world.occupancy.cell_of(rover), world.map.contains(x, y)) else {
            return;
        };
        let to = world.map.index(x, y);
        let task = Task::<(u32, Fixed32)> {
            id: 1,
            considerations: vec![
                // Battery above half counts for, below it against.
                (
                    LOW_BATTERY,
                    Box::new(|_: Handle, (battery, _): &(u32, Fixed32)| *battery as i32 - 50),
                ),
                // A metre of distance costs a fifth of a point.
                (
                    TOO_FAR,
                    Box::new(|_: Handle, (_, distance): &(u32, Fixed32)| {
                        -(distance.raw() / 65_536 / 5)
                    }),
                ),
            ],
            accept_at: 0,
            delay_at: -20,
            delay_minutes: 10,
        };
        let battery = world.battery.get(rover).copied().unwrap_or(0);
        let distance = lockstep_agents::distance_metres(&world.map, from, to, cell_metres());
        let response = evaluate_task(rover, &task, &(battery, distance));
        if response == TaskResponse::Accept {
            self.world.orders.set(rover, to);
        }
        events.push(Event::Answered { rover, response });
    }

    /// Where a rover heads this move, if anywhere: an accepted order first; then what it remembers,
    /// when investigating scores above holding; then home, when it has strayed.
    fn goal(&self, rover: Handle) -> Option<Cell> {
        let world = &self.world;
        if let Some(order) = world.orders.get(rover) {
            return Some(*order);
        }
        let mind = world.minds.get(rover)?;
        if let Some(memory) = mind.memory {
            let battery = world.battery.get(rover).copied().unwrap_or(0);
            let investigate = Choice::<(Fixed32, u32)> {
                id: INVESTIGATE,
                considerations: vec![
                    Box::new(|_: Handle, (confidence, _): &(Fixed32, u32)| {
                        (confidence.raw() as i64 * 100 / 65_536) as i32
                    }),
                    Box::new(|_: Handle, (_, battery): &(Fixed32, u32)| *battery as i32 / 2),
                ],
            };
            let hold = Choice::<(Fixed32, u32)> {
                id: HOLD,
                considerations: vec![Box::new(|_: Handle, _: &(Fixed32, u32)| 40)],
            };
            if mind.alertness != Alertness::Idle
                && choose(rover, &[hold, investigate], &(memory.confidence, battery))
                    == Some(INVESTIGATE)
            {
                return Some(memory.position);
            }
        }
        let leash = world.leashes.get(rover)?;
        (world.occupancy.cell_of(rover) != Some(leash.home)).then_some(leash.home)
    }

    fn drive(&mut self, events: &mut Vec<Event>) {
        let mut by_goal: BTreeMap<Cell, Vec<Handle>> = BTreeMap::new();
        for rover in self.rovers() {
            let battery = self.world.battery.get(rover).copied().unwrap_or(0);
            match self.goal(rover) {
                Some(goal) if battery > 0 => by_goal.entry(goal).or_default().push(rover),
                _ => {
                    self.world.battery.set(rover, (battery + 1).min(100));
                }
            }
        }
        for (goal, rovers) in by_goal {
            self.field(goal);
            let field = &self.fields[&goal];
            let world = &mut self.world;
            let mut steps = Vec::new();
            steer(
                &world.map,
                &mut world.occupancy,
                field,
                &rovers,
                &world.leashes,
                cell_metres(),
                &mut steps,
            );
            for step in steps {
                if let lockstep_agents::SteerEvent::Stepped { who, from, to } = step {
                    world.facings.set(who, heading(&world.map, from, to));
                    let battery = world.battery.get(who).copied().unwrap_or(0);
                    world.battery.set(who, battery.saturating_sub(1));
                }
            }
            for rover in rovers {
                if world.orders.get(rover) == Some(&goal)
                    && world.occupancy.cell_of(rover) == Some(goal)
                {
                    world.orders.unset(rover);
                    events.push(Event::Surveyed { rover, at: goal });
                }
            }
        }
    }
}

/// The angle from one cell's centre to another's, counter-clockwise from east.
fn heading(map: &GridMap<Square8>, from: Cell, to: Cell) -> Turn {
    let (a, b) = (map.centre(from), map.centre(to));
    lockstep_core::math::atan2(a.y - b.y, b.x - a.x)
}

impl Simulation for Survey {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut map: GridMap<Square8> = GridMap::new(configuration.width, configuration.height);
        for (x, y) in &configuration.ridges {
            map.set_passable(map.index(*x, *y), false);
        }
        let mut world = World {
            occupancy: Occupancy::new(&map),
            map,
            entities: StableVector::new(),
            senses: Column::new(),
            minds: Column::new(),
            leashes: Column::new(),
            facings: Column::new(),
            battery: Column::new(),
            orders: Column::new(),
            weather: Weather::new(0, Fixed32::from_int(4)),
            weather_rules: configuration.weather,
        };
        for start in &configuration.rovers {
            let rover = world.entities.insert(Entity::Rover);
            let at = world.map.index(start.x, start.y);
            world
                .occupancy
                .place(at, rover)
                .expect("rovers start apart");
            world.senses.set(rover, senses());
            world.minds.set(rover, Mind::default());
            world.facings.set(rover, 0);
            world.battery.set(rover, 100);
            if let Some(metres) = start.leash_metres {
                world.leashes.set(
                    rover,
                    Leash {
                        home: at,
                        radius_metres: Fixed32::from_int(metres),
                    },
                );
            }
        }
        Survey::with_world(world)
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        let mut stimuli = Vec::new();
        for intent in intents {
            match intent {
                Intent::Land { x, y } => self.land(*x, *y, &mut stimuli, &mut events),
                Intent::Depart { lander } => {
                    if self.world.entities.get(*lander) == Some(&Entity::Lander) {
                        self.world.occupancy.vacate(*lander);
                        self.world.entities.remove(*lander);
                        events.push(Event::Departed { lander: *lander });
                    }
                }
                Intent::Request { rover, x, y } => self.request(*rover, *x, *y, &mut events),
            }
        }
        let rules = self.world.weather_rules;
        self.world.weather.advance(
            context.elapsed_minutes,
            &rules,
            context.randomness,
            &mut Vec::new(),
        );

        let landers = self.landers();
        let world = &self.world;
        let facing_of = |who| world.facings.get(who).copied().unwrap_or(0);
        let mut seen = Vec::new();
        perceive(
            &world.map,
            &world.occupancy,
            &world.senses,
            &facing_of,
            &landers,
            context.step_number,
            PERCEIVE_EVERY,
            cell_metres(),
            &mut seen,
        );
        stimuli.extend(seen.into_iter().map(|seen| {
            (
                seen.agent,
                Stimulus::Saw {
                    target: seen.target,
                    at: seen.at,
                },
            )
        }));
        let mut mind_events = Vec::new();
        let world = &mut self.world;
        think(
            &mut world.minds,
            &world.leashes,
            &stimuli,
            context.elapsed_minutes,
            &mind_rules(),
            &Director {
                alert_budget: ALERT_BUDGET,
            },
            &Surroundings {
                map: &world.map,
                occupancy: &world.occupancy,
                cell_metres: cell_metres(),
            },
            &mut mind_events,
        );
        events.extend(mind_events.into_iter().map(Event::Mind));
        if context.step_number.is_multiple_of(STEPS_PER_MOVE) {
            self.drive(&mut events);
        }
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Survey::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_009;
pub const DEFAULT_STEPS: u64 = 18_000;

/// A 64 by 48 plateau with a ridge, three rovers leashed to craters and three free.
pub fn plateau() -> Configuration {
    let mut ridges = Vec::new();
    for y in 10..38 {
        ridges.push((32, y));
    }
    let leashed = |x, y| RoverStart {
        x,
        y,
        leash_metres: Some(20),
    };
    let free = |x, y| RoverStart {
        x,
        y,
        leash_metres: None,
    };
    Configuration {
        width: 64,
        height: 48,
        ridges,
        rovers: vec![
            leashed(10, 10),
            leashed(50, 12),
            leashed(14, 38),
            free(28, 24),
            free(40, 30),
            free(54, 40),
        ],
        weather: WeatherRules {
            mean_strength: Fixed32::from_int(8),
            fronts_per_day: 3,
        },
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Survey> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 480,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: a lander every 30 seconds at a random spot,
/// leaving 40 seconds later, and now and then a survey request to a random rover.
pub fn script(survey: &Survey, script: &mut Streams, step: u64) -> Vec<Intent> {
    let world = survey.world();
    let mut intents = Vec::new();
    if step % 900 == 450 {
        intents.push(Intent::Land {
            x: script.range("x", 0, world.map.width() as i32) as u32,
            y: script.range("y", 0, world.map.height() as i32) as u32,
        });
    }
    if step.is_multiple_of(900) {
        // The oldest lander leaves, so landers come and go.
        if let Some(lander) = survey.landers().first().copied() {
            if survey.landers().len() > 1 {
                intents.push(Intent::Depart { lander });
            }
        }
    }
    if step % 1_200 == 600 {
        let rovers = survey.rovers();
        intents.push(Intent::Request {
            rover: rovers[script.pick("rover", rovers.len())],
            x: script.range("x", 0, world.map.width() as i32) as u32,
            y: script.range("y", 0, world.map.height() as i32) as u32,
        });
    }
    intents
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Survey> {
    let mut runner = runner(plateau(), seed);
    let mut streams = Streams::new(seed ^ 0x0063_7261_7465);
    for step in 0..steps {
        let intents = script(runner.simulation(), &mut streams, step);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
