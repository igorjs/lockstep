// SPDX-License-Identifier: Apache-2.0
//! A small simulation used by every core test: it sums its intents and draws from one stream.

#![allow(dead_code)]

use lockstep_core::{
    ClockConfiguration, Context, Message, Runner, Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Add(pub i32);

impl Message for Add {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Applied(pub i32);

impl Message for Applied {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeConfiguration {
    pub start: i64,
}

impl Message for ProbeConfiguration {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeSnapshot {
    pub steps: u64,
    pub total: i64,
    pub drawn: u64,
}

impl Message for ProbeSnapshot {
    const VERSION: u32 = 1;
}

pub struct Probe {
    state: ProbeSnapshot,
}

impl Simulation for Probe {
    type Intent = Add;
    type Event = Applied;
    type Snapshot = ProbeSnapshot;
    type Configuration = ProbeConfiguration;

    fn create(configuration: ProbeConfiguration, _randomness: &mut Streams) -> Self {
        Probe {
            state: ProbeSnapshot {
                steps: 0,
                total: configuration.start,
                drawn: 0,
            },
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Add]) {
        self.state.steps += 1;
        for intent in intents {
            self.state.total += intent.0 as i64;
            context.events.push(Applied(intent.0));
        }
        self.state.drawn += context.randomness.range("probe", 0, 1000) as u64;
    }

    fn snapshot(&self) -> ProbeSnapshot {
        self.state.clone()
    }

    fn restore(snapshot: ProbeSnapshot) -> Self {
        Probe { state: snapshot }
    }
}

pub const STEP_SECONDS: f32 = 1.0 / 30.0;

pub fn day_clock() -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes: 120.0,
        sunrise_minute: 360,
        sunset_minute: 1080,
        starting_minute: 0,
        starting_day: 0,
    }
}

pub fn probe_runner(seed: u64) -> Runner<Probe> {
    Runner::new(
        ProbeConfiguration { start: 0 },
        seed,
        StepConfiguration::default(),
        day_clock(),
    )
}
