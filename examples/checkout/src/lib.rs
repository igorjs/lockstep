// SPDX-License-Identifier: Apache-2.0
//! Checkout: a consumer scenario for the waves in `lockstep-agents` that is not a game.
//!
//! A store announces a surge of shoppers. The surge (`data/surge.json`) is spaced into pulses by
//! game minutes. The queue has a budget (`data/queue.json`), kept by the director: a pulse only
//! joins as far as the queue has room, and the rest wait outside. Each open lane serves one
//! shopper every service interval.

use lockstep_agents::{Director, WaveEvent, WaveRules, Waves};
use lockstep_core::{
    ClockConfiguration, Context, Message, Runner, Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

pub const SURGE_JSON: &str = include_str!("../data/surge.json");
pub const QUEUE_JSON: &str = include_str!("../data/queue.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueRules {
    pub queue_budget: u16,
    pub lanes_at_the_start: u16,
    /// Game minutes one lane takes to serve one shopper. At least one.
    pub service_minutes: u32,
}

pub fn surge_rules() -> WaveRules {
    serde_json::from_str(SURGE_JSON).expect("the committed surge rules are valid")
}

pub fn queue_rules() -> QueueRules {
    serde_json::from_str(QUEUE_JSON).expect("the committed queue rules are valid")
}

pub const MOST_LANES: u16 = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// Open lanes at the start, at most `MOST_LANES`.
    pub lanes: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// A surge of this many shoppers is coming.
    Announce {
        shoppers: u32,
    },
    OpenLane,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Surge(WaveEvent),
    /// A surge is still arriving, or no shoppers were announced.
    Ignored,
    /// Every lane is open.
    NoMoreLanes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub surge: Option<Waves>,
    pub queue: u32,
    pub lanes: u16,
    pub served: u32,
    /// The longest the queue has been.
    pub peak_queue: u32,
    /// Raw 16.16 game minutes not yet a whole service interval.
    service_carry: i64,
}

pub struct Checkout {
    world: World,
    surge_rules: WaveRules,
    queue_rules: QueueRules,
}

impl Checkout {
    fn with_world(world: World) -> Self {
        Checkout {
            world,
            surge_rules: surge_rules(),
            queue_rules: queue_rules(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn budget(&self) -> u32 {
        self.queue_rules.queue_budget as u32
    }

    /// Shoppers whose pulse has come and who are still outside.
    pub fn waiting_outside(&self) -> u32 {
        self.world
            .surge
            .as_ref()
            .map_or(0, |surge| surge.waiting(&self.surge_rules))
    }

    fn director(&self) -> Director {
        Director {
            alert_budget: self.queue_rules.queue_budget,
        }
    }
}

impl Simulation for Checkout {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        Checkout::with_world(World {
            surge: None,
            queue: 0,
            lanes: configuration.lanes.min(MOST_LANES),
            served: 0,
            peak_queue: 0,
            service_carry: 0,
        })
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            match *intent {
                Intent::Announce { shoppers } => {
                    let busy = self
                        .world
                        .surge
                        .as_ref()
                        .is_some_and(|surge| !surge.is_finished());
                    if busy || shoppers == 0 {
                        context.events.push(Event::Ignored);
                    } else {
                        self.world.surge = Some(Waves::new(shoppers));
                    }
                }
                Intent::OpenLane if self.world.lanes < MOST_LANES => self.world.lanes += 1,
                Intent::OpenLane => context.events.push(Event::NoMoreLanes),
            }
        }
        let interval = self.queue_rules.service_minutes.max(1) as i64 * 65_536;
        self.world.service_carry += context.elapsed_minutes.raw().max(0) as i64;
        while self.world.service_carry >= interval {
            self.world.service_carry -= interval;
            let leaving = (self.world.lanes as u32).min(self.world.queue);
            self.world.queue -= leaving;
            self.world.served += leaving;
        }
        let headroom = self.director().headroom(self.world.queue);
        if let Some(surge) = self.world.surge.as_mut() {
            let mut events = Vec::new();
            self.world.queue += surge.advance(
                &self.surge_rules,
                context.elapsed_minutes,
                headroom,
                &mut events,
            );
            context.events.extend(events.into_iter().map(Event::Surge));
        }
        self.world.peak_queue = self.world.peak_queue.max(self.world.queue);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Checkout::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_019;
/// One game day of 3,600 steps.
pub const DEFAULT_STEPS: u64 = 3_600;

pub fn store() -> Configuration {
    Configuration {
        lanes: queue_rules().lanes_at_the_start,
    }
}

/// A runner starting at 07:00. A day is 2 real minutes, so a step is 0.4 game minutes.
pub fn runner(configuration: Configuration, seed: u64) -> Runner<Checkout> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 2.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 420,
            starting_day: 0,
        },
    )
}

pub fn step(runner: &mut Runner<Checkout>, intents: &[Intent]) -> Vec<Event> {
    runner.step_once(intents).events
}

/// The scripted session the fixture hash covers: a surge on the first step, one announced too
/// soon, a lane opened a third of the way through, and a second surge after the first has gone.
pub fn script(step_number: u64, script: &mut Streams) -> Vec<Intent> {
    match step_number {
        0 => vec![Intent::Announce {
            shoppers: script.range("surge", 300, 500) as u32,
        }],
        10 => vec![Intent::Announce { shoppers: 50 }],
        1_200 => vec![Intent::OpenLane],
        1_800 => vec![Intent::Announce {
            shoppers: script.range("surge", 300, 500) as u32,
        }],
        _ => Vec::new(),
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Checkout> {
    let mut runner = runner(store(), seed);
    let mut streams = Streams::new(seed ^ 0x0063_6865_636b);
    for step_number in 0..steps {
        let intents = script(step_number, &mut streams);
        step(&mut runner, &intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
