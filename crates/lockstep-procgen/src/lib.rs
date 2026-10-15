// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod layout;
mod templates;

pub use layout::{chunk_seed, Layout, Reservation, RollError, MAXIMUM_CHUNKS, STREAM};
pub use templates::{
    PoolId, Template, TemplateId, Templates, TemplatesError, Tile, MAXIMUM_SIDE,
    MAXIMUM_TOTAL_WEIGHT,
};
