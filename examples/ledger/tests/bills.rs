// SPDX-License-Identifier: Apache-2.0
use ledger::bills::{
    bill_fixture_hash, currency, run_bill_fixture, settle_intents, DEFAULT_ROUNDS,
};
use ledger::{runner, Event, Intent, ACCOUNT_COUNT, DEFAULT_SEED, OPENING_BALANCE_MINOR};
use lockstep_core::math::{Money, MoneyError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_bill_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/ledger-bills.hash").trim();
    assert_eq!(
        format!("{:016x}", bill_fixture_hash(DEFAULT_SEED, DEFAULT_ROUNDS)),
        committed
    );
}

#[test]
fn settling_bills_only_moves_money() {
    let session = run_bill_fixture(DEFAULT_SEED, 500);
    let books = session.simulation().books();
    assert!(books.transfers_applied > 0);
    assert_eq!(
        books.total_minor(),
        ACCOUNT_COUNT as i128 * OPENING_BALANCE_MINOR as i128
    );
}

#[test]
fn the_owed_shares_and_the_payers_own_share_add_up_to_the_bill() {
    let session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let bill = Money::from_minor(10_001, currency());
    let weights = [1, 1, 1, 0, 2, 0, 1, 1];
    let intents = settle_intents(accounts[2], &accounts, &weights, bill).unwrap();
    let owed: i64 = intents
        .iter()
        .map(|intent| match intent {
            Intent::Transfer { amount_minor, .. } => *amount_minor,
            Intent::Deposit { .. } => unreachable!("a bill only transfers"),
        })
        .sum();
    let own = bill.allocate(&weights).unwrap()[2].minor();
    assert_eq!(owed + own, bill.minor());
    // The payer owes nothing to itself and zero shares make no transfer.
    assert_eq!(intents.len(), 5);
}

#[test]
fn a_settled_bill_moves_each_share_to_the_payer() {
    let mut session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let weights = [1; ACCOUNT_COUNT];
    let bill = Money::from_minor(803, currency());
    let intents = settle_intents(accounts[0], &accounts, &weights, bill).unwrap();
    let events = session.step_once(&intents).events;
    assert!(events
        .iter()
        .all(|event| matches!(event, Event::Transferred { .. })));
    let books = session.simulation().books();
    let gained = books.balances.get(accounts[0]).unwrap() - OPENING_BALANCE_MINOR;
    assert_eq!(gained, 803 - bill.allocate(&weights).unwrap()[0].minor());
}

#[test]
fn a_bill_with_no_one_to_share_it_is_refused() {
    let session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let bill = Money::from_minor(100, currency());
    assert_eq!(
        settle_intents(accounts[0], &accounts, &[0; ACCOUNT_COUNT], bill),
        Err(MoneyError::NothingToShareWith)
    );
    assert_eq!(
        settle_intents(accounts[0], &accounts, &[1, 2], bill),
        Err(MoneyError::NothingToShareWith)
    );
}

#[test]
fn a_refund_or_a_bill_in_another_currency_is_refused() {
    let session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let weights = [1; ACCOUNT_COUNT];
    assert_eq!(
        settle_intents(
            accounts[0],
            &accounts,
            &weights,
            Money::from_minor(-5, currency())
        ),
        Err(MoneyError::NothingToShareWith)
    );
    let yen = lockstep_core::math::Currency::new("JPY", 0).unwrap();
    assert_eq!(
        settle_intents(accounts[0], &accounts, &weights, Money::from_minor(5, yen)),
        Err(MoneyError::CurrencyMismatch)
    );
}
