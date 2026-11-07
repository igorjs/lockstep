// SPDX-License-Identifier: Apache-2.0
//! Writes the committed old-version save, the hash of its continuation, and the old recording with
//! the hash it replays to once converted. Run once, when the
//! fixture is first made; the committed files are then the record of what shipped, and tests read
//! them, never this program.
//!
//! `cargo run -p tariffs --example generate_legacy_save -- examples/tariffs/fixtures`

use lockstep_core::{replay, RecordingVersions};
use tariffs::{
    continue_fixture, legacy_recording, legacy_save, migrations, read_recording, Tariffs,
    DEFAULT_SEED, DEFAULT_STEPS, LEGACY_RECORDING_STEPS,
};

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
    let versions = RecordingVersions {
        intent: 1,
        event: 1,
        configuration: 1,
    };
    let bytes = legacy_recording(DEFAULT_SEED, LEGACY_RECORDING_STEPS)
        .to_versioned_bytes(versions)
        .expect("small");
    std::fs::write(format!("{directory}/legacy-recording-v1.bin"), &bytes).expect("writable");
    let converted = read_recording(&bytes).expect("the old recording converts");
    let (runner, _) = replay::<Tariffs>(&converted).expect("the converted recording replays");
    std::fs::write(
        format!("{directory}/legacy-recording-v1.hash"),
        format!("{:016x}\n", runner.hash()),
    )
    .expect("writable");
}
