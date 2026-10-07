// SPDX-License-Identifier: Apache-2.0
//! Decision: an exclusion declared on either node of a pair locks both ways: taking one refuses
//! the other, naming the node that excludes it, until the first is refunded.
//! Alternative rejected: exclusions only in the direction written, which lets a designer forget
//! the other side and an owner take both.
//! Would change if: an owner holding electrical can take mechanical, or the reverse, though only
//! electrical declares the exclusion.

use crate::common::Crew;
use lockstep_progression::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn either_side_of_an_exclusion_locks_the_other() {
    let mut crew = Crew::new(20);
    crew.unlock("basics").unwrap();
    crew.unlock("electrical").unwrap();
    let electrical = crew.node("electrical");
    assert_eq!(
        crew.unlock("mechanical"),
        Err(Refusal::ExcludedBy(electrical))
    );
    crew.refund("electrical").unwrap();
    crew.unlock("mechanical").unwrap();
    let mechanical = crew.node("mechanical");
    assert_eq!(
        crew.unlock("electrical"),
        Err(Refusal::ExcludedBy(mechanical))
    );
}
