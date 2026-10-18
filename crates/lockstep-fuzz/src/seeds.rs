// SPDX-License-Identifier: Apache-2.0
use ledger::{Books, Configuration, Event, Intent, Reason};
use lockstep_core::math::{Currency, Money};
use lockstep_core::{encode, Column, Handle, StableVector};

/// The binary seeds: (target, file name, bytes). The JSON seeds are the committed data files.
/// `lockstep-fuzz write-corpus` writes these into `corpus/`.
pub fn binary_seeds() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let recording = |steps, every| {
        ledger::record_fixture(7, steps, every, true)
            .to_bytes()
            .expect("small")
    };
    // Found by fuzzing: a day of astronomical length used to overflow the clock.
    let astronomical_day = {
        let mut recording = ledger::record_fixture(7, 6, 3, false);
        recording.clock_configuration.day_length_real_minutes = f32::MAX;
        recording.step_configuration.step_seconds = 0.0;
        recording.to_bytes().expect("small")
    };
    let mut accounts = StableVector::new();
    let first = accounts.insert("account-0".to_string());
    let second = accounts.insert("account-1".to_string());
    let mut balances = Column::new();
    balances.set(first, 120_050);
    balances.set(second, -3);
    let books = Books {
        accounts,
        balances,
        transfers_applied: 3,
        transfers_rejected: 1,
    };
    let dollars = Currency::new("USD", 2).expect("valid");
    vec![
        ("decode_recording", "twelve-steps.bin", recording(12, 4)),
        ("decode_recording", "no-steps.bin", recording(0, 1)),
        ("decode_recording", "astronomical-day.bin", astronomical_day),
        ("decode_save", "two-accounts.bin", encode(&books)),
        (
            "decode_message",
            "configuration.bin",
            encode(&Configuration {
                account_names: vec!["a".into(), "b".into()],
                opening_balance_minor: 5,
            }),
        ),
        (
            "decode_message",
            "intent.bin",
            encode(&Intent::Transfer {
                from: Handle::from_raw(0),
                to: Handle::from_raw(1),
                amount_minor: 9,
            }),
        ),
        (
            "decode_message",
            "event.bin",
            encode(&Event::Rejected {
                reason: Reason::Overflow,
            }),
        ),
        (
            "decode_message",
            "money.bin",
            encode(&Money::from_minor(-1_205, dollars)),
        ),
        (
            "arithmetic_fixed",
            "edges.bin",
            [
                i32::MIN.to_le_bytes(),
                i32::MAX.to_le_bytes(),
                (-1i32).to_le_bytes(),
                65_536i32.to_le_bytes(),
            ]
            .concat(),
        ),
        ("arithmetic_fixed", "decimal.bin", b"-12.375".to_vec()),
        (
            "arithmetic_money",
            "edges.bin",
            [
                b"USD".to_vec(),
                vec![2],
                i64::MIN.to_le_bytes().to_vec(),
                i64::MAX.to_le_bytes().to_vec(),
                vec![3, 2, 1, 7],
            ]
            .concat(),
        ),
    ]
}
