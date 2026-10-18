// SPDX-License-Identifier: Apache-2.0
//! Tariffs: a consumer scenario that is not a game.
//!
//! Accounts hold whole minor units. A transfer charges its sender a fee in basis points of the
//! amount, rounded down, kept apart from every balance. The scenario exists to show saves: the
//! first shape of the books had no fee, a save written then still loads, and the migration keeps
//! its behaviour (a fee of nothing).

use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Migrations, Recorder, Recording, Runner,
    Save, Simulation, StableVector, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

pub const SIMULATION_ID: &str = "tariffs";
pub const DEFAULT_SEED: u64 = 20_261_019;
pub const DEFAULT_STEPS: u64 = 6_000;
pub const ACCOUNT_COUNT: usize = 6;
pub const OPENING_BALANCE_MINOR: i64 = 200_000;
/// The fee a new session charges: a quarter of one percent.
pub const FEE_BASIS_POINTS: u32 = 25;
/// The step the committed old save was taken at.
pub const LEGACY_SAVE_STEP: u64 = 1_500;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub account_names: Vec<String>,
    pub opening_balance_minor: i64,
    pub fee_basis_points: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    UnknownAccount,
    NotPositive,
    SameAccount,
    InsufficientFunds,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Deposit {
        account: Handle,
        amount_minor: i64,
    },
    Transfer {
        from: Handle,
        to: Handle,
        amount_minor: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Deposited {
        account: Handle,
        amount_minor: i64,
    },
    Transferred {
        from: Handle,
        to: Handle,
        amount_minor: i64,
        fee_minor: i64,
    },
    Rejected {
        reason: Reason,
    },
}

/// The first shape of the books, before transfers charged a fee. It shipped in saves, so it is
/// frozen: never edit it, only convert it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct BooksV1 {
    pub accounts: StableVector<String>,
    pub balances: Column<i64>,
    pub transfers_applied: u64,
    pub transfers_rejected: u64,
}

/// The current shape: a fee rate and the fees collected so far.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 2)]
pub struct Books {
    pub accounts: StableVector<String>,
    pub balances: Column<i64>,
    pub transfers_applied: u64,
    pub transfers_rejected: u64,
    pub fee_basis_points: u32,
    pub fees_collected_minor: i64,
}

impl Books {
    /// Every balance plus the fees collected: money is only created by deposits.
    pub fn total_minor(&self) -> i128 {
        self.balances
            .iter()
            .map(|(_, balance)| *balance as i128)
            .sum::<i128>()
            + self.fees_collected_minor as i128
    }
}

/// Old saves charged no fee, so the converted books charge none either: loading a save never
/// changes what the next transfer costs.
pub fn migrations() -> Migrations {
    Migrations::new().step(|old: BooksV1| Books {
        accounts: old.accounts,
        balances: old.balances,
        transfers_applied: old.transfers_applied,
        transfers_rejected: old.transfers_rejected,
        fee_basis_points: 0,
        fees_collected_minor: 0,
    })
}

pub struct Tariffs {
    books: Books,
}

impl Tariffs {
    pub fn books(&self) -> &Books {
        &self.books
    }
}

/// The fee on an amount at a rate, rounded down.
pub fn fee_minor(amount_minor: i64, fee_basis_points: u32) -> i64 {
    (amount_minor as i128 * fee_basis_points as i128 / 10_000) as i64
}

impl Simulation for Tariffs {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = Books;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut books = Books {
            accounts: StableVector::new(),
            balances: Column::new(),
            transfers_applied: 0,
            transfers_rejected: 0,
            fee_basis_points: configuration.fee_basis_points,
            fees_collected_minor: 0,
        };
        for name in configuration.account_names {
            let account = books.accounts.insert(name);
            books
                .balances
                .set(account, configuration.opening_balance_minor);
        }
        Tariffs { books }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            let outcome = match intent {
                Intent::Deposit {
                    account,
                    amount_minor,
                } => self
                    .deposit(*account, *amount_minor)
                    .map(|()| Event::Deposited {
                        account: *account,
                        amount_minor: *amount_minor,
                    }),
                Intent::Transfer {
                    from,
                    to,
                    amount_minor,
                } => self
                    .transfer(*from, *to, *amount_minor)
                    .map(|fee_minor| Event::Transferred {
                        from: *from,
                        to: *to,
                        amount_minor: *amount_minor,
                        fee_minor,
                    }),
            };
            match outcome {
                Ok(event) => {
                    self.books.transfers_applied += 1;
                    context.events.push(event);
                }
                Err(reason) => {
                    self.books.transfers_rejected += 1;
                    context.events.push(Event::Rejected { reason });
                }
            }
        }
    }

    fn snapshot(&self) -> Books {
        self.books.clone()
    }

    fn restore(snapshot: Books) -> Self {
        Tariffs { books: snapshot }
    }
}

impl Tariffs {
    fn deposit(&mut self, account: Handle, amount_minor: i64) -> Result<(), Reason> {
        if amount_minor <= 0 {
            return Err(Reason::NotPositive);
        }
        let balance = self
            .books
            .balances
            .get_mut(account)
            .ok_or(Reason::UnknownAccount)?;
        *balance = balance.checked_add(amount_minor).ok_or(Reason::Overflow)?;
        Ok(())
    }

    /// Moves the amount and returns the fee the sender paid on top.
    fn transfer(&mut self, from: Handle, to: Handle, amount_minor: i64) -> Result<i64, Reason> {
        if amount_minor <= 0 {
            return Err(Reason::NotPositive);
        }
        if from == to {
            return Err(Reason::SameAccount);
        }
        let source = *self
            .books
            .balances
            .get(from)
            .ok_or(Reason::UnknownAccount)?;
        let target = *self.books.balances.get(to).ok_or(Reason::UnknownAccount)?;
        let fee = fee_minor(amount_minor, self.books.fee_basis_points);
        let charged = amount_minor.checked_add(fee).ok_or(Reason::Overflow)?;
        if source < charged {
            return Err(Reason::InsufficientFunds);
        }
        let new_target = target.checked_add(amount_minor).ok_or(Reason::Overflow)?;
        let new_fees = self
            .books
            .fees_collected_minor
            .checked_add(fee)
            .ok_or(Reason::Overflow)?;
        self.books.balances.set(from, source - charged);
        self.books.balances.set(to, new_target);
        self.books.fees_collected_minor = new_fees;
        Ok(fee)
    }
}

fn settings(fee_basis_points: u32) -> (Configuration, StepConfiguration, ClockConfiguration) {
    let names = (0..ACCOUNT_COUNT)
        .map(|index| format!("account-{index}"))
        .collect();
    (
        Configuration {
            account_names: names,
            opening_balance_minor: OPENING_BALANCE_MINOR,
            fee_basis_points,
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

/// A new session charging `fee_basis_points`.
pub fn runner_with_fee(seed: u64, fee_basis_points: u32) -> Runner<Tariffs> {
    let (configuration, step_configuration, clock_configuration) = settings(fee_basis_points);
    Runner::new(configuration, seed, step_configuration, clock_configuration)
}

pub fn runner(seed: u64) -> Runner<Tariffs> {
    runner_with_fee(seed, FEE_BASIS_POINTS)
}

fn script_streams(seed: u64) -> Streams {
    Streams::new(seed ^ 0x7461_7269_6666)
}

/// One random intent of the scripted session. Amounts run past the opening balance on purpose, so
/// some transfers are rejected.
fn script_intent(script: &mut Streams, accounts: &[Handle]) -> Intent {
    let from = accounts[script.pick("account", accounts.len())];
    let to = accounts[script.pick("account", accounts.len())];
    let amount_minor = script.range("amount", -50, 60_000) as i64;
    if script.chance("kind", 0.05) {
        Intent::Deposit {
            account: from,
            amount_minor,
        }
    } else {
        Intent::Transfer {
            from,
            to,
            amount_minor,
        }
    }
}

/// The scripted intents of steps `from_step` to `to_step`, the same whichever step the session
/// starts at, so a session continued from a save sees what a straight run would have seen.
pub fn script_between(seed: u64, accounts: &[Handle], from_step: u64, to_step: u64) -> Vec<Intent> {
    let mut script = script_streams(seed);
    (0..to_step)
        .map(|_| script_intent(&mut script, accounts))
        .skip(from_step as usize)
        .collect()
}

/// Runs the scripted session from the start.
pub fn run_fixture_with_fee(seed: u64, steps: u64, fee_basis_points: u32) -> Runner<Tariffs> {
    let mut runner = runner_with_fee(seed, fee_basis_points);
    let accounts = runner.simulation().books().accounts.handles();
    for intent in script_between(seed, &accounts, 0, steps) {
        runner.step_once(&[intent]);
    }
    runner
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Tariffs> {
    run_fixture_with_fee(seed, steps, FEE_BASIS_POINTS)
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}

/// Continues a loaded runner through the script from where its step counter stands up to `steps`.
pub fn continue_fixture(runner: &mut Runner<Tariffs>, seed: u64, steps: u64) {
    let accounts = runner.simulation().books().accounts.handles();
    for intent in script_between(seed, &accounts, runner.step_number(), steps) {
        runner.step_once(&[intent]);
    }
}

pub fn record_fixture(
    seed: u64,
    steps: u64,
    checkpoint_every: u64,
    keep_snapshots: bool,
) -> Recording<Intent> {
    let (configuration, step_configuration, clock_configuration) = settings(FEE_BASIS_POINTS);
    let mut recorder = Recorder::<Tariffs>::new(
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
    let accounts = recorder.runner().simulation().books().accounts.handles();
    for intent in script_between(seed, &accounts, 0, steps) {
        recorder.step_once(&[intent]);
    }
    recorder.into_recording()
}

/// What the first shape of the books would have saved at `LEGACY_SAVE_STEP`: the books of a
/// session that charged no fee, written as snapshot version 1. Used once to write the committed
/// fixture (`examples/generate_legacy_save.rs`); the fixture itself is never regenerated by tests.
pub fn legacy_save(seed: u64) -> Save {
    let runner = run_fixture_with_fee(seed, LEGACY_SAVE_STEP, 0);
    let books = runner.simulation().books().clone();
    let old = BooksV1 {
        accounts: books.accounts,
        balances: books.balances,
        transfers_applied: books.transfers_applied,
        transfers_rejected: books.transfers_rejected,
    };
    let mut save = runner.save(SIMULATION_ID);
    save.snapshot_version = <BooksV1 as Message>::VERSION;
    save.snapshot = lockstep_core::encode(&old);
    save
}
