// SPDX-License-Identifier: Apache-2.0
use ledger::{
    fixture_hash, run_fixture, runner, Event, Intent, Reason, ACCOUNT_COUNT, DEFAULT_SEED,
    DEFAULT_STEPS, OPENING_BALANCE_MINOR,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/ledger.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn money_is_never_created_or_lost_by_transfers() {
    let session = run_fixture(DEFAULT_SEED, 5_000);
    let books = session.simulation().books();
    assert!(books.transfers_applied > 0 && books.transfers_rejected > 0);
    // Only deposits add money; recompute them from a second run that records the events.
    let mut deposited: i128 = 0;
    let mut replay = runner(DEFAULT_SEED);
    let accounts = replay.simulation().books().accounts.handles();
    let mut script = lockstep_core::Streams::new(DEFAULT_SEED ^ 0x6c65_6467_6572);
    for _ in 0..5_000 {
        let from = accounts[script.pick("account", accounts.len())];
        let to = accounts[script.pick("account", accounts.len())];
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
        for event in replay.step_once(&[intent]).events {
            if let Event::Deposited { amount_minor, .. } = event {
                deposited += amount_minor as i128;
            }
        }
    }
    let opening = ACCOUNT_COUNT as i128 * OPENING_BALANCE_MINOR as i128;
    assert_eq!(books.total_minor(), opening + deposited);
    assert_eq!(replay.hash(), session.hash());
}

#[test]
fn a_bad_request_is_rejected_with_its_reason_and_changes_nothing() {
    let mut ledger = runner(1);
    let accounts = ledger.simulation().books().accounts.handles();
    let before = ledger.simulation().books().clone();
    let cases = [
        (
            Intent::Transfer {
                from: accounts[0],
                to: accounts[1],
                amount_minor: 0,
            },
            Reason::NotPositive,
        ),
        (
            Intent::Transfer {
                from: accounts[0],
                to: accounts[0],
                amount_minor: 5,
            },
            Reason::SameAccount,
        ),
        (
            Intent::Transfer {
                from: accounts[0],
                to: accounts[1],
                amount_minor: OPENING_BALANCE_MINOR + 1,
            },
            Reason::InsufficientFunds,
        ),
        (
            Intent::Transfer {
                from: accounts[0],
                to: lockstep_core::Handle::from_raw(0),
                amount_minor: 5,
            },
            Reason::UnknownAccount,
        ),
        (
            Intent::Deposit {
                account: accounts[0],
                amount_minor: i64::MAX,
            },
            Reason::Overflow,
        ),
    ];
    for (intent, reason) in cases {
        let events = ledger.step_once(&[intent]).events;
        assert_eq!(events, vec![Event::Rejected { reason }]);
    }
    let after = ledger.simulation().books();
    assert_eq!(after.balances, before.balances);
}

#[test]
fn a_transfer_moves_exactly_the_amount() {
    let mut ledger = runner(1);
    let accounts = ledger.simulation().books().accounts.handles();
    ledger.step_once(&[Intent::Transfer {
        from: accounts[0],
        to: accounts[1],
        amount_minor: 12_345,
    }]);
    let books = ledger.simulation().books();
    assert_eq!(
        books.balances.get(accounts[0]),
        Some(&(OPENING_BALANCE_MINOR - 12_345))
    );
    assert_eq!(
        books.balances.get(accounts[1]),
        Some(&(OPENING_BALANCE_MINOR + 12_345))
    );
}

#[test]
fn the_same_seed_gives_the_same_hash() {
    assert_eq!(fixture_hash(4, 500), fixture_hash(4, 500));
    assert_ne!(fixture_hash(4, 500), fixture_hash(5, 500));
}
