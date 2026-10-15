// SPDX-License-Identifier: Apache-2.0
use crate::common::three;
use lockstep_meta::{Control, ControlEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn control_falls_back_to_the_first_candidate() {
    let (a, b, c) = three();
    let mut control = Control::new();
    control.assume(a);
    let mut events = Vec::new();
    control.lose(a, &[b, c], &mut events);
    assert_eq!(control.current(), Some(b));
    assert_eq!(events, vec![ControlEvent::Transferred { from: a, to: b }]);
}

#[test]
fn losing_someone_else_changes_nothing() {
    let (a, b, c) = three();
    let mut control = Control::new();
    control.assume(a);
    let mut events = Vec::new();
    control.lose(b, &[c], &mut events);
    assert_eq!(control.current(), Some(a));
    assert!(events.is_empty());
}

#[test]
fn none_left_is_vacant() {
    let (a, _, _) = three();
    let mut control = Control::new();
    control.assume(a);
    let mut events = Vec::new();
    control.lose(a, &[a], &mut events);
    assert_eq!(control.current(), None);
    assert_eq!(events, vec![ControlEvent::Vacant { from: a }]);
}

#[test]
fn nobody_in_control_stays_that_way() {
    let (a, b, _) = three();
    let mut control = Control::new();
    let mut events = Vec::new();
    control.lose(a, &[b], &mut events);
    assert_eq!(control.current(), None);
    assert!(events.is_empty());
}
