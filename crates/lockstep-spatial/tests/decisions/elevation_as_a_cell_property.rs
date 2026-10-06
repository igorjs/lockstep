//! Decision: elevation is a property of a cell, not a third axis, and line of sight compares the
//! top of each cell between the viewer's eye and the floor of the target with the line joining them.
//! Alternative rejected: a full three-dimensional grid, which costs a whole extra axis of memory and
//! of search for what a terrace and a garden wall need.
//! Would change if: a terrace fails to see over a low wall at its edge, a viewer on the ground sees up
//! over it, or a high wall lets anyone see through (the numbers to beat are zero wrong answers in the
//! scenes below).
//!
//! The scene, along one row of cells (eyes are two steps up):
//!
//! ```text
//!   cell:        0  1  2  3 | 4 | 5  6
//!   ground:      3  3  3  3 | 0 | 0  0      the terrace is cells 0 to 3, elevation 3
//!   garden wall:             wall of height 3 at cell 4, on the ground at the foot of the terrace
//! ```
//!
//! From the terrace edge (cell 3) the eye is five steps up and the line to the floor of cell 6 is
//! four steps high at the wall, above its top of three. From the ground at cell 6 the line to the
//! terrace floor is only 2.7 steps high at the wall, under its top.

use lockstep_spatial::{line_of_sight, GridMap, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn scene(wall_height: u8) -> GridMap<Square8> {
    let mut map: GridMap<Square8> = GridMap::new(7, 1);
    for x in 0..4 {
        map.set_elevation(map.index(x, 0), 3);
    }
    map.set_low_wall(map.index(4, 0), wall_height);
    map
}

fn sees(map: &GridMap<Square8>, from: u32, to: u32) -> bool {
    line_of_sight(
        map,
        map.index(from, 0),
        map.index(to, 0),
        2,
        &mut Vec::new(),
    )
}

#[test]
fn a_terrace_sees_over_a_low_wall_at_its_foot_down_to_the_ground() {
    assert!(sees(&scene(3), 3, 6));
}

#[test]
fn the_ground_does_not_see_up_over_the_same_wall_to_the_terrace() {
    assert!(!sees(&scene(3), 6, 3));
}

#[test]
fn a_high_wall_blocks_both_and_no_wall_opens_both() {
    let high = scene(255);
    assert!(!sees(&high, 3, 6));
    assert!(!sees(&high, 6, 3));
    let none = scene(0);
    assert!(sees(&none, 3, 6));
    assert!(sees(&none, 6, 3));
}
