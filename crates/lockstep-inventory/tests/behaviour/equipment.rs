// SPDX-License-Identifier: Apache-2.0
use crate::common::{whole, Bay};
use lockstep_inventory::{Place, Refusal};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn wearing_applies_the_kinds_modifiers_and_taking_off_removes_them() {
    let mut bay = Bay::new();
    let suit = bay.stock("suit", 1, bay.rack);
    let slot = bay.equip(suit).unwrap();
    assert_eq!(slot, bay.catalogue.slot_id("suit").unwrap());
    assert_eq!(
        bay.inventory.place(suit),
        Some(Place::Worn {
            owner: bay.crew,
            slot
        })
    );
    assert!(
        bay.inventory
            .container(bay.rack)
            .unwrap()
            .items()
            .is_empty(),
        "taken from the rack"
    );
    assert_eq!(bay.value("oxygen").1, whole(150));
    assert_eq!(bay.unequip("suit"), Ok(suit));
    assert_eq!(bay.inventory.place(suit), Some(Place::Loose));
    assert_eq!(bay.value("oxygen").1, whole(100));
    assert_eq!(bay.unequip("suit"), Err(Refusal::NothingWorn));
}

#[test]
fn a_slot_holds_one_item_of_its_own_kind() {
    let mut bay = Bay::new();
    let (first, second) = (bay.make("suit", 1), bay.make("suit", 1));
    bay.equip(first).unwrap();
    assert_eq!(bay.equip(second), Err(Refusal::SlotTaken));
    let water = bay.make("water", 1);
    assert_eq!(bay.equip(water), Err(Refusal::NotWearable));
    let wrench = bay.make("wrench", 1);
    assert_eq!(
        bay.equip(wrench),
        Ok(bay.catalogue.slot_id("hand").unwrap())
    );
    assert_eq!(bay.equip(wrench), Err(Refusal::SlotTaken));
}

#[test]
fn a_worn_item_cannot_be_put_moved_split_consumed_or_destroyed() {
    let mut bay = Bay::new();
    let suit = bay.make("suit", 1);
    bay.equip(suit).unwrap();
    assert_eq!(
        bay.inventory.put(suit, bay.rack, &bay.catalogue),
        Err(Refusal::NotLoose)
    );
    assert_eq!(
        bay.inventory.move_between(suit, bay.rack, &bay.catalogue),
        Err(Refusal::Worn)
    );
    assert_eq!(bay.inventory.take(suit), Err(Refusal::Worn));
    assert_eq!(bay.inventory.consume(suit, 1), Err(Refusal::Worn));
    assert_eq!(bay.inventory.destroy(suit), Err(Refusal::Worn));
}

#[test]
fn affixes_fill_slots_and_apply_on_and_off_the_wearer() {
    let mut bay = Bay::new();
    let wrench = bay.make("wrench", 1);
    bay.add_affix(wrench, "steady_grip").unwrap();
    assert_eq!(
        bay.add_affix(wrench, "steady_grip"),
        Err(Refusal::NoAffixSlot)
    );
    bay.equip(wrench).unwrap();
    assert_eq!(bay.value("focus").1, whole(110));
    // Taken off while worn: its modifier goes at once.
    bay.remove_affix(wrench, "steady_grip").unwrap();
    assert_eq!(bay.value("focus").1, whole(100));
    assert_eq!(
        bay.remove_affix(wrench, "steady_grip"),
        Err(Refusal::NoSuchAffix)
    );
    // Added while worn: its modifier comes at once.
    bay.add_affix(wrench, "steady_grip").unwrap();
    assert_eq!(bay.value("focus").1, whole(110));
}

#[test]
fn the_same_affix_twice_counts_twice_and_comes_off_one_at_a_time() {
    let mut bay = Bay::new();
    let suit = bay.make("suit", 1);
    bay.add_affix(suit, "faulty_seal").unwrap();
    bay.add_affix(suit, "faulty_seal").unwrap();
    bay.equip(suit).unwrap();
    assert_eq!(bay.value("oxygen").1, whole(110));
    bay.remove_affix(suit, "faulty_seal").unwrap();
    assert_eq!(bay.value("oxygen").1, whole(130));
    assert_eq!(
        bay.unequip("suit"),
        Err(Refusal::Bound),
        "one seal still binds"
    );
    bay.remove_affix(suit, "faulty_seal").unwrap();
    assert_eq!(bay.value("oxygen").1, whole(150));
    assert_eq!(bay.unequip("suit"), Ok(suit));
}

#[test]
fn a_wearer_without_attributes_still_wears() {
    let mut bay = Bay::new();
    bay.attributes.unset(bay.crew);
    let suit = bay.make("suit", 1);
    assert!(bay.equip(suit).is_ok());
    assert_eq!(bay.unequip("suit"), Ok(suit));
}
