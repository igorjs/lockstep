// SPDX-License-Identifier: Apache-2.0
//! Decision: units merged into one stack share the higher exposure: a stack spoils as its
//! stalest unit, and a split part keeps the stack's exposure.
//! Alternative rejected: a weighted average by count, which hides one bad unit among fresh ones
//! and changes freshness when a stack is split and merged back.
//! Would change if: merging a half-spoiled unit into fresh ones leaves the stack fresher than
//! that unit (here, fresher than one half).

use crate::common::{whole, Bay};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_half_spoiled_ration_makes_its_whole_stack_half_spoiled() {
    let mut bay = Bay::new();
    let old = bay.make("ration", 1);
    // 720 loose minutes at 18 degrees: half way.
    bay.inventory
        .spoil(whole(720), 18, &bay.catalogue, &mut Vec::new());
    let stack = bay.stock("ration", 5, bay.rack);
    assert_eq!(bay.inventory.put(old, bay.rack, &bay.catalogue), Ok(stack));
    let freshness = bay.inventory.freshness(stack, &bay.catalogue).unwrap();
    assert_eq!(freshness, Fixed32::HALF);
    // Split and merged back, nothing changes.
    let part = bay.inventory.split(stack, 3).unwrap();
    assert_eq!(
        bay.inventory.freshness(part, &bay.catalogue),
        Some(Fixed32::HALF)
    );
    assert_eq!(bay.inventory.put(part, bay.rack, &bay.catalogue), Ok(stack));
    assert_eq!(
        bay.inventory.freshness(stack, &bay.catalogue),
        Some(Fixed32::HALF)
    );
    assert_eq!(bay.inventory.item(stack).unwrap().count, 6);
}
