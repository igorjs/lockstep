// SPDX-License-Identifier: Apache-2.0
//! Dispatch: a consumer scenario for `lockstep-relations` that is not a game.
//!
//! A dispatcher assigns routes to drivers on the day and night shifts. Each driver answers an
//! assignment as a delegated task: its trust in the dispatcher (from `data/relations.json`, the
//! pair standing plus the dispatcher's shift group) against the routes it has driven today. Below
//! a trust of 10 it refuses outright, naming trust; above it, it accepts, delays, or refuses
//! naming the weaker reason. Pay on time raises the driver's trust,
//! late pay lowers it more and sours its shift mates on the dispatcher's shift, and trust drifts
//! back toward its rest a little each day.

use lockstep_agents::{evaluate_task, Reason, Task, TaskResponse};
use lockstep_core::math::Fixed32;
use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_relations::{GroupId, Kinds, RelationEvent, RelationId, Relations, Target};
use serde::{Deserialize, Serialize};

pub const RELATIONS_JSON: &str = include_str!("../data/relations.json");

/// Why a driver refuses.
pub const TRUST: Reason = Reason(1);
pub const FATIGUE: Reason = Reason(2);
/// Below this trust a driver refuses outright, naming trust.
pub const TRUST_NEEDED: i32 = 10;
/// What trust below the threshold scores: low enough to refuse whatever else counts.
pub const DISTRUST: i32 = -100;
/// Each route already driven today counts this much against another.
pub const ROUTE_WEIGHT: i32 = 5;
/// Accepted at or above, delayed at or above, refused below.
pub const ACCEPT_AT: i64 = 0;
pub const DELAY_AT: i64 = -15;
pub const DELAY_MINUTES: u16 = 30;
/// Trust from pay on time, and lost to late pay.
pub const ON_TIME: i32 = 4;
pub const LATE: i32 = -12;
/// Trust each shift mate of a driver paid late loses toward the dispatcher's shift.
pub const SHIFT_MATE_LATE: i32 = -3;

pub fn kinds() -> Kinds {
    Kinds::from_json(RELATIONS_JSON).expect("the committed relations are valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// Drivers by name and shift.
    pub drivers: Vec<(String, String)>,
    pub dispatcher_shift: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Assign {
        driver: Handle,
    },
    /// The driver is paid for a route, on time or late.
    Pay {
        driver: Handle,
        on_time: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Answered {
        driver: Handle,
        response: TaskResponse,
    },
    Paid {
        driver: Handle,
        on_time: bool,
    },
    Relation(RelationEvent),
    /// Not a driver.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Person {
    Dispatcher { shift: GroupId },
    Driver { name: String, shift: GroupId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub people: StableVector<Person>,
    pub relations: Relations,
    /// Routes each driver has accepted today.
    pub routes_today: Column<u32>,
    /// The game day the counts are for.
    pub day: u32,
}

pub struct Dispatch {
    world: World,
    kinds: Kinds,
    trust: RelationId,
}

impl Dispatch {
    fn with_world(world: World) -> Self {
        let kinds = kinds();
        Dispatch {
            world,
            trust: kinds.relation_id("trust").expect("defined"),
            kinds,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn kinds(&self) -> &Kinds {
        &self.kinds
    }

    pub fn dispatcher(&self) -> Handle {
        self.world
            .people
            .iter()
            .find(|(_, person)| matches!(person, Person::Dispatcher { .. }))
            .map(|(handle, _)| handle)
            .expect("one dispatcher")
    }

    pub fn drivers(&self) -> Vec<Handle> {
        self.world
            .people
            .iter()
            .filter(|(_, person)| matches!(person, Person::Driver { .. }))
            .map(|(handle, _)| handle)
            .collect()
    }

    fn dispatcher_shift(&self) -> GroupId {
        match self.world.people.get(self.dispatcher()) {
            Some(Person::Dispatcher { shift }) => *shift,
            _ => unreachable!("the dispatcher is a dispatcher"),
        }
    }

    fn shift_of(&self, who: Handle) -> Option<GroupId> {
        match self.world.people.get(who) {
            Some(Person::Driver { shift, .. }) => Some(*shift),
            _ => None,
        }
    }

    /// How much a driver trusts the dispatcher: the pair standing plus how it stands toward the
    /// dispatcher's shift.
    pub fn trust_in_dispatcher(&self, driver: Handle) -> Fixed32 {
        let dispatcher = self.dispatcher();
        let dispatcher_shift = self.dispatcher_shift();
        self.world.relations.toward(
            &self.kinds,
            self.trust,
            driver,
            dispatcher,
            &[dispatcher_shift],
        )
    }

    fn answer(&self, driver: Handle) -> TaskResponse {
        // Whole points, rounded down.
        let trust = self.trust_in_dispatcher(driver).raw() >> 16;
        let routes = self.world.routes_today.get(driver).copied().unwrap_or(0) as i32;
        let task = Task::<(i32, i32)> {
            id: 1,
            considerations: vec![
                (
                    TRUST,
                    // Below the threshold trust is a hard no; above it, how far above counts for.
                    Box::new(|_: Handle, (trust, _): &(i32, i32)| {
                        if *trust < TRUST_NEEDED {
                            DISTRUST
                        } else {
                            trust - TRUST_NEEDED
                        }
                    }),
                ),
                (
                    FATIGUE,
                    Box::new(|_: Handle, (_, routes): &(i32, i32)| -routes * ROUTE_WEIGHT),
                ),
            ],
            accept_at: ACCEPT_AT,
            delay_at: DELAY_AT,
            delay_minutes: DELAY_MINUTES,
        };
        evaluate_task(driver, &task, &(trust, routes))
    }
}

impl Simulation for Dispatch {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut dispatch = Dispatch::with_world(World {
            people: StableVector::new(),
            relations: Relations::new(),
            routes_today: Column::new(),
            day: 0,
        });
        let dispatcher_shift = dispatch
            .kinds
            .group_id(&configuration.dispatcher_shift)
            .expect("a known shift");
        dispatch.world.people.insert(Person::Dispatcher {
            shift: dispatcher_shift,
        });
        for (name, shift) in configuration.drivers {
            let shift = dispatch.kinds.group_id(&shift).expect("a known shift");
            let driver = dispatch.world.people.insert(Person::Driver { name, shift });
            dispatch.world.routes_today.set(driver, 0);
        }
        dispatch
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        let mut relation_events = Vec::new();
        if context.clock.day() != self.world.day {
            self.world.day = context.clock.day();
            for driver in self.drivers() {
                self.world.routes_today.set(driver, 0);
            }
        }
        let dispatcher = self.dispatcher();
        for intent in intents {
            let driver = match intent {
                Intent::Assign { driver } | Intent::Pay { driver, .. } => *driver,
            };
            if self.shift_of(driver).is_none() {
                events.push(Event::Unknown);
                continue;
            }
            match *intent {
                Intent::Assign { driver } => {
                    let response = self.answer(driver);
                    if response == TaskResponse::Accept {
                        let routes = self.world.routes_today.get(driver).copied().unwrap_or(0);
                        self.world.routes_today.set(driver, routes + 1);
                    }
                    events.push(Event::Answered { driver, response });
                }
                Intent::Pay { driver, on_time } => {
                    let delta = if on_time { ON_TIME } else { LATE };
                    self.world.relations.change(
                        &self.kinds,
                        self.trust,
                        driver,
                        Target::Entity(dispatcher),
                        Fixed32::from_int(delta),
                        &mut relation_events,
                    );
                    // A late payment sours the driver's shift mates on the dispatcher's shift as
                    // a whole.
                    if !on_time {
                        let (shift, dispatcher_shift) =
                            (self.shift_of(driver), self.dispatcher_shift());
                        for mate in self.drivers() {
                            if mate != driver && self.shift_of(mate) == shift {
                                self.world.relations.change(
                                    &self.kinds,
                                    self.trust,
                                    mate,
                                    Target::Group(dispatcher_shift),
                                    Fixed32::from_int(SHIFT_MATE_LATE),
                                    &mut relation_events,
                                );
                            }
                        }
                    }
                    events.push(Event::Paid { driver, on_time });
                }
            }
        }
        self.world
            .relations
            .tick(&self.kinds, context.elapsed_minutes, &mut relation_events);
        events.extend(relation_events.into_iter().map(Event::Relation));
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Dispatch::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_012;
pub const DEFAULT_STEPS: u64 = 36_000;

pub fn depot() -> Configuration {
    let driver = |name: &str, shift: &str| (name.to_string(), shift.to_string());
    Configuration {
        drivers: vec![
            driver("ama", "day_shift"),
            driver("bix", "day_shift"),
            driver("col", "night_shift"),
            driver("dru", "night_shift"),
        ],
        dispatcher_shift: "day_shift".into(),
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Dispatch> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 360,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: now and then a route for a random driver, and,
/// a third as often, pay for a random driver, late one time in five.
pub fn script(dispatch: &Dispatch, script: &mut Streams) -> Vec<Intent> {
    let drivers = dispatch.drivers();
    let driver = drivers[script.pick("driver", drivers.len())];
    match script.range("act", 0, 180) {
        0..=2 => vec![Intent::Assign { driver }],
        3 => vec![Intent::Pay {
            driver,
            on_time: script.range("late", 0, 5) != 0,
        }],
        _ => Vec::new(),
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Dispatch> {
    let mut runner = runner(depot(), seed);
    let mut streams = Streams::new(seed ^ 0x0064_6973_7061);
    for _ in 0..steps {
        let intents = script(runner.simulation(), &mut streams);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
