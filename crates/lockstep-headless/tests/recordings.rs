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
            "lockstep-headless-{}-{name}.recording",
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

/// Arguments as the shell would pass them; a path keeps its spaces.
fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn record(name: &str, steps: u64, every: u64) -> Scratch {
    let file = Scratch::new(name);
    let (steps, every) = (steps.to_string(), every.to_string());
    let out = run(&arguments(&[
        "record",
        "ledger",
        "--out",
        &file.path(),
        "--steps",
        &steps,
        "--every",
        &every,
    ]))
    .unwrap();
    assert!(out.starts_with("recorded ledger to "), "{out}");
    file
}

#[test]
fn a_recorded_ledger_session_replays_identically() {
    let file = record("identical", 1_500, 250);
    let out = run(&arguments(&["replay", &file.path()])).unwrap();
    let expected = run(&arguments(&["fixture", "ledger", "--steps", "1500"])).unwrap();
    assert_eq!(
        out,
        format!("identical ledger after 1500 steps, hash {expected}")
    );
    let out = run(&arguments(&["bisect", &file.path()])).unwrap();
    assert_eq!(out, "no divergence in 1500 steps of ledger");
}

#[test]
fn stats_report_length_intents_and_multipliers() {
    let file = record("stats", 1_800, 600);
    let out = run(&arguments(&["stats", &file.path()])).unwrap();
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        [
            "simulation ledger",
            "seed 20260925",
            "steps 1800 (60.0 real seconds)",
            "intents 1800 (1800.0 per real minute)",
            "clock multipliers: 1 for 1800 steps",
            "checkpoints 4, snapshots 4",
        ]
    );
}

#[test]
fn replay_until_stops_early_and_still_matches() {
    let file = record("until", 1_000, 100);
    let out = run(&arguments(&["replay", &file.path(), "--until", "450"])).unwrap();
    let expected = run(&arguments(&["fixture", "ledger", "--steps", "450"])).unwrap();
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
    let error = run(&arguments(&["replay", &file.path()])).unwrap_err();
    assert!(
        matches!(&error, CommandError::Diverged(report) if report.starts_with("diverged ledger at step 700: ")),
        "{error}"
    );
    let error = run(&arguments(&["bisect", &file.path()])).unwrap_err();
    let CommandError::Diverged(report) = error else {
        panic!("bisect reports a divergence as an error: {error}");
    };
    assert_eq!(
        report.lines().collect::<Vec<_>>(),
        [
            "last good checkpoint: step 600",
            "first bad checkpoint: step 700",
            "the kept snapshot matches, so only the recorded hash differs",
        ]
    );
    // A replay stopped exactly on the tampered checkpoint still compares it.
    let error = run(&arguments(&["replay", &file.path(), "--until", "700"])).unwrap_err();
    assert!(matches!(&error, CommandError::Diverged(_)), "{error}");
    let fine = run(&arguments(&["replay", &file.path(), "--until", "699"])).unwrap();
    assert!(
        fine.starts_with("identical ledger after 699 steps"),
        "{fine}"
    );
}

#[test]
fn files_that_are_not_recordings_are_refused() {
    let file = Scratch::new("garbage");
    std::fs::write(&file.0, b"not a recording at all").unwrap();
    let error = run(&arguments(&["replay", &file.path()])).unwrap_err();
    assert!(matches!(error, CommandError::Recording(_)), "{error}");
    let missing = run(&arguments(&["stats", "/nowhere/at/all.recording"])).unwrap_err();
    assert!(matches!(missing, CommandError::Unreadable(_)), "{missing}");
    let unknown = run(&arguments(&[
        "record",
        "capsule",
        "--out",
        "/nowhere/never.recording",
    ]))
    .unwrap_err();
    assert!(matches!(unknown, CommandError::Recording(_)), "{unknown}");
    let no_out = run(&arguments(&["record", "ledger"])).unwrap_err();
    assert!(matches!(no_out, CommandError::Usage(_)), "{no_out}");
}

#[test]
fn each_command_refuses_options_it_does_not_use() {
    let file = record("options", 10, 5);
    for words in [
        vec!["bisect", file.path().as_str(), "--until", "5"],
        vec!["stats", file.path().as_str(), "--until", "5"],
        vec!["replay", file.path().as_str(), "--seed", "1"],
        vec!["fixture", "ledger", "--out", "/nowhere"],
    ] {
        let error = run(&arguments(&words)).unwrap_err();
        assert!(
            matches!(error, CommandError::Usage(_)),
            "{words:?}: {error}"
        );
    }
}

#[test]
fn a_write_that_fails_is_reported_as_unwritable() {
    let error = run(&arguments(&[
        "record",
        "ledger",
        "--out",
        "/nowhere/at/all.recording",
    ]))
    .unwrap_err();
    assert!(matches!(error, CommandError::Unwritable(_)), "{error}");
}
