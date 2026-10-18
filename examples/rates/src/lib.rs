// SPDX-License-Identifier: Apache-2.0
//! Rates: a consumer scenario that is not a game.
//!
//! A parcel desk quotes prices from a rate table: a base price and a price per started kilogram for
//! each service. The table is data, and the scenario swaps it while the desk is open, through the
//! `SwapRates` intent. A swap that fails a check is refused and the old rates stay; a swap with the
//! same content changes nothing; a recording replays every swap at the step it happened.

use lockstep_core::{
    ClockConfiguration, Context, Message, Recorder, Recording, Runner, Simulation,
    StepConfiguration, Streams,
};
use lockstep_tables::{Swapped, Table, TableRow, TableSwap};
use serde::{Deserialize, Serialize};

pub const SIMULATION_ID: &str = "rates";
pub const DEFAULT_SEED: u64 = 20_261_020;
pub const DEFAULT_STEPS: u64 = 4_000;
/// The name of the one table of this simulation, as a swap names it.
pub const RATES_TABLE: &str = "rates";

/// The rates a desk opens with.
pub const OPENING_RATES: &str = r#"{"rows":[
  {"name":"economy","base_minor":500,"per_kilogram_minor":120},
  {"name":"express","base_minor":900,"per_kilogram_minor":260},
  {"name":"freight","base_minor":2000,"per_kilogram_minor":80}
]}"#;

/// Rates after a rise, listed in another order: the content, not the order, is what counts.
pub const RISEN_RATES: &str = r#"{"rows":[
  {"name":"freight","base_minor":2200,"per_kilogram_minor":90},
  {"name":"express","base_minor":1000,"per_kilogram_minor":300},
  {"name":"economy","base_minor":550,"per_kilogram_minor":130}
]}"#;

/// A table that fails a check: a negative price.
pub const BROKEN_RATES: &str = r#"{"rows":[
  {"name":"economy","base_minor":-1,"per_kilogram_minor":130}
]}"#;

/// A table that drops a service.
pub const WITHOUT_FREIGHT: &str = r#"{"rows":[
  {"name":"economy","base_minor":550,"per_kilogram_minor":130},
  {"name":"express","base_minor":1000,"per_kilogram_minor":300}
]}"#;

/// The step each scripted swap is sent at.
pub const RISE_STEP: u64 = 1_000;
pub const BROKEN_STEP: u64 = 2_000;
pub const SAME_STEP: u64 = 2_500;
pub const DROP_STEP: u64 = 3_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    pub base_minor: i64,
    pub per_kilogram_minor: i64,
}

impl TableRow for Rate {
    fn check(&self) -> Result<(), String> {
        if self.base_minor < 0 || self.per_kilogram_minor < 0 {
            return Err("a price below zero".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub rates: Table<Rate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    UnknownService,
    NoWeight,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Quote { service: String, grams: u32 },
    SwapRates(TableSwap),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Quoted {
        service: String,
        price_minor: i64,
    },
    Refused {
        reason: Reason,
    },
    RatesSwapped {
        from: u64,
        to: u64,
        rows: u32,
    },
    RatesUnchanged,
    /// The swap failed a check or named a table this desk does not have; the old rates stay.
    SwapRefused {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Desk {
    pub rates: Table<Rate>,
    pub quotes: u64,
    pub refusals: u64,
    pub revenue_minor: i64,
    pub swaps_applied: u32,
    pub swaps_refused: u32,
}

pub struct Rates {
    desk: Desk,
}

impl Rates {
    pub fn desk(&self) -> &Desk {
        &self.desk
    }

    fn quote(&self, service: &str, grams: u32) -> Result<i64, Reason> {
        let rate = self.desk.rates.get(service).ok_or(Reason::UnknownService)?;
        if grams == 0 {
            return Err(Reason::NoWeight);
        }
        // Every started kilogram is charged.
        let kilograms = grams.div_ceil(1000) as i64;
        rate.per_kilogram_minor
            .checked_mul(kilograms)
            .and_then(|variable| variable.checked_add(rate.base_minor))
            .ok_or(Reason::Overflow)
    }

    fn swap(&mut self, swap: &TableSwap) -> Event {
        if swap.table != RATES_TABLE {
            self.desk.swaps_refused += 1;
            return Event::SwapRefused {
                reason: format!("no table named {}", swap.table),
            };
        }
        match self.desk.rates.swap(&swap.json) {
            Ok(Swapped::Unchanged) => Event::RatesUnchanged,
            Ok(Swapped::Replaced { from, to, rows }) => {
                self.desk.swaps_applied += 1;
                Event::RatesSwapped {
                    from,
                    to,
                    rows: rows as u32,
                }
            }
            Err(error) => {
                self.desk.swaps_refused += 1;
                Event::SwapRefused {
                    reason: error.to_string(),
                }
            }
        }
    }
}

impl Simulation for Rates {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = Desk;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        Rates {
            desk: Desk {
                rates: configuration.rates,
                quotes: 0,
                refusals: 0,
                revenue_minor: 0,
                swaps_applied: 0,
                swaps_refused: 0,
            },
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            let event = match intent {
                Intent::Quote { service, grams } => match self.quote(service, *grams) {
                    Ok(price_minor) => match self.desk.revenue_minor.checked_add(price_minor) {
                        Some(revenue) => {
                            self.desk.revenue_minor = revenue;
                            self.desk.quotes += 1;
                            Event::Quoted {
                                service: service.clone(),
                                price_minor,
                            }
                        }
                        None => {
                            self.desk.refusals += 1;
                            Event::Refused {
                                reason: Reason::Overflow,
                            }
                        }
                    },
                    Err(reason) => {
                        self.desk.refusals += 1;
                        Event::Refused { reason }
                    }
                },
                Intent::SwapRates(swap) => self.swap(swap),
            };
            context.events.push(event);
        }
    }

    fn snapshot(&self) -> Desk {
        self.desk.clone()
    }

    fn restore(snapshot: Desk) -> Self {
        Rates { desk: snapshot }
    }
}

fn settings() -> (Configuration, StepConfiguration, ClockConfiguration) {
    (
        Configuration {
            rates: Table::from_json(OPENING_RATES).expect("the opening rates are valid"),
        },
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

pub fn runner(seed: u64) -> Runner<Rates> {
    let (configuration, step_configuration, clock_configuration) = settings();
    Runner::new(configuration, seed, step_configuration, clock_configuration)
}

/// A swap of the rates table to `json`.
pub fn swap_to(json: &str) -> Intent {
    Intent::SwapRates(TableSwap {
        table: RATES_TABLE.to_string(),
        json: json.to_string(),
    })
}

/// The intents of step `step` of the scripted session: a random quote, and at four steps a swap
/// first (a rise, a broken table, the same rates again, and a table without freight). The quote
/// stream is read once per step, so the script is the same whichever step it starts at.
pub fn script_intents(script: &mut Streams, step: u64) -> Vec<Intent> {
    let mut intents = Vec::new();
    match step {
        RISE_STEP => intents.push(swap_to(RISEN_RATES)),
        BROKEN_STEP => intents.push(swap_to(BROKEN_RATES)),
        SAME_STEP => intents.push(swap_to(RISEN_RATES)),
        DROP_STEP => intents.push(swap_to(WITHOUT_FREIGHT)),
        _ => {}
    }
    let services = ["economy", "express", "freight"];
    let service = services[script.pick("service", services.len())];
    let grams = script.range("grams", 0, 12_000) as u32;
    intents.push(Intent::Quote {
        service: service.to_string(),
        grams,
    });
    intents
}

fn script_streams(seed: u64) -> Streams {
    Streams::new(seed ^ 0x7261_7465_7321)
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Rates> {
    let mut runner = runner(seed);
    let mut script = script_streams(seed);
    for step in 0..steps {
        let intents = script_intents(&mut script, step);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}

pub fn record_fixture(
    seed: u64,
    steps: u64,
    checkpoint_every: u64,
    keep_snapshots: bool,
) -> Recording<Intent> {
    let (configuration, step_configuration, clock_configuration) = settings();
    let mut recorder = Recorder::<Rates>::new(
        SIMULATION_ID,
        configuration,
        seed,
        step_configuration,
        clock_configuration,
        checkpoint_every,
    );
    if keep_snapshots {
        recorder = recorder.keep_snapshots();
    }
    let mut script = script_streams(seed);
    for step in 0..steps {
        let intents = script_intents(&mut script, step);
        recorder.step_once(&intents);
    }
    recorder.into_recording()
}
