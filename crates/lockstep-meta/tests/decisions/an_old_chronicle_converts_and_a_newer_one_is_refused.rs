// SPDX-License-Identifier: Apache-2.0
//! Decision 0084: a saved chronicle of an older version is carried forward by a chain of
//! conversions held next to a frozen copy of each old shape, the same pattern as saves; a version
//! newer than the code, or older with no conversion, is refused naming both versions.
//! Alternative rejected: making `open` convert silently, or refusing every old chronicle and
//! losing the retired names a host kept.
//! Would change if: a newer version opens, a refusal omits either version, or a conversion drops
//! a retired name.

use lockstep_core::Message;
use lockstep_meta::{
    chronicle_migrations, open_with, seal, Chronicle, ChronicleV1, EnvelopeError, LineRecord,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn an_old_chronicle_keeps_its_retired_names_and_a_newer_one_is_refused() {
    let mut old = ChronicleV1::default();
    old.lost.push(LineRecord {
        name: "Okafor".to_string(),
        run: 1,
        members: 3,
    });
    let migrations = chronicle_migrations();
    let opened: Chronicle = open_with(&seal(&old), &migrations).unwrap();
    assert!(opened.is_retired("Okafor"));

    let mut bytes = seal(&opened);
    let newer = Chronicle::VERSION + 1;
    bytes[8..12].copy_from_slice(&newer.to_le_bytes());
    assert_eq!(
        open_with::<Chronicle>(&bytes, &migrations),
        Err(EnvelopeError::MessageVersion {
            found: newer,
            expected: Chronicle::VERSION
        })
    );
}
