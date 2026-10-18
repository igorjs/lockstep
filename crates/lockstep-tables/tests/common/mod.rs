// SPDX-License-Identifier: Apache-2.0
//! A small simulation for the decision tests: a counter whose step size comes from a table.
#![allow(dead_code)]

use lockstep_core::{
    ClockConfiguration, Context, Message, Recorder, Recording, Runner, Simulation,
    StepConfiguration, Streams,
};
use lockstep_tables::{Table, TableRow, TableSwap};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub amount: i64,
}

impl TableRow for Size {
    fn check(&self) -> Result<(), String> {
        if self.amount < 0 {
            Err("a size below zero".to_string())
        } else {
            Ok(())
        }
    }
}

pub const SMALL: &str = r#"{"rows":[{"name":"step","amount":1}]}"#;
pub const LARGE: &str = r#"{"rows":[{"name":"step","amount":10}]}"#;
pub const BROKEN: &str = r#"{"rows":[{"name":"step","amount":-5}]}"#;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub sizes: Table<Size>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Add,
    Swap(TableSwap),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Event {
    pub swapped: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Counter {
    pub sizes: Table<Size>,
    pub total: i64,
}

impl Simulation for Counter {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = Counter;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        Counter {
            sizes: configuration.sizes,
            total: 0,
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            match intent {
                Intent::Add => self.total += self.sizes.get("step").unwrap().amount,
                Intent::Swap(swap) => {
                    let swapped = self.sizes.swap(&swap.json).is_ok();
                    context.events.push(Event { swapped });
                }
            }
        }
    }

    fn snapshot(&self) -> Counter {
        self.clone()
    }

    fn restore(snapshot: Counter) -> Self {
        snapshot
    }
}

fn clock() -> ClockConfiguration {
    ClockConfiguration {
        day_length_real_minutes: 1440.0,
        sunrise_minute: 0,
        sunset_minute: 1440,
        starting_minute: 0,
        starting_day: 0,
    }
}

pub fn configuration() -> Configuration {
    Configuration {
        sizes: Table::from_json(SMALL).unwrap(),
    }
}

pub fn runner() -> Runner<Counter> {
    Runner::new(configuration(), 1, StepConfiguration::default(), clock())
}

pub fn swap(json: &str) -> Intent {
    Intent::Swap(TableSwap {
        table: "sizes".into(),
        json: json.into(),
    })
}

/// Ten steps of adding, with `swap_to` sent before step 5, recorded.
pub fn recorded(swap_to: &str) -> Recording<Intent> {
    let mut recorder = Recorder::<Counter>::new(
        "counter",
        configuration(),
        1,
        StepConfiguration::default(),
        clock(),
        2,
    );
    for step in 0..10 {
        if step == 5 {
            recorder.step_once(&[swap(swap_to), Intent::Add]);
        } else {
            recorder.step_once(&[Intent::Add]);
        }
    }
    recorder.into_recording()
}
