use bincode::Options;
use serde::Serialize;

/// Hash of the fixed-endian byte encoding of a value.
///
/// Integers use a fixed width and little-endian order. Floats are encoded by their exact bits.
/// Sequence lengths are encoded as 64-bit integers, so the bytes are identical on every platform.
pub fn hash_of<T: Serialize>(value: &T) -> u64 {
    let bytes = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_little_endian()
        .serialize(value)
        .expect("serialisable");
    xxhash_rust::xxh3::xxh3_64(&bytes)
}
