// SPDX-License-Identifier: Apache-2.0
//! Every example that can be recorded replays to the hash its fixture prints.

use lockstep_headless::recordings::replayables;
use lockstep_headless::{fixtures, format_hash, run};
use std::path::PathBuf;

/// Fixtures with no single session a recording could capture, each for the reason named.
const NOT_RECORDABLE: [(&str, &str); 2] = [
    ("mars-rovers", "one fixture runs many separate sessions"),
    (
        "orchard",
        "the host sets daylight on the clock between steps",
    ),
];

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
        assert_eq!(
            replayed,
            format!("identical {id} after 1000 steps, hash {expected}"),
            "{id}"
        );
        let bisected = run(&arguments(&["bisect", &path])).unwrap();
        assert_eq!(bisected, format!("no divergence in 1000 steps of {id}"));
        let stats = run(&arguments(&["stats", &path])).unwrap();
        assert!(stats.contains("steps 1000"), "{id}: {stats}");
        std::fs::remove_file(&file).unwrap();
    }
}

#[test]
fn a_recording_taken_with_another_seed_replays_to_that_seed() {
    for replayable in replayables() {
        let id = replayable.id;
        let bytes = (replayable.record)(7, 1000, 100).unwrap();
        let replayed = (replayable.replay)(&bytes, None).unwrap();
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
