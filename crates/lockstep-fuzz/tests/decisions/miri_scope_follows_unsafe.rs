// SPDX-License-Identifier: Apache-2.0
//! Decision: no crate contains unsafe code, so Miri runs on the integer-heavy core (the store,
//! `Fixed32`, `Money`) and the fuzz smoke, as an advisory job, rather than on every crate as a
//! required one.
//! Alternative rejected: Miri on the whole workspace as a required check, which takes far longer
//! than the rest of continuous integration put together and finds nothing in code with no unsafe.
//! Would change if: any source file contains the `unsafe` keyword (this test fails and Miri must
//! then cover that crate and become required).

use std::path::Path;

fn sources(directory: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let keyword = ["un", "safe"].concat();
            if text
                .lines()
                .map(str::trim_start)
                .filter(|line| !line.starts_with("//"))
                .any(|line| {
                    line.split(|c: char| !c.is_alphanumeric() && c != '_')
                        .any(|word| word == keyword)
                })
            {
                found.push(path.display().to_string());
            }
        }
    }
}

#[test]
fn no_source_file_uses_unsafe_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for group in ["crates", "examples"] {
        for entry in std::fs::read_dir(root.join(group)).unwrap().flatten() {
            sources(&entry.path().join("src"), &mut found);
        }
    }
    assert!(found.is_empty(), "unsafe code in {found:?}");
}
