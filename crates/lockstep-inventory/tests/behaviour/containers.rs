// SPDX-License-Identifier: Apache-2.0
use crate::common::{whole, Bay};
use lockstep_core::math::Fixed32;
use lockstep_inventory::{Place, Refusal};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_put_holds_items_in_order_and_take_makes_them_loose() {
    let mut bay = Bay::new();
    let water = bay.stock("water", 2, bay.rack);
    let wrench = bay.stock("wrench", 1, bay.rack);
    assert_eq!(
        bay.inventory.container(bay.rack).unwrap().items(),
        &[water, wrench]
    );
    assert_eq!(bay.inventory.place(water), Some(Place::In(bay.rack)));
    assert_eq!(
        bay.inventory.weight_of(bay.rack, &bay.catalogue),
        Fixed32::from_ratio(7, 2)
    );
    bay.inventory.take(water).unwrap();
    assert_eq!(bay.inventory.place(water), Some(Place::Loose));
    assert_eq!(
        bay.inventory.container(bay.rack).unwrap().items(),
        &[wrench]
    );
    assert_eq!(
        bay.inventory.put(wrench, bay.cold, &bay.catalogue),
        Err(Refusal::NotLoose),
        "only a loose item goes in; a contained one moves"
    );
}

#[test]
fn a_full_or_overweight_container_refuses() {
    let mut bay = Bay::new();
    // The cold store has two slots.
    bay.stock("wrench", 1, bay.cold);
    bay.stock("water", 1, bay.cold);
    let third = bay.make("suit", 1);
    assert_eq!(
        bay.inventory.put(third, bay.cold, &bay.catalogue),
        Err(Refusal::Full)
    );
    // The rack holds 30: two suits are 24, a third would be 36.
    bay.stock("suit", 1, bay.rack);
    bay.stock("suit", 1, bay.rack);
    assert_eq!(
        bay.inventory.put(third, bay.rack, &bay.catalogue),
        Err(Refusal::TooHeavy)
    );
    assert_eq!(bay.inventory.weight_of(bay.rack, &bay.catalogue), whole(24));
}

#[test]
fn units_of_one_kind_stack_into_one_slot() {
    let mut bay = Bay::new();
    let first = bay.stock("ration", 2, bay.cold);
    let more = bay.make("ration", 3);
    assert_eq!(bay.inventory.put(more, bay.cold, &bay.catalogue), Ok(first));
    assert_eq!(bay.inventory.item(first).unwrap().count, 5);
    assert!(
        bay.inventory.item(more).is_none(),
        "the merged item is gone"
    );
    assert_eq!(bay.inventory.container(bay.cold).unwrap().items(), &[first]);
}

#[test]
fn move_between_is_all_or_nothing() {
    let mut bay = Bay::new();
    let suit = bay.stock("suit", 1, bay.rack);
    bay.stock("water", 1, bay.cold);
    bay.stock("wrench", 1, bay.cold);
    assert_eq!(
        bay.inventory.move_between(suit, bay.cold, &bay.catalogue),
        Err(Refusal::Full)
    );
    assert_eq!(
        bay.inventory.place(suit),
        Some(Place::In(bay.rack)),
        "nothing changed"
    );
    assert_eq!(
        bay.inventory.move_between(suit, bay.rack, &bay.catalogue),
        Err(Refusal::AlreadyThere)
    );
    let loose = bay.make("water", 1);
    assert_eq!(
        bay.inventory.move_between(loose, bay.rack, &bay.catalogue),
        Err(Refusal::NotContained)
    );
    let water = bay.inventory.container(bay.cold).unwrap().items()[0];
    assert_eq!(
        bay.inventory.move_between(water, bay.rack, &bay.catalogue),
        Ok(water)
    );
    assert_eq!(
        bay.inventory.container(bay.rack).unwrap().items(),
        &[suit, water]
    );
}

#[test]
fn spoiled_and_fresh_units_never_share_a_stack() {
    let mut bay = Bay::new();
    let old = bay.make("ration", 1);
    bay.inventory
        .spoil(whole(1_440), 18, &bay.catalogue, &mut Vec::new());
    assert!(bay.inventory.item(old).unwrap().spoiled);
    let fresh = bay.stock("ration", 2, bay.rack);
    assert_eq!(bay.inventory.put(old, bay.rack, &bay.catalogue), Ok(old));
    assert!(!bay.inventory.item(fresh).unwrap().spoiled);
    assert_eq!(
        bay.inventory.container(bay.rack).unwrap().items(),
        &[fresh, old]
    );
}

#[test]
fn split_and_consume_count_units() {
    let mut bay = Bay::new();
    let rations = bay.stock("ration", 5, bay.cold);
    let part = bay.inventory.split(rations, 2).unwrap();
    assert_eq!(bay.inventory.item(rations).unwrap().count, 3);
    assert_eq!(bay.inventory.item(part).unwrap().count, 2);
    assert_eq!(bay.inventory.place(part), Some(Place::Loose));
    assert_eq!(
        bay.inventory.split(rations, 3),
        Err(Refusal::Count),
        "nothing would be left"
    );
    assert_eq!(bay.inventory.split(rations, 0), Err(Refusal::Count));
    assert_eq!(bay.inventory.consume(rations, 4), Err(Refusal::Count));
    assert_eq!(bay.inventory.consume(rations, 2), Ok(1));
    assert_eq!(bay.inventory.consume(rations, 1), Ok(0));
    assert!(bay.inventory.item(rations).is_none());
    assert!(bay
        .inventory
        .container(bay.cold)
        .unwrap()
        .items()
        .is_empty());
}

#[test]
fn find_by_tag_lists_matching_items_in_container_order() {
    let mut bay = Bay::new();
    let water = bay.stock("water", 1, bay.rack);
    bay.stock("wrench", 1, bay.rack);
    let ration = bay.stock("ration", 1, bay.rack);
    assert_eq!(
        bay.inventory.find_by_tag(bay.rack, "crew", &bay.catalogue),
        vec![water, ration]
    );
    assert_eq!(
        bay.inventory.find_by_tag(bay.rack, "food", &bay.catalogue),
        vec![ration]
    );
    assert!(bay
        .inventory
        .find_by_tag(bay.cold, "food", &bay.catalogue)
        .is_empty());
}

#[test]
fn an_inventory_saves_and_loads_to_an_equal_one() {
    let mut bay = Bay::new();
    bay.stock("ration", 3, bay.cold);
    let suit = bay.make("suit", 1);
    bay.add_affix(suit, "faulty_seal").unwrap();
    bay.equip(suit).unwrap();
    let bytes = bincode::serialize(&bay.inventory).unwrap();
    let loaded: lockstep_inventory::Inventory = bincode::deserialize(&bytes).unwrap();
    assert_eq!(loaded, bay.inventory);
}
