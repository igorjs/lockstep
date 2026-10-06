// SPDX-License-Identifier: Apache-2.0
//! The recording commands, end to end through files.

use lockstep_core::Recording;
use lockstep_headless::{run, CommandError};
use std::path::PathBuf;

/// A file in the system's temporary directory, unique to this test and process, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        Scratch(std::env::temp_dir().join(format!(
            "lockstep-headless-{}-{name}.lkrec",
            std::process::id()
        )))
    }

    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn arguments(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

fn record(name: &str, steps: u64, every: u64) -> Scratch {
    let file = Scratch::new(name);
    let out = run(&arguments(&format!(
        "record ledger --out {} --steps {steps} --every {every}",
        file.path()
    )))
    .unwrap();
    assert!(out.starts_with("recorded ledger to "), "{out}");
    file
}

#[test]
fn a_recorded_ledger_session_replays_identically() {
    let file = record("identical", 1_500, 250);
    let out = run(&arguments(&format!("replay {}", file.path()))).unwrap();
    let expected = run(&arguments("fixture ledger --steps 1500")).unwrap();
    assert_eq!(
        out,
        format!("identical ledger after 1500 steps, hash {expected}")
    );
    let out = run(&arguments(&format!("bisect {}", file.path()))).unwrap();
    assert_eq!(out, "no divergence in 1500 steps of ledger");
}

#[test]
fn stats_report_length_intents_and_multipliers() {
    let file = record("stats", 1_800, 600);
    let out = run(&arguments(&format!("stats {}", file.path()))).unwrap();
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "simulation ledger",
            "seed 20260925",
            "steps 1800 (60.0 real seconds)",
            "intents 1800 (1800.0 per real minute)",
            "clock multipliers: 1 for 1800 steps",
            "checkpoints 4, snapshots 0",
        ]
    );
}

#[test]
fn replay_until_stops_early_and_still_matches() {
    let file = record("until", 1_000, 100);
    let out = run(&arguments(&format!("replay {} --until 450", file.path()))).unwrap();
    let expected = run(&arguments("fixture ledger --steps 450")).unwrap();
    assert_eq!(
        out,
        format!("identical ledger after 450 steps, hash {expected}")
    );
}

#[test]
fn a_tampered_checkpoint_is_reported_and_bisected() {
    let file = record("tampered", 1_000, 100);
    let bytes = std::fs::read(&file.0).unwrap();
    let mut recording = Recording::<ledger::Intent>::from_bytes(&bytes).unwrap();
    let checkpoint = recording
        .checkpoints
        .iter_mut()
        .find(|(step, _)| *step == 700)
        .unwrap();
    checkpoint.1 ^= 1;
    std::fs::write(&file.0, recording.to_bytes().unwrap()).unwrap();
    let error = run(&arguments(&format!("replay {}", file.path()))).unwrap_err();
    assert!(
        matches!(&error, CommandError::Diverged(report) if report.starts_with("diverged ledger at step 700: ")),
        "{error}"
    );
    let out = run(&arguments(&format!("bisect {}", file.path()))).unwrap();
    assert_eq!(
        out.lines().take(3).collect::<Vec<_>>(),
        [
            "last good checkpoint: step 600",
            "first bad checkpoint: step 700",
            "the recording kept no snapshot here, so no difference to show",
        ]
    );
}

#[test]
fn files_that_are_not_recordings_are_refused() {
    let file = Scratch::new("garbage");
    std::fs::write(&file.0, b"not a recording at all").unwrap();
    let error = run(&arguments(&format!("replay {}", file.path()))).unwrap_err();
    assert!(matches!(error, CommandError::Recording(_)), "{error}");
    let missing = run(&arguments("stats /nowhere/at/all.lkrec")).unwrap_err();
    assert!(matches!(missing, CommandError::Unreadable(_)), "{missing}");
    let unknown = run(&arguments("record capsule --out /tmp/never.lkrec")).unwrap_err();
    assert!(matches!(unknown, CommandError::Recording(_)), "{unknown}");
    let no_out = run(&arguments("record ledger")).unwrap_err();
    assert!(matches!(no_out, CommandError::Usage(_)), "{no_out}");
}

#[test]
fn the_simulation_id_is_read_from_the_front_of_the_file() {
    let file = record("identifier", 10, 5);
    let bytes = std::fs::read(&file.0).unwrap();
    assert_eq!(
        lockstep_headless::recordings::simulation_id(&bytes).unwrap(),
        "ledger"
    );
    assert!(lockstep_headless::recordings::simulation_id(&bytes[..5]).is_err());
    let mut lying = bytes.clone();
    lying[..8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(lockstep_headless::recordings::simulation_id(&lying).is_err());
}
