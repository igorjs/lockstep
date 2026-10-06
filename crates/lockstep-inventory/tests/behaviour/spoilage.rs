// SPDX-License-Identifier: Apache-2.0
use crate::common::{whole, Bay};
use lockstep_core::math::Fixed32;
use lockstep_inventory::InventoryEvent;

/// A fraction of one, rounded down, as freshness is.
fn down(numerator: i64, denominator: i64) -> Fixed32 {
    Fixed32::from_raw((65_536 * numerator / denominator) as i32)
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Rations spoil after 1,440 minutes at 100 percent: the rack (18 degrees) is 100 percent, the
/// cold store (2 degrees) 25, and outside at 35 degrees 300.
#[test]
fn spoilage_runs_at_the_containers_temperature() {
    let mut bay = Bay::new();
    let racked = bay.stock("ration", 1, bay.rack);
    let cold = bay.stock("ration", 1, bay.cold);
    let loose = bay.make("ration", 1);
    let mut events = Vec::new();
    bay.inventory
        .spoil(whole(480), 35, &bay.catalogue, &mut events);
    let fresh = |bay: &Bay, item| bay.inventory.freshness(item, &bay.catalogue).unwrap();
    assert_eq!(fresh(&bay, racked), down(2, 3));
    assert_eq!(fresh(&bay, cold), down(11, 12));
    assert_eq!(fresh(&bay, loose), Fixed32::ZERO);
    assert_eq!(events, vec![InventoryEvent::Spoiled { item: loose }]);
    assert!(bay.inventory.item(loose).unwrap().spoiled);
}

#[test]
fn spoiled_fires_once_and_a_kind_that_never_spoils_has_no_freshness() {
    let mut bay = Bay::new();
    let ration = bay.stock("ration", 1, bay.rack);
    let water = bay.stock("water", 1, bay.rack);
    let mut events = Vec::new();
    for _ in 0..4 {
        bay.inventory
            .spoil(whole(720), 18, &bay.catalogue, &mut events);
    }
    assert_eq!(events, vec![InventoryEvent::Spoiled { item: ration }]);
    assert_eq!(
        bay.inventory.freshness(ration, &bay.catalogue),
        Some(Fixed32::ZERO)
    );
    assert_eq!(bay.inventory.freshness(water, &bay.catalogue), None);
    assert_eq!(bay.inventory.item(water).unwrap().exposure(), 0);
}

#[test]
fn a_warmer_container_spoils_faster_from_the_moment_it_warms() {
    let mut bay = Bay::new();
    let cold = bay.stock("ration", 1, bay.cold);
    let mut events = Vec::new();
    bay.inventory
        .spoil(whole(960), 35, &bay.catalogue, &mut events);
    assert_eq!(
        bay.inventory.freshness(cold, &bay.catalogue),
        Some(down(5, 6))
    );
    // The cold store loses power and warms to 18 degrees: the rest goes at 100 percent.
    bay.inventory.set_temperature(bay.cold, 18).unwrap();
    bay.inventory
        .spoil(whole(1_200), 35, &bay.catalogue, &mut events);
    assert_eq!(events, vec![InventoryEvent::Spoiled { item: cold }]);
}
