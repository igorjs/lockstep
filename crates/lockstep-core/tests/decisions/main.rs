//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod clock_as_a_rate;
mod clock_in_integer_ticks;
mod fixed_endian_hash;
mod fixed_step;
mod named_streams;
mod stable_vector_over_slotmap_rows;
mod store_reuses_the_lowest_free_slot;
