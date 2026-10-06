// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod bisect_finds_a_planted_divergence;
mod chance_in_basis_points;
mod clock_as_a_rate;
mod clock_in_integer_ticks;
mod derived_messages;
mod fixed_endian_hash;
mod fixed_point_math;
mod fixed_step;
mod game_minutes_add_up_exactly;
mod integer_cordic_tables;
mod movement_never_overshoots;
mod named_streams;
mod smoothing_table_committed;
mod stable_vector_over_slotmap_rows;
mod store_reuses_the_lowest_free_slot;
