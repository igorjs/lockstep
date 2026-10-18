// SPDX-License-Identifier: Apache-2.0
//! The bounded fuzz run every pull request makes: the whole corpus, then a fixed number of
//! mutations per target. `just fuzz` runs the same targets for far longer.

use lockstep_fuzz::{fuzz, read_corpus, targets};

fn iterations() -> u64 {
    if cfg!(miri) {
        return 3;
    }
    std::env::var("LOCKSTEP_FUZZ_ITERATIONS")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(1_500)
}

#[test]
fn every_target_survives_its_corpus_and_a_bounded_run_of_mutations() {
    for target in targets() {
        if let Err(failure) = fuzz(&target, 20_261_019, iterations()) {
            panic!("{failure}");
        }
    }
}

#[test]
fn every_target_has_a_committed_corpus() {
    for target in targets() {
        assert!(
            !read_corpus(&target).is_empty(),
            "target {} has no seeds in corpus/{}",
            target.name,
            target.name
        );
    }
}

#[test]
fn a_target_that_panics_is_reported_with_its_input() {
    use lockstep_fuzz::Target;
    let target = Target {
        name: "decode_message",
        run: |bytes| assert!(bytes.len() < 4, "too long"),
    };
    let failure = fuzz(&target, 1, 200).expect_err("some mutation is long enough");
    assert!(failure.message.contains("too long"));
    assert!(failure.to_string().contains("input ("));
}
