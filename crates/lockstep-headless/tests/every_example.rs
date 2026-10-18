// SPDX-License-Identifier: Apache-2.0
//! Every example that can be recorded replays to the hash its fixture prints.

use lockstep_headless::recordings::replayables;
use lockstep_headless::{fixtures, format_hash, run};
use std::path::PathBuf;

/// Fixtures with no single session a recording could capture, each for the reason named.
const NOT_RECORDABLE: [(&str, &str); 2] = [
    (
        "ledger-bills",
        "it runs the ledger simulation, which is recorded as ledger",
    ),
    (
        "roster",
        "the second of its two projects starts from a chronicle folded from the first's events",
    ),
];

/// Fixtures that record as a set of sessions, with the sessions in one fixture.
const SETS: [(&str, u64); 1] = [("mars-rovers", 21)];

fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lockstep-headless-{}-every-{name}.recording",
        std::process::id()
    ))
}

#[test]
fn every_fixture_is_recordable_or_names_why_not() {
    for fixture in fixtures() {
        let recordable = replayables()
            .iter()
            .any(|replayable| replayable.id == fixture.name);
        let excused = NOT_RECORDABLE.iter().any(|(name, _)| *name == fixture.name);
        assert!(
            recordable != excused,
            "{} must be recordable or excused, and not both",
            fixture.name
        );
    }
}

#[test]
fn every_recording_replays_to_its_fixture_hash() {
    for replayable in replayables() {
        let id = replayable.id;
        let file = scratch(id);
        let path = file.to_string_lossy().into_owned();
        run(&arguments(&[
            "record", id, "--out", &path, "--steps", "1000", "--every", "100",
        ]))
        .unwrap_or_else(|error| panic!("{id}: {error}"));
        let expected = run(&arguments(&["fixture", id, "--steps", "1000"])).unwrap();
        let replayed =
            run(&arguments(&["replay", &path])).unwrap_or_else(|error| panic!("{id}: {error}"));
        if let Some((_, sessions)) = SETS.iter().find(|(name, _)| *name == id) {
            // Each session lands, programs, then runs the 1000 steps.
            assert_eq!(
                replayed,
                format!("identical {id} after {sessions} sessions of 1002 steps, hash {expected}"),
                "{id}"
            );
            let bisected = run(&arguments(&["bisect", &path])).unwrap();
            assert_eq!(
                bisected,
                format!(
                    "no divergence in {} steps of {sessions} sessions of {id}",
                    sessions * 1002
                )
            );
            let stats = run(&arguments(&["stats", &path])).unwrap();
            assert!(
                stats.contains(&format!("sessions {sessions}")),
                "{id}: {stats}"
            );
        } else {
            assert_eq!(
                replayed,
                format!("identical {id} after 1000 steps, hash {expected}"),
                "{id}"
            );
            let bisected = run(&arguments(&["bisect", &path])).unwrap();
            assert_eq!(bisected, format!("no divergence in 1000 steps of {id}"));
            let stats = run(&arguments(&["stats", &path])).unwrap();
            assert!(stats.contains("steps 1000"), "{id}: {stats}");
        }
        std::fs::remove_file(&file).unwrap();
    }
}

#[test]
fn a_recording_taken_with_another_seed_replays_to_that_seed() {
    for replayable in replayables() {
        let id = replayable.id;
        let bytes = (replayable.record)(7, 1000, 100).unwrap();
        let replayed = (replayable.replay)(&bytes, None, false).unwrap();
        let fixture = fixtures()
            .into_iter()
            .find(|fixture| fixture.name == id)
            .unwrap();
        assert!(replayed.identical, "{id}: {}", replayed.report);
        assert!(
            replayed
                .report
                .ends_with(&format_hash((fixture.run)(7, 1000))),
            "{id}: {}",
            replayed.report
        );
    }
}

/// Long enough to cross two day boundaries, where the orchard's daylight changes.
#[test]
fn an_orchard_recording_replays_across_days_to_its_fixture_hash() {
    let file = scratch("orchard-days");
    let path = file.to_string_lossy().into_owned();
    run(&arguments(&[
        "record", "orchard", "--out", &path, "--steps", "8000", "--every", "1000",
    ]))
    .unwrap();
    let expected = run(&arguments(&["fixture", "orchard", "--steps", "8000"])).unwrap();
    let replayed = run(&arguments(&["replay", &path])).unwrap();
    assert_eq!(
        replayed,
        format!("identical orchard after 8000 steps, hash {expected}")
    );
    std::fs::remove_file(&file).unwrap();
    // The replayed clock holds the daylight of its day, set by the simulation.
    let recording = orchard::record_fixture(orchard::DEFAULT_SEED, 8_000, 1_000, false);
    let (runner, _) = lockstep_core::replay::<orchard::Orchard>(&recording).unwrap();
    let day = runner.clock().day();
    assert!(day >= 2);
    let (sunrise, sunset) = runner.simulation().calendar().daylight(day);
    let configuration = runner.clock().configuration();
    assert_eq!(
        (configuration.sunrise_minute, configuration.sunset_minute),
        (sunrise, sunset)
    );
}

#[test]
fn a_recorded_session_set_names_the_session_that_diverged() {
    let mut set = lockstep_core::RecordingSet::<mars_rovers::Intent>::from_bytes(
        &(replayables()
            .iter()
            .find(|replayable| replayable.id == "mars-rovers")
            .unwrap()
            .record)(1, 50, 10)
        .unwrap(),
    )
    .unwrap();
    set.sessions[9].1.checkpoints[2].1 ^= 1;
    let bytes = set.to_bytes().unwrap();
    let replayable = replayables()
        .into_iter()
        .find(|replayable| replayable.id == "mars-rovers")
        .unwrap();
    let replayed = (replayable.replay)(&bytes, None, false).unwrap();
    assert!(!replayed.identical);
    assert!(
        replayed
            .report
            .starts_with("diverged mars-rovers in session 9 (square8) at step 20: "),
        "{}",
        replayed.report
    );
    let bisected = (replayable.bisect)(&bytes).unwrap();
    assert!(
        bisected
            .report
            .starts_with("session 9 (square8)\nlast good checkpoint: step 10"),
        "{}",
        bisected.report
    );
    assert!((replayable.replay)(&bytes, None, true).is_err());
}
