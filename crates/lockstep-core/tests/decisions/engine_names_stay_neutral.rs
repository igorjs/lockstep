// SPDX-License-Identifier: Apache-2.0
//! Decision: names in the repository stay neutral engine terms, and a rename that moves a fixture
//! hash is made, not avoided (decision 0071).
//! Alternative rejected: keeping game-flavoured stream names and identifiers because renaming them
//! moves a hash.
//! Would change if: the scan below finds a game-flavoured word in code, data or prose (the number
//! to beat is zero).
#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;

// Written in pieces so this file does not contain the words it scans for.
fn banned() -> Vec<String> {
    [
        ("loo", "t"),
        ("vall", "ey"),
        ("wou", "nd"),
        ("band", "age"),
        ("pr", "ay"),
        ("ma", "ra"),
        ("hu", "sk"),
        ("dunge", "on"),
        ("godo", "t"),
        ("bos", "s"),
    ]
    .iter()
    .map(|(head, tail)| format!("{head}{tail}"))
    .collect()
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .map(|word| word.to_ascii_lowercase())
}

fn scan(path: &Path, banned: &[String], found: &mut Vec<String>) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    if path.is_dir() {
        if name == "target"
            || name == ".git"
            || name == "corpus"
            || name == "decisions" && path.ends_with("docs/decisions")
        {
            return;
        }
        let mut entries: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for entry in entries {
            scan(&entry, banned, found);
        }
    } else if let Ok(text) = fs::read_to_string(path) {
        if name == "engine_names_stay_neutral.rs" {
            return;
        }
        for word in words(&text) {
            if banned.iter().any(|stem| word.starts_with(stem.as_str())) {
                found.push(format!("{}: {word}", path.display()));
            }
        }
    }
}

#[test]
fn no_game_flavoured_word_is_left_in_code_data_or_prose() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let banned = banned();
    let mut found = Vec::new();
    for part in ["crates", "examples", "scripts", "README.md", "docs"] {
        let path = root.join(part);
        if path.exists() {
            scan(&path, &banned, &mut found);
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}
