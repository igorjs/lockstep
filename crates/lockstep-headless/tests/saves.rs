// SPDX-License-Identifier: Apache-2.0
//! The save commands, end to end through files: `replay --save` and `resume`.

use lockstep_headless::{run, CommandError};
use std::path::PathBuf;

/// A file in the system's temporary directory, unique to this test and process, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        Scratch(
            std::env::temp_dir().join(format!("lockstep-headless-{}-{name}", std::process::id())),
        )
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

fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn record(name: &str, fixture: &str, steps: &str) -> Scratch {
    let file = Scratch::new(name);
    run(&arguments(&[
        "record",
        fixture,
        "--out",
        &file.path(),
        "--steps",
        steps,
        "--every",
        "100",
    ]))
    .unwrap();
    file
}

#[test]
fn a_save_taken_part_way_through_a_recording_resumes_it_to_the_same_hash() {
    for fixture in ["ledger", "tariffs"] {
        let recording = record(&format!("{fixture}-rec"), fixture, "1200");
        let save = Scratch::new(&format!("{fixture}-save"));
        let out = run(&arguments(&[
            "replay",
            &recording.path(),
            "--until",
            "450",
            "--save",
            &save.path(),
        ]))
        .unwrap();
        assert!(
            out.contains("identical") && out.contains("saved to "),
            "{out}"
        );
        let resumed = run(&arguments(&[
            "resume",
            &save.path(),
            "--recording",
            &recording.path(),
        ]))
        .unwrap();
        let expected = run(&arguments(&["fixture", fixture, "--steps", "1200"])).unwrap();
        assert_eq!(
            resumed,
            format!("identical {fixture} from step 450 to step 1200, hash {expected}")
        );
    }
}

#[test]
fn resuming_from_a_save_of_another_run_reports_the_divergence() {
    let recording = record("other-rec", "ledger", "600");
    let other = Scratch::new("other-source");
    run(&arguments(&[
        "record",
        "ledger",
        "--out",
        &other.path(),
        "--seed",
        "5",
        "--steps",
        "600",
        "--every",
        "100",
    ]))
    .unwrap();
    let save = Scratch::new("other-save");
    run(&arguments(&[
        "replay",
        &other.path(),
        "--until",
        "300",
        "--save",
        &save.path(),
    ]))
    .unwrap();
    let error = run(&arguments(&[
        "resume",
        &save.path(),
        "--recording",
        &recording.path(),
    ]))
    .unwrap_err();
    assert!(
        matches!(&error, CommandError::Diverged(report) if report.starts_with("diverged ledger at step 300: ")),
        "{error}"
    );
}

#[test]
fn a_save_for_another_simulation_is_refused() {
    let ledger = record("kind-ledger", "ledger", "300");
    let tariffs = record("kind-tariffs", "tariffs", "300");
    let save = Scratch::new("kind-save");
    run(&arguments(&[
        "replay",
        &tariffs.path(),
        "--save",
        &save.path(),
    ]))
    .unwrap();
    let error = run(&arguments(&[
        "resume",
        &save.path(),
        "--recording",
        &ledger.path(),
    ]))
    .unwrap_err();
    assert!(
        matches!(&error, CommandError::Recording(message) if message.contains("tariffs") && message.contains("ledger")),
        "{error}"
    );
}

#[test]
fn the_committed_old_save_is_refused_against_a_current_recording_with_the_reason() {
    let recording = record("legacy-rec", "tariffs", "2000");
    let legacy = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/tariffs/fixtures/legacy-v1.bin"
    );
    let error = run(&arguments(&[
        "resume",
        legacy,
        "--recording",
        &recording.path(),
    ]))
    .unwrap_err();
    assert!(
        matches!(&error, CommandError::Recording(message) if message.contains("snapshot version 1")),
        "{error}"
    );
}

#[test]
fn the_save_options_are_refused_where_they_do_not_apply() {
    let recording = record("options-rec", "ledger", "10");
    for words in [
        vec!["resume", "nowhere"],
        vec!["bisect", recording.path().as_str(), "--save", "x"],
        vec!["resume", "nowhere", "--recording", "x", "--save", "x"],
        vec!["replay", recording.path().as_str(), "--recording", "x"],
    ] {
        let error = run(&arguments(&words)).unwrap_err();
        assert!(
            matches!(error, CommandError::Usage(_)),
            "{words:?}: {error}"
        );
    }
    let missing = run(&arguments(&["resume", "/nowhere/save", "--recording", "x"])).unwrap_err();
    assert!(matches!(missing, CommandError::Unreadable(_)), "{missing}");
}
