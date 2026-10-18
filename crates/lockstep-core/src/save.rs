// SPDX-License-Identifier: Apache-2.0
//! Saves: a runner's whole state in a versioned envelope, and the chain of conversions that loads
//! a save written by an older shape of the snapshot.

use crate::clock::Clock;
use crate::hashing::{decode, encode, DECODE_LIMIT};
use crate::message::Message;
use crate::runner::StepConfiguration;
use crate::streams::Streams;
use crate::ClockConfiguration;
use serde::{Deserialize, Serialize};

/// The version of the envelope itself. A save of another format is refused, never guessed at.
pub const SAVE_FORMAT: u32 = 1;

/// A runner at a step boundary: everything `Runner::from_save` needs to continue it. The snapshot
/// travels as bytes with the version of its shape, so the envelope can be read, and the shape
/// converted, before anything knows the simulation's current snapshot type.
///
/// A save holds no partial step: the accumulator of unspent real time and the queued intents are
/// not saved, and neither is in the runner hash. Replays use whole steps, so they never see them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Save {
    /// Always first, so a reader can refuse an envelope it does not know.
    pub format: u32,
    /// Second, so a tool can pick the simulation before decoding the rest.
    pub simulation_id: String,
    pub snapshot_version: u32,
    pub step_configuration: StepConfiguration,
    pub clock_configuration: ClockConfiguration,
    pub step_number: u64,
    pub clock: Clock,
    pub randomness: Streams,
    pub snapshot: Vec<u8>,
}

impl Save {
    /// The save's bytes, in the fixed-endian encoding `hash_of` hashes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        let bytes = encode(self);
        if bytes.len() as u64 > DECODE_LIMIT {
            return Err(SaveError::TooLarge(bytes.len() as u64));
        }
        Ok(bytes)
    }

    /// Refuses trailing bytes, anything past `DECODE_LIMIT`, and a format this code does not know.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        let format = read_format(bytes)?;
        if format != SAVE_FORMAT {
            return Err(SaveError::UnknownFormat(format));
        }
        let save: Save = decode(bytes).map_err(SaveError::NotASave)?;
        save.validate()?;
        Ok(save)
    }

    /// Refuses a save whose clock could not have come from a runner, so a damaged file is an error
    /// here and not a panic later. `Runner::from_save` checks it too.
    pub fn validate(&self) -> Result<(), SaveError> {
        if self.format != SAVE_FORMAT {
            return Err(SaveError::UnknownFormat(self.format));
        }
        // No session reaches 2^63 steps; refusing more leaves room to count on from a loaded save.
        if self.step_number > u64::MAX / 2 {
            return Err(SaveError::NotASave(
                "the step number is out of range".to_string(),
            ));
        }
        if !self.clock.is_consistent() {
            return Err(SaveError::NotASave("the clock is inconsistent".to_string()));
        }
        Ok(())
    }
}

fn read_format(bytes: &[u8]) -> Result<u32, SaveError> {
    bytes
        .get(..4)
        .map(|prefix| u32::from_le_bytes(prefix.try_into().expect("four bytes")))
        .ok_or_else(|| SaveError::NotASave("too short".to_string()))
}

/// The simulation id of a save's bytes, read without knowing the simulation, so a tool can pick
/// which simulation loads the rest. A test pins the layout it relies on: a 32-bit format, then
/// the id as a little-endian 64-bit length and that many bytes of text.
pub fn saved_simulation_id(bytes: &[u8]) -> Result<String, SaveError> {
    let refuse = |reason: &str| SaveError::NotASave(reason.to_string());
    let format = read_format(bytes)?;
    if format != SAVE_FORMAT {
        return Err(SaveError::UnknownFormat(format));
    }
    let length = bytes
        .get(4..12)
        .map(|prefix| u64::from_le_bytes(prefix.try_into().expect("eight bytes")))
        .ok_or_else(|| refuse("too short"))?;
    let text = usize::try_from(length)
        .ok()
        .and_then(|length| bytes.get(12..12usize.checked_add(length)?))
        .ok_or_else(|| refuse("the simulation id runs past the end"))?;
    String::from_utf8(text.to_vec()).map_err(|_| refuse("the simulation id is not text"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    NotASave(String),
    UnknownFormat(u32),
    /// The save belongs to another simulation.
    WrongSimulation {
        expected: String,
        found: String,
    },
    /// The save was written by a newer shape than this code knows.
    Newer {
        found: u32,
        current: u32,
    },
    /// No conversion starts at this version, so the chain to the current one is broken.
    MissingMigration {
        from_version: u32,
    },
    /// A conversion could not read the bytes it was given.
    Migration {
        from_version: u32,
        reason: String,
    },
    /// The converted bytes do not decode as the current snapshot.
    Snapshot(String),
    TooLarge(u64),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NotASave(reason) => write!(formatter, "not a save: {reason}"),
            SaveError::UnknownFormat(format) => write!(
                formatter,
                "the save has envelope format {format}, which this code does not know"
            ),
            SaveError::WrongSimulation { expected, found } => {
                write!(formatter, "the save is for '{found}', not '{expected}'")
            }
            SaveError::Newer { found, current } => write!(
                formatter,
                "the save holds snapshot version {found}, newer than the current {current}"
            ),
            SaveError::MissingMigration { from_version } => write!(
                formatter,
                "no migration starts at snapshot version {from_version}"
            ),
            SaveError::Migration {
                from_version,
                reason,
            } => write!(
                formatter,
                "the migration from snapshot version {from_version} failed: {reason}"
            ),
            SaveError::Snapshot(reason) => {
                write!(formatter, "the snapshot does not decode: {reason}")
            }
            SaveError::TooLarge(bytes) => write!(
                formatter,
                "the save is {bytes} bytes, over the {DECODE_LIMIT} bytes it could be read back with"
            ),
        }
    }
}

impl std::error::Error for SaveError {}

type Conversion = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String>>;

/// The conversions that carry an old snapshot shape forward, one version at a time. A shape that
/// shipped in a save is frozen: its type stays in the code so the bytes can still be read, and a
/// new shape arrives with the conversion from the one before.
///
/// ```
/// use lockstep_core::{Message, Migrations};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
/// #[message(version = 1)]
/// struct Before { total: i64 }
///
/// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
/// #[message(version = 2)]
/// struct After { total: i64, fee_rate: u32 }
///
/// let migrations = Migrations::new().step(|before: Before| After {
///     total: before.total,
///     fee_rate: 0,
/// });
/// let bytes = lockstep_core::encode(&Before { total: 5 });
/// let converted = migrations.migrate::<After>(1, &bytes).unwrap();
/// assert_eq!(converted, After { total: 5, fee_rate: 0 });
/// ```
#[derive(Default)]
pub struct Migrations {
    /// (from version, to version, conversion), one per from version.
    steps: Vec<(u32, u32, Conversion)>,
}

impl Migrations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the conversion from shape `From` to shape `To`, by their versions. Panics when `To`
    /// is not newer than `From` or when a conversion from `From` was already added: both are
    /// mistakes in the code that builds the chain, found the first time it runs.
    pub fn step<From: Message + 'static, To: Message + 'static>(
        mut self,
        convert: fn(From) -> To,
    ) -> Self {
        assert!(
            To::VERSION > From::VERSION,
            "a migration must go to a newer version"
        );
        assert!(
            self.steps.iter().all(|(from, _, _)| *from != From::VERSION),
            "version {} already has a migration",
            From::VERSION
        );
        self.steps.push((
            From::VERSION,
            To::VERSION,
            Box::new(move |bytes| {
                let old: From = decode(bytes)?;
                Ok(encode(&convert(old)))
            }),
        ));
        self
    }

    /// Carries the snapshot bytes written at `from_version` up the chain to `Current` and decodes
    /// them. Bytes already at `Current::VERSION` just decode; a newer version is refused.
    pub fn migrate<Current: Message>(
        &self,
        from_version: u32,
        bytes: &[u8],
    ) -> Result<Current, SaveError> {
        if from_version > Current::VERSION {
            return Err(SaveError::Newer {
                found: from_version,
                current: Current::VERSION,
            });
        }
        let mut version = from_version;
        let mut current = bytes.to_vec();
        while version < Current::VERSION {
            let (_, to, convert) = self
                .steps
                .iter()
                .find(|(from, _, _)| *from == version)
                .ok_or(SaveError::MissingMigration {
                    from_version: version,
                })?;
            current = convert(&current).map_err(|reason| SaveError::Migration {
                from_version: version,
                reason,
            })?;
            version = *to;
        }
        if version != Current::VERSION {
            // A chain that jumped past the current version never lands on it.
            return Err(SaveError::MissingMigration {
                from_version: version,
            });
        }
        decode(&current).map_err(SaveError::Snapshot)
    }
}
