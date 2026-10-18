// SPDX-License-Identifier: Apache-2.0
//! Decision: a season's values (daylight and mean temperature) hold at the middle of the season
//! and change in a straight line to the next season's middle, so the longest day is in the middle
//! of summer and the shortest in the middle of winter, and no day jumps at a season boundary.
//! Alternative rejected: holding a season's values for all its days, which steps the daylight by
//! up to a whole season's difference on the first day of the next one.
//! Would change if: the longest day of the twelve day test year is not day 4, the shortest is not
//! day 10, or daylight changes by more than 100 minutes between any two days.

use crate::common::{calendar, daylight_minutes};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_longest_day_is_midsummer_and_no_day_jumps() {
    let calendar = calendar();
    let lengths: Vec<u32> = (0..13)
        .map(|day| daylight_minutes(&calendar, day))
        .collect();
    let longest = lengths
        .iter()
        .position(|&length| length == *lengths.iter().max().unwrap());
    let shortest = lengths
        .iter()
        .position(|&length| length == *lengths.iter().min().unwrap());
    assert_eq!((longest, shortest), (Some(4), Some(10)));
    for pair in lengths.windows(2) {
        assert!(pair[0].abs_diff(pair[1]) <= 100, "{pair:?}");
    }
}
