// SPDX-License-Identifier: Apache-2.0
//! Decision: a keystone is irreversible: it is never refunded, and the nodes it requires stay
//! locked in because it requires them.
//! Alternative rejected: refunding keystones at a loss, which makes them ordinary nodes.
//! Would change if: chief can be refunded, or senior can be refunded while chief is taken.

use crate::common::Crew;
use lockstep_progression::Refusal;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_keystone_and_what_it_stands_on_never_come_back() {
    let mut crew = Crew::new(20);
    crew.set("experience", 90);
    for node in ["basics", "diagnostics", "senior", "chief"] {
        crew.unlock(node).unwrap();
    }
    assert_eq!(crew.refund("chief"), Err(Refusal::Keystone));
    let chief = crew.node("chief");
    assert_eq!(crew.refund("senior"), Err(Refusal::RequiredBy(chief)));
}
