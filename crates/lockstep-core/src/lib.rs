// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod chance;
mod clock;
mod hashing;
pub mod math;
mod message;
mod replay;
mod runner;
mod save;
mod simulation;
mod store;
mod streams;
mod timeline;

pub use chance::{
    search_smoothing_increment, smoothed_rate, smoothing_increment, Chance, SmoothedState, CERTAIN,
    SMOOTHING_ENTRIES,
};
pub use clock::{Clock, ClockConfiguration, ClockEvent};
pub use hashing::{decode, encode, hash_of, DECODE_LIMIT};
pub use lockstep_macros::Message;

/// Paths the derive macros use; not part of the public interface.
#[doc(hidden)]
pub mod __private {
    pub use serde::de::DeserializeOwned;
    pub use serde::Serialize;
}
pub use message::{Indexable, Message};
pub use replay::{
    bisect, record_session, recorded_simulation_id, replay, replay_from, Bisection, RecordedStep,
    Recorder, Recording, RecordingSet, ReplayError, ReplayOutcome, Session,
};
pub use runner::{Advanced, Runner, StepConfiguration};
pub use save::{saved_simulation_id, Migrations, Save, SaveError, SAVE_FORMAT};
pub use simulation::{Context, Simulation};
pub use store::{Column, Handle, StableVector};
pub use streams::Streams;
pub use timeline::{Entry, Timeline};
