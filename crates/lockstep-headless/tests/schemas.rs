// SPDX-License-Identifier: Apache-2.0
//! The `schema` command, and the saved schemas every example keeps per message version.

use lockstep_headless::{fixtures, run, schema_file_name, CommandError};
use std::path::{Path, PathBuf};

fn arguments(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

fn examples_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// The messages in a `schema` output, as (role, schema value).
fn messages(name: &str) -> Vec<(String, serde_json::Value)> {
    let out = run(&arguments(&["schema", name])).unwrap();
    let document: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(document["fixture"], name);
    document["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| {
            (
                message["role"].as_str().unwrap().to_string(),
                message["schema"].clone(),
            )
        })
        .collect()
}

#[test]
fn every_fixture_prints_valid_schemas_for_its_four_messages() {
    for fixture in fixtures() {
        let found = messages(fixture.name);
        let roles: Vec<&str> = found.iter().map(|(role, _)| role.as_str()).collect();
        assert_eq!(
            roles,
            ["intent", "event", "snapshot", "configuration"],
            "{}",
            fixture.name
        );
        for (role, schema) in &found {
            assert!(schema["name"].is_string(), "{}: {role}", fixture.name);
            assert!(
                schema["version"].as_u64().unwrap() >= 1,
                "{}: {role}",
                fixture.name
            );
        }
    }
}

/// A shape that changed without a new version, or a new version with no saved schema, fails here
/// for every example that keeps saved schemas (decision 0063).
#[test]
fn every_saved_schema_matches_its_message_at_the_current_version() {
    let mut checked = 0;
    for fixture in fixtures() {
        let directory = examples_directory()
            .join(fixture.name)
            .join("fixtures/schema");
        if !directory.is_dir() {
            continue;
        }
        let out = run(&arguments(&["schema", fixture.name])).unwrap();
        let document: serde_json::Value = serde_json::from_str(&out).unwrap();
        for message in document["messages"].as_array().unwrap() {
            let name = message["schema"]["name"].as_str().unwrap();
            let version = message["schema"]["version"].as_u64().unwrap();
            let file = directory.join(format!("{name}.v{version}.json"));
            let saved = std::fs::read_to_string(&file).unwrap_or_else(|_| {
                panic!(
                    "{}: {name} is at version {version} with no saved schema {}",
                    fixture.name,
                    file.display()
                )
            });
            let current = messages(fixture.name)
                .into_iter()
                .find(|(_, schema)| schema["name"] == name)
                .unwrap()
                .1;
            let saved_value: serde_json::Value = serde_json::from_str(&saved).unwrap();
            assert_eq!(
                saved_value, current,
                "{}: {name} changed shape without a new version",
                fixture.name
            );
            checked += 1;
        }
    }
    assert!(checked >= 8, "at least two examples keep saved schemas");
}

#[test]
fn schema_out_writes_one_file_per_message_with_a_license_sidecar() {
    let directory =
        std::env::temp_dir().join(format!("lockstep-headless-{}-schemas", std::process::id()));
    let path = directory.to_string_lossy().into_owned();
    let out = run(&arguments(&["schema", "bakery", "--out", &path])).unwrap();
    assert_eq!(out.lines().count(), 4);
    let written = std::fs::read_to_string(directory.join("Intent.v1.json")).unwrap();
    let saved =
        std::fs::read_to_string(examples_directory().join("bakery/fixtures/schema/Intent.v1.json"))
            .unwrap();
    assert_eq!(written, saved);
    let sidecar = std::fs::read_to_string(directory.join("Intent.v1.json.license")).unwrap();
    assert_eq!(sidecar, "SPDX-License-Identifier: Apache-2.0\n");
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn schema_refuses_an_unknown_fixture_and_options_it_does_not_use() {
    let error = run(&arguments(&["schema", "nowhere"])).unwrap_err();
    assert!(matches!(error, CommandError::UnknownFixture(_)), "{error}");
    let error = run(&arguments(&["schema", "bakery", "--seed", "1"])).unwrap_err();
    assert!(matches!(error, CommandError::Usage(_)), "{error}");
}

#[test]
fn the_file_name_is_the_type_and_its_version() {
    assert_eq!(
        schema_file_name(r#"{ "name": "Intent", "version": 3 }"#).unwrap(),
        "Intent.v3.json"
    );
    assert!(schema_file_name("{}").is_err());
}
