// SPDX-License-Identifier: Apache-2.0
//! Decision: the cross-run record is not part of any simulation's state; it is saved on its own
//! in an envelope carrying its message version, and a save written for another version is refused
//! rather than guessed at.
//! Alternative rejected: keeping the record inside the simulation snapshot, where a new run would
//! start from nothing, or saving bare bytes, where a changed shape reads as garbage.
//! Would change if: a save of another version opens, or a damaged save opens.

use lockstep_core::Message;
use lockstep_meta::{open, seal, Chronicle, EnvelopeError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_envelope_carries_the_message_version_and_refuses_another() {
    let mut chronicle = Chronicle::new();
    chronicle.begin_run();
    let mut bytes = seal(&chronicle);
    assert_eq!(bytes[8..12], Chronicle::VERSION.to_le_bytes());
    bytes[8..12].copy_from_slice(&(Chronicle::VERSION + 1).to_le_bytes());
    assert_eq!(
        open::<Chronicle>(&bytes),
        Err(EnvelopeError::MessageVersion {
            found: Chronicle::VERSION + 1,
            expected: Chronicle::VERSION
        })
    );
}
