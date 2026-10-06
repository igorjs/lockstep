// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod noise;
mod perception;

pub use noise::{audible_metres, effective_range, hear, hearing_threshold, Heard, Noise, Wind};
pub use perception::{checks_on, distance_metres, perceive, sees, Cone, Seen, Senses};
