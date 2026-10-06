// SPDX-License-Identifier: Apache-2.0
use crate::clock::Clock;
use crate::message::Message;
use crate::streams::Streams;

/// A deterministic state machine advanced in fixed steps.
/// Same configuration, seed, and intents produce the same state on every platform.
pub trait Simulation: Sized {
    type Intent: Message;
    type Event: Message;
    type Snapshot: Message;
    type Configuration: Message;

    fn create(configuration: Self::Configuration, randomness: &mut Streams) -> Self;
    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Self::Intent]);
    fn snapshot(&self) -> Self::Snapshot;
    fn restore(snapshot: Self::Snapshot) -> Self;
}

/// Everything a simulation may touch during one step.
pub struct Context<'a, S: Simulation> {
    pub clock: &'a Clock,
    pub elapsed_game_minutes: f32,
    pub randomness: &'a mut Streams,
    pub events: &'a mut Vec<S::Event>,
    pub step_number: u64,
    pub step_seconds: f32,
}
