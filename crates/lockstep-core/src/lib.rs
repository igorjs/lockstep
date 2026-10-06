//! Lockstep core: a deterministic state machine advanced in fixed steps from intents.
//!
//! The same configuration, seed, and intents produce the same state and the same hash on every
//! platform. The core never renders, never reads input, and never knows what a game is.

mod clock;
mod hashing;
mod message;
mod runner;
mod simulation;
mod store;
mod streams;

pub use clock::{Clock, ClockConfiguration, ClockEvent};
pub use hashing::hash_of;
pub use message::Message;
pub use runner::{Advanced, Runner, StepConfiguration};
pub use simulation::{Context, Simulation};
pub use store::{Column, Handle, StableVector};
pub use streams::Streams;
