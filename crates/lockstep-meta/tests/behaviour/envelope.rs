// SPDX-License-Identifier: Apache-2.0
use lockstep_core::Message;
use lockstep_meta::{
    chronicle_migrations, open, open_with, seal, Chronicle, ChronicleV1, ControlEvent,
    EnvelopeError, MAGIC,
};

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

fn old_chronicle() -> ChronicleV1 {
    let mut old = ChronicleV1 {
        runs: 2,
        ..Default::default()
    };
    old.lost.push(lockstep_meta::LineRecord {
        name: "Okafor".to_string(),
        run: 1,
        members: 3,
    });
    old.tallies.insert("departures".to_string(), 4);
    old
}

#[test]
fn a_version_one_chronicle_opens_as_version_two() {
    let bytes = seal(&old_chronicle());
    // The strict reader still refuses it; only the chain converts.
    assert!(matches!(
        open::<Chronicle>(&bytes),
        Err(EnvelopeError::MessageVersion { .. })
    ));
    let opened: Chronicle = open_with(&bytes, &chronicle_migrations()).unwrap();
    assert_eq!(opened.runs(), 2);
    assert!(opened.is_retired("Okafor"));
    assert_eq!(opened.tally("departures"), 4);
    assert_eq!(opened.note("any"), None);
}

#[test]
fn a_note_is_kept_replaced_and_survives_a_save() {
    let mut chronicle = chronicle();
    chronicle.set_note("lesson", "first");
    chronicle.set_note("lesson", "second");
    assert_eq!(chronicle.note("lesson"), Some("second"));
    let opened: Chronicle = open(&seal(&chronicle)).unwrap();
    assert_eq!(opened.note("lesson"), Some("second"));
}

#[test]
fn a_current_chronicle_opens_through_the_chain_unchanged() {
    let bytes = seal(&chronicle());
    assert_eq!(
        open_with::<Chronicle>(&bytes, &chronicle_migrations()),
        Ok(chronicle())
    );
}

#[test]
fn a_newer_or_unknown_version_is_refused_naming_both() {
    let mut bytes = seal(&chronicle());
    for found in [Chronicle::VERSION + 1, 0] {
        bytes[8..12].copy_from_slice(&found.to_le_bytes());
        assert_eq!(
            open_with::<Chronicle>(&bytes, &chronicle_migrations()),
            Err(EnvelopeError::MessageVersion {
                found,
                expected: Chronicle::VERSION
            })
        );
    }
}

#[test]
fn a_damaged_old_chronicle_is_refused_before_conversion() {
    let mut bytes = seal(&old_chronicle());
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(
        open_with::<Chronicle>(&bytes, &chronicle_migrations()),
        Err(EnvelopeError::Corrupt)
    );
}

#[test]
fn a_payload_that_does_not_decode_is_refused() {
    // Sealed as one message, opened as another of the same version: the checksum holds, the shape
    // does not.
    let mut bytes = seal(&chronicle());
    bytes[8..12].copy_from_slice(&ControlEvent::VERSION.to_le_bytes());
    assert!(matches!(
        open::<ControlEvent>(&bytes),
        Err(EnvelopeError::Decode(_))
    ));
}
