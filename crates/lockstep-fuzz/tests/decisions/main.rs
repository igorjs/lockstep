// SPDX-License-Identifier: Apache-2.0
//! One test per design decision of the fuzz harness. A failing decision test is a design
//! question: stop and report, never edit the test to pass.

mod deterministic_harness_over_libfuzzer;
mod miri_scope_follows_unsafe;
