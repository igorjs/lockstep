//! A deterministic spatial toolkit: cells as integer indices, a topology trait (square with eight
//! or four neighbours, hexes behind the `hex` feature), dense maps with elevation, and a packed
//! occupancy set for bodies.
//!
//! Collision is not detection: it is occupancy, resolved in handle order. Everything takes the
//! topology as a type parameter and only calls the four topology functions.

mod cell;
mod map;
mod occupancy;
mod topology;

pub use cell::Cell;
pub use map::{GridMap, CHUNK};
pub use occupancy::{Occupancy, Occupied};
pub use topology::{Square4, Square8, Topology};

#[cfg(feature = "hex")]
pub use topology::Hex;
