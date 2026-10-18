// SPDX-License-Identifier: Apache-2.0
//! Writes the committed old-version save and the hash of its continuation. Run once, when the
//! fixture is first made; the committed files are then the record of what shipped, and tests read
//! them, never this program.
//!
//! `cargo run -p tariffs --example generate_legacy_save -- examples/tariffs/fixtures`

use tariffs::{continue_fixture, legacy_save, migrations, Tariffs, DEFAULT_SEED, DEFAULT_STEPS};

fn main() {
    let directory = std::env::args()
        .nth(1)
        .expect("the directory to write into");
    let save = legacy_save(DEFAULT_SEED);
    std::fs::write(
        format!("{directory}/legacy-v1.bin"),
        save.to_bytes().expect("small"),
    )
    .expect("writable");
    let mut runner = lockstep_core::Runner::<Tariffs>::from_save(&save, &migrations())
        .expect("the legacy save loads");
    continue_fixture(&mut runner, DEFAULT_SEED, DEFAULT_STEPS);
    std::fs::write(
        format!("{directory}/legacy-v1.hash"),
        format!("{:016x}\n", runner.hash()),
    )
    .expect("writable");
}
