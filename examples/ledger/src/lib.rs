// SPDX-License-Identifier: Apache-2.0
//! The ledger: a consumer scenario that is not a game.
//!
//! Accounts hold whole minor units (cents, pence) as integers. Transfers either move money
//! exactly or are rejected with a reason; no money is created or lost. The framework has no
//! money type yet (it arrives in milestone M16), so this scenario uses plain integers and a
//! single currency, which also shows the core needs nothing game-specific.

use lockstep_core::ClockConfiguration;
use lockstep_core::{
    Column, Context, Handle, Message, Runner, Simulation, StableVector, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub account_names: Vec<String>,
    pub opening_balance_minor: i64,
}

impl Message for Configuration {
    const VERSION: u32 = 1;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    UnknownAccount,
    NotPositive,
    SameAccount,
    InsufficientFunds,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

impl Message for Intent {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Deposited {
        account: Handle,
        amount_minor: i64,
    },
    Transferred {
        from: Handle,
        to: Handle,
        amount_minor: i64,
    },
    Rejected {
        reason: Reason,
    },
}

impl Message for Event {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Books {
    pub accounts: StableVector<String>,
    pub balances: Column<i64>,
    pub transfers_applied: u64,
    pub transfers_rejected: u64,
}

impl Message for Books {
    const VERSION: u32 = 1;
}

impl Books {
    pub fn total_minor(&self) -> i128 {
        self.balances
            .iter()
            .map(|(_, balance)| *balance as i128)
            .sum()
    }
}

pub struct Ledger {
    books: Books,
}

impl Ledger {
    pub fn books(&self) -> &Books {
        &self.books
    }
}

impl Simulation for Ledger {
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
        };
        for name in configuration.account_names {
            let account = books.accounts.insert(name);
            books
                .balances
                .set(account, configuration.opening_balance_minor);
        }
        Ledger { books }
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
                    .map(|()| Event::Transferred {
                        from: *from,
                        to: *to,
                        amount_minor: *amount_minor,
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
        Ledger { books: snapshot }
    }
}

impl Ledger {
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

    fn transfer(&mut self, from: Handle, to: Handle, amount_minor: i64) -> Result<(), Reason> {
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
        if source < amount_minor {
            return Err(Reason::InsufficientFunds);
        }
        let new_target = target.checked_add(amount_minor).ok_or(Reason::Overflow)?;
        self.books.balances.set(from, source - amount_minor);
        self.books.balances.set(to, new_target);
        Ok(())
    }
}

pub const DEFAULT_SEED: u64 = 20_260_925;
pub const DEFAULT_STEPS: u64 = 10_000;
pub const ACCOUNT_COUNT: usize = 8;
pub const OPENING_BALANCE_MINOR: i64 = 100_000;

pub fn runner(seed: u64) -> Runner<Ledger> {
    let names = (0..ACCOUNT_COUNT)
        .map(|index| format!("account-{index}"))
        .collect();
    Runner::new(
        Configuration {
            account_names: names,
            opening_balance_minor: OPENING_BALANCE_MINOR,
        },
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

/// The scripted session the fixture hash covers: one random intent per step, drawn from a
/// separate stream set so the script depends only on the seed.
pub fn run_fixture(seed: u64, steps: u64) -> Runner<Ledger> {
    let mut runner = runner(seed);
    let accounts = runner.simulation().books().accounts.handles();
    let mut script = Streams::new(seed ^ 0x6c65_6467_6572);
    for _ in 0..steps {
        let from = accounts[script.pick("account", accounts.len())];
        let to = accounts[script.pick("account", accounts.len())];
        // Amounts run past the opening balance on purpose, so some transfers are rejected.
        let amount_minor = script.range("amount", -50, 40_000) as i64;
        let intent = if script.chance("kind", 0.05) {
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
        };
        runner.step_once(&[intent]);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
