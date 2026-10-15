// SPDX-License-Identifier: Apache-2.0
//! Decision: the calendar is integers only, so a year of daylight and of temperature every six
//! hours hashes to the committed value natively and under WebAssembly.
//! Alternative rejected: floating point sunrise curves, whose last digits differ across
//! platforms.
//! Would change if: the hash changes without a commit message naming the calendar rule that
//! changed (seasons at their middles, a straight line between them, or the temperature swing).

use crate::common::calendar;
use lockstep_core::hash_of;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn two_years_of_days_match_the_committed_hash() {
    let calendar = calendar();
    let mut samples = Vec::new();
    for day in 0..24 {
        samples.push((
            day,
            calendar.daylight(day).0 as i32,
            calendar.daylight(day).1 as i32,
        ));
        for minute in (0..1_440).step_by(360) {
            samples.push((day, minute, calendar.temperature(day, minute as u32).raw()));
        }
    }
    let committed = include_str!("../fixtures/year.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&samples)), committed);
}
