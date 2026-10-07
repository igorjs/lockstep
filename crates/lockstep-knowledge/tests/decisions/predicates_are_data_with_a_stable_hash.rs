// SPDX-License-Identifier: Apache-2.0
//! Decision: predicates are data, a tree of known, fragments, opened, fired, happened, all, any
//! and not, read from JSON. A catalogue can be saved, compared and hashed, and its hash is the
//! same on every platform.
//! Alternative rejected: predicates as closures, which cannot be saved, compared or hashed, so a
//! save could not say which rules it was made with.
//! Would change if: the test catalogue's hash differs from the committed value natively or under
//! WebAssembly, without a commit naming the predicate rule that changed.

use crate::common::catalogue;
use lockstep_core::hash_of;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_catalogue_hash_matches_the_committed_value() {
    let committed = include_str!("../fixtures/predicates.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&catalogue())), committed);
}
