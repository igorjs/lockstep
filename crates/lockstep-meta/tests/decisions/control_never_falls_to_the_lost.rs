// SPDX-License-Identifier: Apache-2.0
//! Decision: when the one in control is lost, control falls back to the first candidate the
//! caller lists, in the caller's order, and never to the one just lost; with no candidate it is
//! vacant and says so, rather than staying with the lost.
//! Alternative rejected: the library choosing the heir (by age or handle), which would hide a
//! rule the game or business owns.
//! Would change if: a transfer names the lost entity, or skips an earlier candidate.

use crate::common::three;
use lockstep_meta::{Control, ControlEvent};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_callers_order_decides_and_the_lost_is_skipped() {
    let (a, b, c) = three();
    let mut control = Control::new();
    control.assume(a);
    let mut events = Vec::new();
    control.lose(a, &[a, c, b], &mut events);
    assert_eq!(events, vec![ControlEvent::Transferred { from: a, to: c }]);
    control.lose(c, &[], &mut events);
    assert_eq!(control.current(), None);
    assert_eq!(events[1], ControlEvent::Vacant { from: c });
}
