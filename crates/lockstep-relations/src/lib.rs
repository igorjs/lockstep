// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod kinds;
mod standings;

pub use kinds::{GroupId, Kinds, KindsError, Relation, RelationId, Threshold};
pub use standings::{RelationEvent, Relations, Standing, Target};
