// SPDX-License-Identifier: Apache-2.0
use lockstep_meta::Chronicle;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn runs_count_from_one() {
    let mut chronicle = Chronicle::new();
    assert_eq!(chronicle.runs(), 0);
    assert_eq!(chronicle.begin_run(), 1);
    assert_eq!(chronicle.begin_run(), 2);
}

#[test]
fn a_retired_name_is_kept_as_first_recorded() {
    let mut chronicle = Chronicle::new();
    assert!(chronicle.retire("Okafor", 1, 3));
    assert!(!chronicle.retire("Okafor", 2, 9));
    assert_eq!(chronicle.lost().len(), 1);
    assert_eq!(chronicle.lost()[0].run, 1);
    assert_eq!(chronicle.lost()[0].members, 3);
    assert!(chronicle.is_retired("Okafor"));
    assert!(!chronicle.is_retired("Silva"));
}

#[test]
fn retired_names_come_in_the_order_they_were_lost() {
    let mut chronicle = Chronicle::new();
    chronicle.retire("Silva", 1, 1);
    chronicle.retire("Okafor", 2, 1);
    assert_eq!(chronicle.retired_names(), vec!["Silva", "Okafor"]);
}

#[test]
fn tallies_start_at_zero_and_add_up() {
    let mut chronicle = Chronicle::new();
    assert_eq!(chronicle.tally("departures"), 0);
    chronicle.add("departures", 2);
    chronicle.add("departures", -1);
    assert_eq!(chronicle.tally("departures"), 1);
    chronicle.add("departures", i64::MAX);
    assert_eq!(chronicle.tally("departures"), i64::MAX);
}
