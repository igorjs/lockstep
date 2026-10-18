// SPDX-License-Identifier: Apache-2.0
use lockstep_meta::{open, seal, Chronicle, ControlEvent, EnvelopeError, MAGIC};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn chronicle() -> Chronicle {
    let mut chronicle = Chronicle::new();
    chronicle.begin_run();
    chronicle.retire("Okafor", 1, 3);
    chronicle.add("departures", 4);
    chronicle
}

#[test]
fn a_sealed_chronicle_opens_equal() {
    let bytes = seal(&chronicle());
    assert_eq!(&bytes[..4], &MAGIC);
    assert_eq!(open::<Chronicle>(&bytes), Ok(chronicle()));
}

#[test]
fn bytes_that_are_not_an_envelope_are_refused() {
    assert_eq!(
        open::<Chronicle>(b"nope"),
        Err(EnvelopeError::NotAnEnvelope)
    );
    let mut bytes = seal(&chronicle());
    bytes[0] ^= 1;
    assert_eq!(open::<Chronicle>(&bytes), Err(EnvelopeError::NotAnEnvelope));
}

#[test]
fn a_damaged_payload_is_refused() {
    let mut bytes = seal(&chronicle());
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(open::<Chronicle>(&bytes), Err(EnvelopeError::Corrupt));
}

#[test]
fn another_envelope_layout_is_refused() {
    let mut bytes = seal(&chronicle());
    bytes[4] = 9;
    assert_eq!(
        open::<Chronicle>(&bytes),
        Err(EnvelopeError::EnvelopeVersion(9))
    );
}

#[test]
fn a_payload_that_does_not_decode_is_refused() {
    // Sealed as one message, opened as another of the same version: the checksum holds, the shape
    // does not.
    let bytes = seal(&chronicle());
    assert!(matches!(
        open::<ControlEvent>(&bytes),
        Err(EnvelopeError::Decode(_))
    ));
}
