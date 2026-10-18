// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod graph;
mod spread;

pub use graph::{Region, RegionId, Regions, RegionsError};
pub use spread::{Breeze, Condition, Spread, SpreadEvent, SpreadRules};
