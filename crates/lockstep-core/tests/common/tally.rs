// SPDX-License-Identifier: Apache-2.0
//! A small simulation for replay tests. `Tally<PLANT>` adds what it is told, a rare bonus from a
//! named stream, and its game minutes; from step `PLANT` on it also adds one more each step, a
//! planted rule change. `Honest` never plants.

use lockstep_core::math::Fixed32;
use lockstep_core::{
    Chance, ClockConfiguration, Context, Message, Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Start {
    pub total: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Add(pub i64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Noted {
    Bonus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct State {
    pub total: i64,
    pub minutes: Fixed32,
}

pub struct Tally<const PLANT: u64> {
    state: State,
}

pub type Honest = Tally<{ u64::MAX }>;

impl<const PLANT: u64> Simulation for Tally<PLANT> {
    type Intent = Add;
    type Event = Noted;
    type Snapshot = State;
    type Configuration = Start;

    fn create(configuration: Start, _randomness: &mut Streams) -> Self {
        Tally {
            state: State {
                total: configuration.total,
                minutes: Fixed32::ZERO,
            },
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Add]) {
        for Add(amount) in intents {
            self.state.total += amount;
        }
        if context.randomness.roll("bonus", Chance::percent(3)) {
            self.state.total += 100;
            context.events.push(Noted::Bonus);
        }
        self.state.minutes += context.elapsed_minutes;
        if context.step_number >= PLANT {
            self.state.total += 1;
        }
    }

    fn snapshot(&self) -> State {
        self.state.clone()
    }

    fn restore(snapshot: State) -> Self {
        Tally { state: snapshot }
    }
}

pub fn step_configuration() -> StepConfiguration {
    StepConfiguration::default()
}

pub fn clock_configuration() -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes: 24.0,
        sunrise_minute: 360,
        sunset_minute: 1_080,
        starting_minute: 0,
        starting_day: 0,
    }
}
