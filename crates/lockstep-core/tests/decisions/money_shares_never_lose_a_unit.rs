// SPDX-License-Identifier: Apache-2.0
//! Decision: money is whole minor units of one currency in an `i64`, every operation is checked,
//! and sharing an amount uses the largest remainder method so the parts always add up to the
//! amount exactly.
//! Alternative rejected: a float or a fixed point amount, which rounds each part on its own and
//! so loses or creates a unit (a third of 1.00, three times, is 0.99); and rounding the last part
//! to absorb the difference, which hides who paid the extra unit.
//! Would change if: any share of 20,000 random amounts and weight lists differs from the sum of
//! its parts by even one minor unit, or a share differs from its exact proportion by a whole unit.

use lockstep_core::math::{Currency, Money};
use lockstep_core::Streams;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn parts_always_add_up_and_stay_within_a_unit_of_the_exact_share() {
    let currency = Currency::new("USD", 2).unwrap();
    let mut streams = Streams::new(20_261_019);
    for _ in 0..20_000 {
        let total = streams.range("total", -2_000_000, 2_000_000) as i64;
        let count = streams.range("count", 1, 9) as usize;
        let weights: Vec<u64> = (0..count)
            .map(|_| streams.range("weight", 0, 50) as u64)
            .collect();
        let weight_sum: u64 = weights.iter().sum();
        let money = Money::from_minor(total, currency);
        let parts = match money.allocate(&weights) {
            Ok(parts) => parts,
            Err(_) => {
                assert_eq!(weight_sum, 0);
                continue;
            }
        };
        let sum: i64 = parts.iter().map(|part| part.minor()).sum();
        assert_eq!(sum, total);
        for (part, weight) in parts.iter().zip(&weights) {
            let exact = total as i128 * *weight as i128;
            let difference = part.minor() as i128 * weight_sum as i128 - exact;
            assert!(difference.abs() < weight_sum as i128, "within one unit");
        }
    }
}

#[test]
fn a_third_three_times_loses_nothing() {
    let currency = Currency::new("USD", 2).unwrap();
    let parts = Money::from_minor(100, currency).split(3).unwrap();
    assert_eq!(
        Money::checked_sum(parts, currency),
        Ok(Money::from_minor(100, currency))
    );
}
