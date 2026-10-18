// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod layout;
mod templates;

pub use layout::{
    chunk_seed, search_budget, Layout, Reservation, RollError, MAXIMUM_CHUNKS, STREAM,
};
pub use templates::{
    PoolId, SocketId, Template, TemplateId, Templates, TemplatesError, Tile, EAST, MAXIMUM_SIDE,
    MAXIMUM_TOTAL_WEIGHT, NORTH, SOUTH, WEST,
};
