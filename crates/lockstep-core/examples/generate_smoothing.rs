// SPDX-License-Identifier: Apache-2.0
//! Writes `fixtures/smoothing.bin`: the smoothed-roll increment for every basis point, as
//! little-endian `u32`. Run with `cargo run --release -p lockstep-core --example generate_smoothing`.

use lockstep_core::{search_smoothing_increment, SMOOTHING_ENTRIES};

fn main() {
    let bytes: Vec<u8> = (0..SMOOTHING_ENTRIES as u32)
        .flat_map(|points| search_smoothing_increment(points).to_le_bytes())
        .collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/smoothing.bin");
    std::fs::write(path, bytes).expect("write the table");
    println!("wrote {SMOOTHING_ENTRIES} entries to {path}");
}
