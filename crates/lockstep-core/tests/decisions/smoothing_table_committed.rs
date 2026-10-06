// SPDX-License-Identifier: Apache-2.0
//! Decision: the smoothed-roll increment for each basis point is found once by a search and
//! committed as `fixtures/smoothing.bin`, and a test regenerates it and compares.
//! Alternative rejected: searching at run time, which costs a search per new chance, or a closed
//! form, which does not exist for this distribution.
//! Would change if: a regenerated entry differs from the committed one (the number to beat is zero
//! differing entries), or a committed increment's long-run rate misses its nominal chance by more
//! than 0.001 percent.

use lockstep_core::{
    search_smoothing_increment, smoothed_rate, smoothing_increment, CERTAIN, SMOOTHING_ENTRIES,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Every entry natively. Under WebAssembly a spread of entries, which proves the same arithmetic
/// gives the same bits there.
fn checked_entries() -> Vec<u32> {
    if cfg!(target_arch = "wasm32") {
        (0..SMOOTHING_ENTRIES as u32)
            .step_by(97)
            .chain([1, 2, CERTAIN - 1, CERTAIN])
            .collect()
    } else {
        (0..SMOOTHING_ENTRIES as u32).collect()
    }
}

#[test]
fn the_committed_table_equals_a_fresh_search() {
    assert_eq!(
        include_bytes!("../../fixtures/smoothing.bin").len(),
        SMOOTHING_ENTRIES * 4
    );
    for points in checked_entries() {
        assert_eq!(
            smoothing_increment(points),
            search_smoothing_increment(points),
            "{points} basis points"
        );
    }
}

#[test]
fn every_committed_increment_lands_on_its_nominal_rate() {
    for points in checked_entries() {
        if points == 0 || points == CERTAIN {
            continue;
        }
        let rate = smoothed_rate(smoothing_increment(points) as u64);
        let nominal = points as f64 / CERTAIN as f64;
        assert!(
            (rate - nominal).abs() < 0.000_01,
            "{points} basis points land at {rate}"
        );
    }
}
