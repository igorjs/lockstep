<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0080 Coverage-guided fuzzing is a wrapper crate and an advisory job (decided)

Status: decided and implemented. It narrows gap G5 and departs from the trigger in decision 0039, on the owner's request.

## Decision

- `fuzz/` is a libFuzzer wrapper crate, `lockstep-fuzz-guided`, with one binary per target of `crates/lockstep-fuzz`. Each binary is one line that calls the plain function by name, so the deterministic harness and the guided one test the same code.
- `fuzz/` has its own workspace and is listed in the root `exclude`, so `cargo test --workspace` and the stable build never need the nightly compiler. The determinism lint only reads `crates/*/src` and `examples/*/src`, so it never reads `fuzz/`. The crate sets `publish = false`.
- `scripts/check-fuzz-targets.sh`, run by `just check`, fails when the target names in `crates/lockstep-fuzz/src/targets.rs`, the files in `fuzz/fuzz_targets` and the `[[bin]]` entries differ. It runs on the stable compiler.
- `just fuzz-guided <target> <seconds>` seeds `fuzz/corpus` from the committed corpus and runs `cargo +nightly fuzz run`.
- `.github/workflows/guided-fuzz.yml` runs every target for two minutes, weekly and on demand, with `continue-on-error`. It does not run on pull requests, because a guided run differs each time and a pull request could fail on an input nobody can regenerate.

## Why the job is advisory, and when it becomes blocking

The deterministic harness (decision 0039) is the blocking check because its failures reproduce from a seed. A guided run does not. The job turns blocking, and moves to a bounded pull request run with a fixed corpus, when it finds the first bug that the deterministic harness missed: that is the evidence that guidance finds what the harness cannot. Its finding then becomes a regression test and a corpus seed, as decision 0039 requires.

## Alternatives rejected

- Replace the deterministic harness with libFuzzer: loses reproducible pull request runs.
- Put `fuzz/` in the workspace: every stable build would need the nightly compiler and a sanitizer toolchain.
- Run the guided job on every pull request: slow, and a different result each time.

## Would change if

`crates/lockstep-fuzz/tests/decisions/guided_fuzzing_is_advisory_and_outside_the_workspace.rs` fails: the root manifest stops excluding `fuzz`, or the job loses `continue-on-error` without a recorded finding that justifies it.

## Tests

`crates/lockstep-fuzz/tests/decisions/guided_fuzzing_is_advisory_and_outside_the_workspace.rs`, and `scripts/check-fuzz-targets.sh` in `just check`.
