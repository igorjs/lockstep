// SPDX-License-Identifier: Apache-2.0
//! Shared bills: one account pays a bill and the others owe it their share. The shares come from
//! `Money::allocate`, so they add up to the bill exactly, and each owed share becomes an ordinary
//! transfer intent. The ledger itself is unchanged.

use crate::{runner, Intent, Ledger};
use lockstep_core::math::{Currency, Money, MoneyError};
use lockstep_core::{Handle, Runner, Streams};

/// The ledger's one currency: two minor unit digits.
pub fn currency() -> Currency {
    Currency::new("USD", 2).expect("a valid currency")
}

pub const DEFAULT_ROUNDS: u64 = 2_000;

/// The transfers that settle a bill: every payee except the payer sends the payer its share.
/// Shares of zero are left out, since a transfer must be positive. The shares of all payees,
/// the payer's own included, add up to `bill`.
pub fn settle_intents(
    payer: Handle,
    payees: &[Handle],
    weights: &[u64],
    bill: Money,
) -> Result<Vec<Intent>, MoneyError> {
    if payees.len() != weights.len() {
        return Err(MoneyError::NothingToShareWith);
    }
    if bill.currency() != currency() {
        return Err(MoneyError::CurrencyMismatch);
    }
    if bill.is_negative() {
        return Err(MoneyError::NothingToShareWith);
    }
    let shares = bill.allocate(weights)?;
    Ok(payees
        .iter()
        .zip(&shares)
        .filter(|(payee, share)| **payee != payer && share.minor() > 0)
        .map(|(payee, share)| Intent::Transfer {
            from: *payee,
            to: payer,
            amount_minor: share.minor(),
        })
        .collect())
}

/// A scripted session of bills: each round a random account pays a random bill split by random
/// weights among all accounts, settled in one step. Transfers only move money, so the total never
/// changes.
pub fn run_bill_fixture(seed: u64, rounds: u64) -> Runner<Ledger> {
    let mut runner = runner(seed);
    let accounts = runner.simulation().books().accounts.handles();
    let mut script = Streams::new(seed ^ 0x0062_696c_6c73);
    for _ in 0..rounds {
        let payer = accounts[script.pick("payer", accounts.len())];
        let bill = Money::from_minor(script.range("bill", 1, 50_000) as i64, currency());
        let mut weights: Vec<u64> = accounts
            .iter()
            .map(|_| script.range("weight", 0, 6) as u64)
            .collect();
        if weights.iter().all(|weight| *weight == 0) {
            weights[0] = 1;
        }
        let intents = settle_intents(payer, &accounts, &weights, bill).expect("a valid bill");
        runner.step_once(&intents);
    }
    runner
}

pub fn bill_fixture_hash(seed: u64, rounds: u64) -> u64 {
    run_bill_fixture(seed, rounds).hash()
}
