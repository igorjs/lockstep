//! One test per design decision for the spatial crate: the decision, the alternative, and the number
//! that would change it. A failing decision test is a design question: stop and report.

#[path = "../common/mod.rs"]
mod common;

mod atomic_footprints;
mod elevation_as_a_cell_property;
mod flow_fields_for_hordes;
mod heap_key_cell_tie_break;
mod queries_sort_their_output;
mod square8_for_silent_bells;
