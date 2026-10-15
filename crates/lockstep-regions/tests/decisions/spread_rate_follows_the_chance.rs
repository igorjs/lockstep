// SPDX-License-Identifier: Apache-2.0
//! Decision (the gate): a try is a draw of one in a thousand against the chance, so the number of
//! intervals a condition takes to cross a link is geometric: with a chance of 300 in a thousand
//! it averages 1 / 0.3 = 3.33 intervals, and with the pace halved 6.67.
//! Alternative rejected: a fixed delay per link, which makes the rate a constant and gives no
//! spread that is faster by luck, wind or season.
//! Would change if: the mean over 20,000 crossings is more than 3 percent from the expected.

use crate::common::{calm, id, regions, sure};
use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_regions::{Spread, SpreadRules};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// The mean number of intervals until `a` starts `b`, over 20,000 seeds.
fn mean_intervals(pace_percent: u32) -> f64 {
    let regions = regions();
    let rules = SpreadRules {
        base_permille: 300,
        // Long enough that the source never burns out before the crossing in these trials.
        active_intervals: 1_000,
        ..sure()
    };
    let (a, b) = (id(&regions, "a"), id(&regions, "b"));
    let mut total = 0u64;
    for seed in 0..20_000 {
        let mut spread = Spread::new(&regions);
        let mut streams = Streams::new(seed);
        spread.start(a, &rules, &mut Vec::new());
        let mut intervals = 0u64;
        while !spread.is_active(b) {
            spread.advance(
                &regions,
                &rules,
                calm(),
                pace_percent,
                Fixed32::from_int(60),
                &mut streams,
                &mut Vec::new(),
            );
            intervals += 1;
        }
        total += intervals;
    }
    total as f64 / 20_000.0
}

#[test]
fn the_mean_crossing_time_is_the_inverse_of_the_chance() {
    let full = mean_intervals(100);
    assert!((full - 10.0 / 3.0).abs() < 10.0 / 3.0 * 0.03, "{full}");
    let half = mean_intervals(50);
    assert!((half - 20.0 / 3.0).abs() < 20.0 / 3.0 * 0.03, "{half}");
}
