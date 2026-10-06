//! A deterministic spatial toolkit: cells as integer indices, a topology trait (square with eight
//! or four neighbours, hexes behind the `hex` feature), dense maps with elevation, a packed
//! occupancy set for bodies, a buffer-reusing A* pathfinder, flow fields for hordes, and integer
//! line of sight.
//!
//! Collision is not detection: it is occupancy. The first caller to claim a cell gets it, so a
//! simulation resolves contested cells in handle order by applying its moves in handle order.
//! Everything takes the topology as a type parameter and only calls the four topology functions.

mod cell;
mod flow;
mod map;
mod occupancy;
mod pathfinder;
mod sight;
mod topology;

pub use cell::Cell;
pub use flow::FlowField;
pub use map::{GridMap, CHUNK};
pub use occupancy::{Occupancy, Occupied};
pub use pathfinder::{PathOptions, PathResult, Pathfinder};
pub use sight::{line_of_sight, line_of_sight_symmetric};
pub use topology::{Square4, Square8, Topology};

#[cfg(feature = "hex")]
pub use topology::Hex;
