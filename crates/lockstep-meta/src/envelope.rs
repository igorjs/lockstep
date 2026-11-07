// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{decode, encode, hash_of, Message, Migrations, SaveError};

/// The first four bytes of every envelope.
pub const MAGIC: [u8; 4] = *b"LKMT";
/// The version of the envelope layout itself.
pub const ENVELOPE_VERSION: u32 = 1;

const HEADER: usize = 4 + 4 + 4 + 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Shorter than the header, or not starting with the magic.
    NotAnEnvelope,
    /// Written by another layout of the envelope.
    EnvelopeVersion(u32),
    /// Written for another version of the message.
    MessageVersion { found: u32, expected: u32 },
    /// The payload does not match its checksum.
    Corrupt,
    /// The payload does not decode as the message.
    Decode(String),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EnvelopeError {}

/// Wraps a message for saving: the magic, the envelope version, the message version, a checksum of
/// the payload, and the payload (the fixed-endian encoding the hash covers).
pub fn seal<T: Message>(message: &T) -> Vec<u8> {
    let payload = encode(message);
    let mut bytes = Vec::with_capacity(HEADER + payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&ENVELOPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&T::VERSION.to_le_bytes());
    bytes.extend_from_slice(&checksum(&payload).to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes
}

/// Reads a message sealed by `seal`, refusing anything damaged or written for another version.
pub fn open<T: Message>(bytes: &[u8]) -> Result<T, EnvelopeError> {
    if bytes.len() < HEADER || bytes[..4] != MAGIC {
        return Err(EnvelopeError::NotAnEnvelope);
    }
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    if word(4) != ENVELOPE_VERSION {
        return Err(EnvelopeError::EnvelopeVersion(word(4)));
    }
    if word(8) != T::VERSION {
        return Err(EnvelopeError::MessageVersion {
            found: word(8),
            expected: T::VERSION,
        });
    }
    decode(checked_payload(bytes)?).map_err(EnvelopeError::Decode)
}

/// Like `open`, but a message sealed at an older version is carried up the chain of conversions
/// to `T`. A newer version, and an older one no conversion starts at, are refused with
/// `MessageVersion` naming both versions. The checksum is checked before any conversion runs.
pub fn open_with<T: Message>(bytes: &[u8], migrations: &Migrations) -> Result<T, EnvelopeError> {
    if bytes.len() < HEADER || bytes[..4] != MAGIC {
        return Err(EnvelopeError::NotAnEnvelope);
    }
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"));
    if word(4) != ENVELOPE_VERSION {
        return Err(EnvelopeError::EnvelopeVersion(word(4)));
    }
    let found = word(8);
    let payload = checked_payload(bytes)?;
    migrations
        .migrate::<T>(found, payload)
        .map_err(|error| match error {
            SaveError::Newer { .. } | SaveError::MissingMigration { .. } => {
                EnvelopeError::MessageVersion {
                    found,
                    expected: T::VERSION,
                }
            }
            other => EnvelopeError::Decode(other.to_string()),
        })
}

fn checked_payload(bytes: &[u8]) -> Result<&[u8], EnvelopeError> {
    let expected = u64::from_le_bytes(bytes[12..20].try_into().expect("eight bytes"));
    let payload = &bytes[HEADER..];
    if checksum(payload) != expected {
        return Err(EnvelopeError::Corrupt);
    }
    Ok(payload)
}

fn checksum(payload: &[u8]) -> u64 {
    hash_of(&payload.to_vec())
}
