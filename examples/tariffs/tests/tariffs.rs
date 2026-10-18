// SPDX-License-Identifier: Apache-2.0
use tariffs::{
    fee_minor, fixture_hash, run_fixture, runner, Event, Intent, Reason, DEFAULT_SEED,
    DEFAULT_STEPS, FEE_BASIS_POINTS, OPENING_BALANCE_MINOR,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/tariffs.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn a_fee_is_charged_on_top_and_rounded_down() {
    assert_eq!(fee_minor(10_000, 25), 25);
    assert_eq!(fee_minor(199, 25), 0);
    assert_eq!(fee_minor(399, 25), 0);
    assert_eq!(fee_minor(400, 25), 1);
    let mut session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let events = session
        .step_once(&[Intent::Transfer {
            from: accounts[0],
            to: accounts[1],
            amount_minor: 40_000,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Transferred {
            from: accounts[0],
            to: accounts[1],
            amount_minor: 40_000,
            fee_minor: 100
        }]
    );
    let books = session.simulation().books();
    assert_eq!(
        books.balances.get(accounts[0]),
        Some(&(OPENING_BALANCE_MINOR - 40_100))
    );
    assert_eq!(books.fees_collected_minor, 100);
}

#[test]
fn a_sender_who_cannot_cover_the_fee_is_rejected_and_nothing_moves() {
    let mut session = runner(1);
    let accounts = session.simulation().books().accounts.handles();
    let before = session.simulation().books().clone();
    // The amount alone would fit; the fee on top does not.
    let events = session
        .step_once(&[Intent::Transfer {
            from: accounts[0],
            to: accounts[1],
            amount_minor: OPENING_BALANCE_MINOR,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Rejected {
            reason: Reason::InsufficientFunds
        }]
    );
    assert_eq!(session.simulation().books().balances, before.balances);
    assert_eq!(session.simulation().books().fees_collected_minor, 0);
}

#[test]
fn balances_and_fees_together_equal_the_opening_money_plus_deposits() {
    let mut deposited: i128 = 0;
    let mut session = runner(DEFAULT_SEED);
    let accounts = session.simulation().books().accounts.handles();
    for intent in tariffs::script_between(DEFAULT_SEED, &accounts, 0, 3_000) {
        for event in session.step_once(&[intent]).events {
            if let Event::Deposited { amount_minor, .. } = event {
                deposited += amount_minor as i128;
            }
        }
    }
    let books = session.simulation().books();
    let opening = accounts.len() as i128 * OPENING_BALANCE_MINOR as i128;
    assert_eq!(books.total_minor(), opening + deposited);
    assert!(books.fees_collected_minor > 0 && books.transfers_rejected > 0);
    assert_eq!(session.hash(), run_fixture(DEFAULT_SEED, 3_000).hash());
    assert_eq!(FEE_BASIS_POINTS, books.fee_basis_points);
}

/// A message type whose shape changed without a new version fails here, natively and under
/// WebAssembly (decision 0063): bump `#[message(version = N)]` and save the new schema with
/// `lockstep-headless schema tariffs --out examples/tariffs/fixtures/schema`.
#[test]
fn the_message_schemas_match_the_saved_ones_natively_and_under_webassembly() {
    use lockstep_core::Message;
    use tariffs::{Books, Configuration};
    for (name, schema, saved) in [
        (
            "Intent",
            <Intent as Message>::SCHEMA,
            include_str!("../fixtures/schema/Intent.v1.json"),
        ),
        (
            "Event",
            <Event as Message>::SCHEMA,
            include_str!("../fixtures/schema/Event.v1.json"),
        ),
        (
            "Books",
            <Books as Message>::SCHEMA,
            include_str!("../fixtures/schema/Books.v2.json"),
        ),
        (
            "Configuration",
            <Configuration as Message>::SCHEMA,
            include_str!("../fixtures/schema/Configuration.v1.json"),
        ),
    ] {
        assert_eq!(format!("{schema}\n"), saved, "{name}: bump its version");
    }
}
