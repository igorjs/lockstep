// SPDX-License-Identifier: Apache-2.0
//! Decision: a retired name stays retired for every later run; retiring it again changes
//! nothing, and no call returns a name to use.
//! Alternative rejected: letting a later run clear a retirement, which would make a lost line
//! cheap to forget and its loss meaningless.
//! Would change if: a retired name is not retired after another run begins, or after a save.

use lockstep_meta::{open, seal, Chronicle};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_name_retired_in_one_run_is_retired_in_every_later_run() {
    let mut chronicle = Chronicle::new();
    let first = chronicle.begin_run();
    chronicle.retire("Okafor", first, 3);
    let mut chronicle: Chronicle = open(&seal(&chronicle)).unwrap();
    for _ in 0..3 {
        let run = chronicle.begin_run();
        assert!(!chronicle.retire("Okafor", run, 1));
        assert!(chronicle.is_retired("Okafor"));
    }
    assert_eq!(chronicle.lost().len(), 1);
}
