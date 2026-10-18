// SPDX-License-Identifier: Apache-2.0
//! Decision: the spread draws from the `"spread"` stream in a fixed order (regions by number,
//! neighbours lowest first), so a spread from seed 42 hashes to the committed value natively and
//! under WebAssembly.
//! Alternative rejected: an order that follows how regions were reached, which would change the
//! draws whenever the graph is written in another order.
//! Would change if: the hash changes without a commit message naming the spread rule that
//! changed.

use crate::common::{hours, id, regions};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Streams};
use lockstep_regions::{Breeze, Spread, SpreadRules};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_spread_from_seed_42_matches_the_committed_hash() {
    let regions = regions();
    let rules = SpreadRules {
        base_permille: 350,
        interval_minutes: 30,
        active_intervals: 5,
        wind_percent_per_metre: 4,
    };
    let breeze = Breeze {
        direction: 3_000,
        strength: Fixed32::from_int(6),
    };
    let mut spread = Spread::new(&regions);
    let mut streams = Streams::new(42);
    let mut events = Vec::new();
    spread.start(id(&regions, "a"), &rules, &mut events);
    events.extend(hours(
        &mut spread,
        &regions,
        &rules,
        breeze,
        80,
        24,
        &mut streams,
    ));
    let committed = include_str!("../fixtures/spread.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&(spread, events))), committed);
}
