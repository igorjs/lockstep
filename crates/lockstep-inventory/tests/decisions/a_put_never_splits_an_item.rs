// SPDX-License-Identifier: Apache-2.0
//! Decision: a put merges an item into a stack only when every unit fits; otherwise the item
//! takes a slot of its own, or is refused. A put never splits an item, so the caller always
//! knows which item holds the units afterwards. Split first to top a stack up.
//! Alternative rejected: filling the stack and leaving the rest loose or in a new slot, which
//! makes one put touch two items and fail half way when the container is full.
//! Would change if: putting 4 rations beside a stack of 4 (stack limit 6) changes the stack.

use crate::common::Bay;
use lockstep_inventory::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn four_rations_beside_a_stack_of_four_take_their_own_slot() {
    let mut bay = Bay::new();
    let stack = bay.stock("ration", 4, bay.cold);
    let more = bay.make("ration", 4);
    assert_eq!(bay.inventory.put(more, bay.cold, &bay.catalogue), Ok(more));
    assert_eq!(bay.inventory.item(stack).unwrap().count, 4);
    assert_eq!(
        bay.inventory.container(bay.cold).unwrap().items(),
        &[stack, more]
    );
    // The cold store is now full: a third lot that does not fit whole is refused.
    let third = bay.make("ration", 3);
    assert_eq!(
        bay.inventory.put(third, bay.cold, &bay.catalogue),
        Err(Refusal::Full)
    );
    // Split to top up: two fit into the first stack.
    let two = bay.inventory.split(third, 2).unwrap();
    assert_eq!(bay.inventory.put(two, bay.cold, &bay.catalogue), Ok(stack));
    assert_eq!(bay.inventory.item(stack).unwrap().count, 6);
}
