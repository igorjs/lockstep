// SPDX-License-Identifier: Apache-2.0
//! Decision: an item carrying a binding affix cannot be taken off, and the affix can be removed
//! while the item is worn (a repair), which lifts the binding and its modifiers at once.
//! Alternative rejected: letting only an unworn item lose an affix, which would leave a bound
//! item worn for ever.
//! Would change if: a suit with a faulty seal comes off, or stays bound after the seal is fixed.

use crate::common::{whole, Bay};
use lockstep_inventory::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_suit_with_a_faulty_seal_stays_on_until_the_seal_is_fixed() {
    let mut bay = Bay::new();
    let suit = bay.make("suit", 1);
    bay.add_affix(suit, "faulty_seal").unwrap();
    bay.equip(suit).unwrap();
    // 100, plus 50 for the suit, minus 20 for the seal.
    assert_eq!(bay.value("oxygen").1, whole(130));
    assert_eq!(bay.unequip("suit"), Err(Refusal::Bound));
    bay.remove_affix(suit, "faulty_seal").unwrap();
    assert_eq!(bay.value("oxygen").1, whole(150));
    assert_eq!(bay.unequip("suit"), Ok(suit));
    assert_eq!(bay.value("oxygen").1, whole(100));
}
