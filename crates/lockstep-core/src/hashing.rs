// SPDX-License-Identifier: Apache-2.0
use bincode::Options;
use serde::Serialize;

/// Hash of the fixed-endian byte encoding of a value.
///
/// Integers use a fixed width and little-endian order. Floats are encoded by their exact bits.
/// Sequence lengths are encoded as 64-bit integers, so the bytes are identical on every platform.
pub fn hash_of<T: Serialize>(value: &T) -> u64 {
    xxhash_rust::xxh3::xxh3_64(&encode(value))
}

/// The largest encoded value `decode` accepts, so a damaged or hostile file cannot ask for an
/// enormous allocation.
pub const DECODE_LIMIT: u64 = 256 * 1024 * 1024;

/// The same fixed-endian bytes `hash_of` hashes, for recordings and saves.
pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_little_endian()
        .serialize(value)
        .expect("serialisable")
}

/// Reads bytes written by `encode`. Refuses trailing bytes and anything past `DECODE_LIMIT`.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_little_endian()
        .with_limit(DECODE_LIMIT)
        .reject_trailing_bytes()
        .deserialize(bytes)
        .map_err(|error| error.to_string())
}
