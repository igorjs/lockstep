// SPDX-License-Identifier: Apache-2.0
//! Calls a `lockstep-fuzz` target by name, so each libFuzzer binary stays one line.

pub fn run(name: &str, data: &[u8]) {
    let target = lockstep_fuzz::targets()
        .into_iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| panic!("no fuzz target named {name}"));
    (target.run)(data);
}
