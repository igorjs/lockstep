// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod table;

pub use table::{
    Swapped, Table, TableError, TableRow, TableSwap, MAXIMUM_ROWS, MAXIMUM_SWAP_BYTES,
};
