// SPDX-License-Identifier: Apache-2.0
//! Writes the committed version 1 chronicle and the hash of its conversion. Run once, when the
//! fixture is first made; the committed files are then the record of what shipped, and tests read
//! them, never this program.
//!
//! `cargo run -p roster --example generate_legacy_chronicle -- examples/roster/fixtures`

use lockstep_meta::{chronicle_migrations, open_with, seal, Chronicle};
use roster::{converted_chronicle_hash, legacy_chronicle};

fn main() {
    let directory = std::env::args()
        .nth(1)
        .expect("the directory to write into");
    let bytes = seal(&legacy_chronicle());
    std::fs::write(format!("{directory}/legacy-chronicle-v1.bin"), &bytes).expect("writable");
    let converted: Chronicle = open_with(&bytes, &chronicle_migrations()).expect("converts");
    std::fs::write(
        format!("{directory}/legacy-chronicle-v1.hash"),
        format!("{:016x}\n", converted_chronicle_hash(&converted)),
    )
    .expect("writable");
}
