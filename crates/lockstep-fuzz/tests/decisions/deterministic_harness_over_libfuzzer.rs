// SPDX-License-Identifier: Apache-2.0
//! Decision: fuzzing is a deterministic harness with a seeded generator and a committed corpus,
//! run by `cargo test` on the stable compiler, not cargo-fuzz and libFuzzer.
//! Alternative rejected: cargo-fuzz, which needs the nightly compiler, a sanitizer build and a
//! separate fuzz workspace, and whose runs differ every time, so a pull request could fail on an
//! input nobody can regenerate.
//! Would change if: the harness finds nothing in a long run (`just fuzz`) of a bug that a
//! coverage-guided fuzzer finds within an hour on the same targets, or a failure cannot be
//! reproduced from its seed and iteration.

use lockstep_fuzz::{fuzz, mutate, SplitMix, Target};

#[test]
fn the_same_seed_gives_the_same_mutations() {
    let base = b"{\"attributes\": [1, 2, 3]}";
    let run = || {
        let mut random = SplitMix::new(7);
        (0..50)
            .map(|_| mutate(base, base, &mut random))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_failure_names_the_seed_and_iteration_that_reproduce_it() {
    let target = Target {
        name: "decode_message",
        run: |bytes| assert!(bytes.len() < 6, "too long"),
    };
    let first = fuzz(&target, 11, 500).expect_err("some mutation is long enough");
    let again = fuzz(&target, first.seed, 500).expect_err("the same seed fails again");
    assert_eq!(
        (first.iteration, first.input),
        (again.iteration, again.input)
    );
}
