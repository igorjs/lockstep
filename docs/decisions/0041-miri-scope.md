<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0041 Miri runs on the core and the fuzz smoke, and is advisory (decided)

Status: decided and implemented in milestone M16.

## Decision

- No crate contains unsafe code (a test checks every source file). Miri therefore guards against
  undefined behaviour in dependencies and against arithmetic and indexing slips, not against our
  own unsafe code.
- `just miri` runs the fuzz smoke (the corpus and a few mutations per target) and the core's
  store and money tests under Miri. Other crates add nothing Miri could catch that the fuzz
  smoke does not already cover through their loaders.
- The continuous integration job runs on the nightly compiler, takes about two minutes, and is
  marked `continue-on-error`: a failure shows but does not block a merge. The required check
  stays `verify (ubuntu-latest)`.

## Alternatives rejected

- Miri on the whole workspace as a required check: far slower than the rest of the pipeline
  together, for code with no unsafe.
- No Miri job: it is the one tool that would catch a future unsafe block going wrong.

## Would change if

`miri_scope_follows_unsafe.rs` fails (some source file uses unsafe code): Miri must then cover
that crate and become required.
