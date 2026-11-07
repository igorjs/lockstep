// SPDX-License-Identifier: Apache-2.0
//! Decision 0080: the coverage-guided wrappers live in `fuzz/`, outside the workspace, and the
//! job that runs them is advisory.
//! Alternative rejected: wrappers inside the workspace (every stable build would need nightly)
//! and a required guided job (its runs differ each time).
//! Would change if: the root manifest stops excluding `fuzz`, or the job loses `continue-on-error`
//! (record the first finding the deterministic harness missed before changing either).

use std::path::Path;

fn read(relative: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(relative)).expect("the file exists")
}

#[test]
fn the_wrapper_crate_is_excluded_from_the_workspace() {
    let manifest = read("Cargo.toml");
    assert!(manifest.contains("exclude = [\"fuzz\"]"));
    assert!(read("fuzz/Cargo.toml").contains("[workspace]"));
    assert!(read("fuzz/Cargo.toml").contains("publish = false"));
}

#[test]
fn the_guided_job_is_advisory() {
    let workflow = read(".github/workflows/guided-fuzz.yml");
    assert!(workflow.contains("continue-on-error: true"));
    assert!(!workflow.contains("pull_request"));
}
