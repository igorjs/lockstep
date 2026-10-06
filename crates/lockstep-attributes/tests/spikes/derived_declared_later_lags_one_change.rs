// SPDX-License-Identifier: Apache-2.0
//! Spike: derived attributes recompute once, in registry order. One that reads a derived attribute
//! declared after it sees that attribute's value from before the change, and catches up on the
//! next change. Declaring inputs first avoids the lag; this documents it rather than hiding it.

use lockstep_attributes::{Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::StableVector;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_derived_input_declared_later_lags_one_change() {
    let registry = Registry::from_json(
        r#"{ "attributes": [
            { "name": "strength", "minimum": 0, "maximum": 100, "starting": 10 },
            { "name": "carry", "minimum": 0, "maximum": 1000, "starting": 0,
              "derived": { "inputs": ["might"], "curve": { "linear": { "per_point": 10, "offset": 0 } } } },
            { "name": "might", "minimum": 0, "maximum": 100, "starting": 0,
              "derived": { "inputs": ["strength"], "curve": { "linear": { "per_point": 1, "offset": 0 } } } }
        ] }"#,
    )
    .unwrap();
    let (strength, carry, might) = (
        registry.id("strength").unwrap(),
        registry.id("carry").unwrap(),
        registry.id("might").unwrap(),
    );
    let who = StableVector::new().insert(());
    let mut attributes = Attributes::from_registry(&registry);
    let mut events = Vec::new();
    // At creation `carry` already reads the stale `might` of 0.
    assert_eq!(attributes.get(might).current(), Fixed32::from_int(10));
    assert_eq!(attributes.get(carry).current(), Fixed32::from_int(0));

    attributes.apply(who, strength, Fixed32::from_int(5), &registry, &mut events);
    assert_eq!(attributes.get(might).current(), Fixed32::from_int(15));
    assert_eq!(
        attributes.get(carry).current(),
        Fixed32::from_int(100),
        "one change behind"
    );

    attributes.apply(who, strength, Fixed32::ZERO, &registry, &mut events);
    assert_eq!(
        attributes.get(carry).current(),
        Fixed32::from_int(150),
        "caught up"
    );
}
