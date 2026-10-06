// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod mind;
mod noise;
mod perception;
mod steering;
mod utility;

pub use mind::{
    think, Alertness, Director, LastKnown, Leash, Mind, MindEvent, MindRules, Stimulus,
    Surroundings,
};
pub use noise::{audible_metres, effective_range, hear, hearing_threshold, Heard, Noise, Wind};
pub use perception::{checks_on, distance_metres, perceive, sees, Cone, Seen, Senses};
pub use steering::{steer, SteerEvent};
pub use utility::{choose, evaluate_task, Choice, Consideration, Reason, Task, TaskResponse};
