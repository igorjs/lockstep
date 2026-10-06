// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod chance;
mod clock;
mod hashing;
pub mod math;
mod message;
mod runner;
mod simulation;
mod store;
mod streams;

pub use chance::{
    search_smoothing_increment, smoothed_rate, smoothing_increment, Chance, SmoothedState, CERTAIN,
    SMOOTHING_ENTRIES,
};
pub use clock::{Clock, ClockConfiguration, ClockEvent};
pub use hashing::hash_of;
pub use message::Message;
pub use runner::{Advanced, Runner, StepConfiguration};
pub use simulation::{Context, Simulation};
pub use store::{Column, Handle, StableVector};
pub use streams::Streams;
