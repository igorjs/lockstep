<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0039 Fuzzing is a deterministic harness, not libFuzzer (decided)

Status: decided and implemented in milestone M16.

## Decision

- The fuzz targets live in `crates/lockstep-fuzz` and run under `cargo test` on the stable
  compiler. Each target takes bytes and must neither panic nor break an invariant.
- Inputs are the committed seeds in `crates/lockstep-fuzz/corpus/<target>/` plus mutations drawn
  from a seeded generator (SplitMix64). A run is the same on every machine, and a failure prints
  the target, seed, iteration and the exact bytes.
- Every pull request runs the corpus and 1,500 mutations per target as part of `just test`.
  `just fuzz` runs a long run (a million per target by default; count and seed are arguments).
- The harness is built with the `fuzz` profile: release speed with overflow checks on and
  `panic = "unwind"`, because the release profile aborts on a panic and so cannot report one.
- The targets: decoding a recording, a save and each message; the JSON loaders of the attributes,
  inventory, crafting, relations, procgen, knowledge, progression, regions and calendar crates;
  `Fixed32` and `Money` arithmetic invariants.
- A bug the harness finds becomes a regression test in `crates/lockstep-fuzz/tests/regressions.rs`
  (or the crate's own tests) and a seed in the corpus.

## Alternatives rejected

- cargo-fuzz with libFuzzer: needs the nightly compiler and a sanitizer build, lives in a separate
  workspace, and its runs differ each time, so a pull request could fail on an input nobody can
  regenerate. Coverage guidance is its one advantage; a long `just fuzz` run and good seeds stand
  in for it, and the targets are plain functions that a libFuzzer wrapper could call later.
- A property-testing crate: one more dependency for what a few dozen lines of generator do.

## Would change if

Either of these fails: `deterministic_harness_over_libfuzzer.rs` (the same seed gives the same
mutations, and a failure reproduces from its seed), or a long run misses a bug that a
coverage-guided fuzzer finds within an hour on the same targets.
