// SPDX-License-Identifier: Apache-2.0
//! Decision: a standing is how one entity stands toward another entity or a group, in that
//! direction only; how the other stands back is a standing of its own.
//! Alternative rejected: one shared value per pair, which cannot say that a driver distrusts a
//! dispatcher who trusts the driver.
//! Would change if: raising a's trust in b changes b's trust in a.

use crate::common::{kinds, three, whole};
use lockstep_relations::{Relations, Target};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn trust_one_way_is_not_trust_the_other_way() {
    let kinds = kinds();
    let trust = kinds.relation_id("trust").unwrap();
    let (a, b, _) = three();
    let mut relations = Relations::new();
    relations.change(
        &kinds,
        trust,
        a,
        Target::Entity(b),
        whole(40),
        &mut Vec::new(),
    );
    assert_eq!(
        relations.get(&kinds, trust, a, Target::Entity(b)),
        whole(40)
    );
    assert_eq!(relations.get(&kinds, trust, b, Target::Entity(a)), whole(0));
}
