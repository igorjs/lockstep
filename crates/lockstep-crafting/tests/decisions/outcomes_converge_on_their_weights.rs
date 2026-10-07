// SPDX-License-Identifier: Apache-2.0
//! Decision: an outcome is drawn from the `"crafting"` stream as one number below the table's
//! total weight, taking the row whose running total passes it. Over many draws each row's share
//! converges on its weight, and the same seed draws the same rows on every platform.
//! Alternative rejected: one roll per row in turn, which skews toward the rows listed first.
//! Would change if: 100,000 draws of a table of 85, 10 and 5 land outside half a percent of
//! each weight.

use lockstep_core::Streams;
use lockstep_crafting::roll;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_hundred_thousand_draws_land_within_half_a_percent_of_each_weight() {
    let weights = [85u32, 10, 5];
    let mut streams = Streams::new(7);
    let mut counts = [0u32; 3];
    for _ in 0..100_000 {
        counts[roll(weights.iter().copied(), &mut streams)] += 1;
    }
    for (count, weight) in counts.iter().zip(weights) {
        let expected = weight * 1_000;
        assert!(count.abs_diff(expected) <= 500, "{counts:?}");
    }
    // A weight of zero is never drawn.
    let mut never = 0;
    for _ in 0..10_000 {
        never += usize::from(roll([3u32, 0, 1].iter().copied(), &mut streams) == 1);
    }
    assert_eq!(never, 0);
}
