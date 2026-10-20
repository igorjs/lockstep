// SPDX-License-Identifier: Apache-2.0
use crate::clock::Clock;
use crate::math::Fixed32;
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

    /// The sunrise and sunset minutes the clock uses on `day`, asked before every step. The
    /// default keeps the clock's own, so a simulation whose daylight follows a calendar says so
    /// here and a replay sets it the same way the first run did (decision 0060).
    fn daylight(&self, _day: u32) -> Option<(u32, u32)> {
        None
    }
}

/// Everything a simulation may touch during one step.
pub struct Context<'a, S: Simulation> {
    pub clock: &'a Clock,
    pub elapsed_simulated_minutes: f32,
    /// The same minutes as exact 16.16 fixed point. Use this in simulation state: the minutes of
    /// many steps add up exactly, so nothing that runs on simulated minutes drifts.
    pub elapsed_minutes: Fixed32,
    pub randomness: &'a mut Streams,
    pub events: &'a mut Vec<S::Event>,
    pub step_number: u64,
    pub step_seconds: f32,
}
