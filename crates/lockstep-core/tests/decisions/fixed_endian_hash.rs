// SPDX-License-Identifier: Apache-2.0
//! Decision: the state hash is xxh3 over fixed-width, little-endian bincode bytes.
//! Alternative rejected: hashing the in-memory structs.
//! Would change if: the committed value below differs on any platform (the number to beat is
//! zero differing platforms). The value changes only with a commit naming the encoding change.

use lockstep_core::hash_of;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_known_value_hashes_to_the_committed_number() {
    let value = (1u32, 2.5f32, "lockstep", vec![7u8, 8, 9], Some(-3i64));
    assert_eq!(format!("{:016x}", hash_of(&value)), "ca4a30507e62d7f5");
}

#[test]
fn floats_hash_by_their_exact_bits() {
    assert_ne!(hash_of(&0.0f32), hash_of(&-0.0f32));
    assert_eq!(hash_of(&1.5f32), hash_of(&1.5f32.to_bits()));
}
