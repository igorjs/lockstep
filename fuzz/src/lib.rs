// SPDX-License-Identifier: Apache-2.0
//! Calls a `lockstep-fuzz` target by name, so each libFuzzer binary stays one line.

use std::sync::OnceLock;

pub fn run(name: &str, data: &[u8]) {
    static TARGETS: OnceLock<Vec<lockstep_fuzz::Target>> = OnceLock::new();
    let target = TARGETS
        .get_or_init(lockstep_fuzz::targets)
        .iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| panic!("no fuzz target named {name}"));
    (target.run)(data);
}
