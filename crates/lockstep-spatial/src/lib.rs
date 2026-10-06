// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod batch;
mod cell;
mod flow;
mod map;
mod occupancy;
mod pathfinder;
mod sight;
mod topology;

pub use batch::{find_paths, find_paths_serially, PathAnswer, PathRequest};
pub use cell::Cell;
pub use flow::FlowField;
pub use map::{GridMap, CHUNK};
pub use occupancy::{Occupancy, Occupied};
pub use pathfinder::{PathOptions, PathResult, Pathfinder};
pub use sight::{line_of_sight, line_of_sight_symmetric};
pub use topology::{Square4, Square8, Topology};

#[cfg(feature = "hex")]
pub use topology::Hex;
