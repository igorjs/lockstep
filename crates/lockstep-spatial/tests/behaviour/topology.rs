// SPDX-License-Identifier: Apache-2.0
use lockstep_spatial::{Cell, GridMap, Square4, Square8, Topology};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const WIDTH: u32 = 10;
const HEIGHT: u32 = 8;

fn cell(x: u32, y: u32) -> Cell {
    Cell(y * WIDTH + x)
}

fn neighbours<T: Topology>(at: Cell) -> Vec<Cell> {
    let mut out = vec![Cell(999)];
    T::neighbours(at, WIDTH, HEIGHT, &mut out);
    out
}

#[test]
fn square8_lists_its_neighbours_clockwise_from_north() {
    assert_eq!(
        neighbours::<Square8>(cell(4, 4)),
        vec![
            cell(4, 3),
            cell(5, 3),
            cell(5, 4),
            cell(5, 5),
            cell(4, 5),
            cell(3, 5),
            cell(3, 4),
            cell(3, 3)
        ]
    );
}

#[test]
fn square4_lists_north_east_south_west() {
    assert_eq!(
        neighbours::<Square4>(cell(4, 4)),
        vec![cell(4, 3), cell(5, 4), cell(4, 5), cell(3, 4)]
    );
}

#[test]
fn edges_and_corners_keep_only_neighbours_inside_the_map() {
    assert_eq!(
        neighbours::<Square8>(cell(0, 0)),
        vec![cell(1, 0), cell(1, 1), cell(0, 1)]
    );
    assert_eq!(neighbours::<Square8>(cell(9, 7)).len(), 3);
    assert_eq!(neighbours::<Square8>(cell(0, 4)).len(), 5);
    assert_eq!(
        neighbours::<Square4>(cell(0, 0)),
        vec![cell(1, 0), cell(0, 1)]
    );
    assert_eq!(neighbours::<Square4>(cell(9, 3)).len(), 3);
}

#[test]
fn neighbours_clears_the_output_first() {
    let mut out = vec![Cell(1), Cell(2), Cell(3)];
    Square4::neighbours(cell(0, 0), WIDTH, HEIGHT, &mut out);
    assert_eq!(out.len(), 2);
}

#[test]
fn neighbours_are_symmetric_and_in_the_map() {
    for index in 0..WIDTH * HEIGHT {
        for neighbour in neighbours::<Square8>(Cell(index)) {
            assert!(neighbour.0 < WIDTH * HEIGHT);
            assert!(neighbours::<Square8>(neighbour).contains(&Cell(index)));
        }
    }
}

#[test]
fn distances_are_in_tenths_with_octile_and_manhattan_rules() {
    assert_eq!(Square8::distance(cell(0, 0), cell(0, 0), WIDTH), 0);
    assert_eq!(Square8::distance(cell(0, 0), cell(1, 0), WIDTH), 10);
    assert_eq!(Square8::distance(cell(0, 0), cell(1, 1), WIDTH), 14);
    assert_eq!(
        Square8::distance(cell(0, 0), cell(5, 2), WIDTH),
        58,
        "10 * 5 + 4 * 2"
    );
    assert_eq!(Square8::distance(cell(5, 2), cell(0, 0), WIDTH), 58);
    assert_eq!(Square4::distance(cell(0, 0), cell(5, 2), WIDTH), 70);
    assert_eq!(Square4::distance(cell(5, 2), cell(0, 0), WIDTH), 70);
}

#[test]
fn step_costs_are_ten_straight_and_fourteen_diagonal() {
    assert_eq!(Square8::step_cost(cell(3, 3), cell(4, 3), WIDTH), 10);
    assert_eq!(Square8::step_cost(cell(3, 3), cell(4, 4), WIDTH), 14);
    assert_eq!(Square8::step_cost(cell(3, 3), cell(2, 4), WIDTH), 14);
    assert_eq!(Square4::step_cost(cell(3, 3), cell(3, 4), WIDTH), 10);
}

fn line<T: Topology>(from: Cell, to: Cell) -> Vec<Cell> {
    let mut out = vec![Cell(777)];
    T::line(from, to, WIDTH, &mut out);
    out
}

#[test]
fn a_line_includes_both_ends_and_has_one_cell_per_step_along_the_longer_axis() {
    assert_eq!(line::<Square8>(cell(2, 2), cell(2, 2)), vec![cell(2, 2)]);
    assert_eq!(
        line::<Square8>(cell(0, 0), cell(3, 0)),
        vec![cell(0, 0), cell(1, 0), cell(2, 0), cell(3, 0)]
    );
    assert_eq!(
        line::<Square8>(cell(0, 0), cell(3, 3)),
        vec![cell(0, 0), cell(1, 1), cell(2, 2), cell(3, 3)]
    );
    assert_eq!(
        line::<Square8>(cell(3, 3), cell(0, 0)),
        vec![cell(3, 3), cell(2, 2), cell(1, 1), cell(0, 0)]
    );
    assert_eq!(line::<Square8>(cell(0, 0), cell(6, 2)).len(), 7);
    let long = line::<Square8>(cell(1, 6), cell(8, 1));
    assert_eq!((long[0], *long.last().unwrap()), (cell(1, 6), cell(8, 1)));
    assert_eq!(long.len(), 8);
}

#[test]
fn consecutive_cells_on_a_line_are_neighbours() {
    for (from, to) in [
        (cell(0, 0), cell(9, 7)),
        (cell(9, 0), cell(0, 7)),
        (cell(4, 7), cell(5, 0)),
        (cell(2, 3), cell(2, 3)),
    ] {
        let cells = line::<Square8>(from, to);
        for pair in cells.windows(2) {
            assert_eq!(
                Square8::distance(pair[0], pair[1], WIDTH).min(14),
                Square8::distance(pair[0], pair[1], WIDTH)
            );
            assert!(neighbours::<Square8>(pair[0]).contains(&pair[1]));
        }
    }
}

#[test]
fn a_square4_line_has_no_diagonal_steps() {
    for (from, to) in [
        (cell(0, 0), cell(5, 3)),
        (cell(9, 7), cell(2, 1)),
        (cell(0, 7), cell(9, 0)),
    ] {
        let cells = line::<Square4>(from, to);
        assert_eq!((cells[0], *cells.last().unwrap()), (from, to));
        for pair in cells.windows(2) {
            assert_eq!(Square4::distance(pair[0], pair[1], WIDTH), 10, "{pair:?}");
        }
    }
}

/// Every line between every pair of cells on the test map: the right ends, one neighbour step at a
/// time, and the same cells in both directions.
fn every_line_is_sound<T: Topology>() {
    for from in 0..WIDTH * HEIGHT {
        for to in 0..WIDTH * HEIGHT {
            let (from, to) = (Cell(from), Cell(to));
            let forward = line::<T>(from, to);
            assert_eq!((forward[0], *forward.last().unwrap()), (from, to));
            for pair in forward.windows(2) {
                assert!(
                    neighbours::<T>(pair[0]).contains(&pair[1]),
                    "{from:?} to {to:?}: {pair:?} are not neighbours"
                );
            }
            let mut backward = line::<T>(to, from);
            backward.reverse();
            assert_eq!(
                forward, backward,
                "{from:?} to {to:?} is not the reverse of {to:?} to {from:?}"
            );
        }
    }
}

#[test]
fn every_square8_line_has_the_right_ends_adjacent_steps_and_is_symmetric() {
    every_line_is_sound::<Square8>();
}

#[test]
fn every_square4_line_has_the_right_ends_adjacent_steps_and_is_symmetric() {
    every_line_is_sound::<Square4>();
}

#[cfg(feature = "hex")]
mod hex {
    use super::*;

    // An explicit import wins over the glob above, which would otherwise make `test` ambiguous.
    use lockstep_spatial::Hex;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn a_hex_has_six_neighbours_each_at_distance_one() {
        for (x, y) in [(4, 3), (4, 4), (5, 4)] {
            let around = neighbours::<Hex>(cell(x, y));
            assert_eq!(around.len(), 6);
            for neighbour in around {
                assert_eq!(Hex::distance(cell(x, y), neighbour, WIDTH), 10);
                assert!(neighbours::<Hex>(neighbour).contains(&cell(x, y)));
            }
        }
    }

    #[test]
    fn odd_rows_are_shifted_right_so_neighbours_differ_by_row_parity() {
        // Even row 4: the upper neighbours are columns 3 and 4. Odd row 3: columns 4 and 5.
        assert_eq!(neighbours::<Hex>(cell(4, 4))[0], cell(4, 3));
        assert_eq!(neighbours::<Hex>(cell(4, 3))[0], cell(5, 2));
    }

    #[test]
    fn hex_distance_matches_the_cube_distance() {
        assert_eq!(Hex::distance(cell(0, 0), cell(3, 0), WIDTH), 30);
        assert_eq!(Hex::distance(cell(0, 0), cell(0, 2), WIDTH), 20);
        assert_eq!(Hex::distance(cell(0, 0), cell(4, 4), WIDTH), 60);
        assert_eq!(Hex::distance(cell(4, 4), cell(0, 0), WIDTH), 60);
    }

    #[test]
    fn every_hex_line_stays_inside_the_map_and_steps_between_neighbours() {
        for from in 0..WIDTH * HEIGHT {
            for to in 0..WIDTH * HEIGHT {
                let (from, to) = (Cell(from), Cell(to));
                let cells = line::<Hex>(from, to);
                assert_eq!((cells[0], *cells.last().unwrap()), (from, to));
                assert_eq!(cells.len() as u32, Hex::distance(from, to, WIDTH) / 10 + 1);
                for pair in cells.windows(2) {
                    // A cell that wrapped around the edge would not be a neighbour.
                    assert!(
                        neighbours::<Hex>(pair[0]).contains(&pair[1]),
                        "{from:?} to {to:?}: {pair:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_hex_line_down_an_edge_column_never_wraps_to_the_other_edge() {
        // These two crossed a rounding tie that once picked the hex outside the map.
        assert_eq!(
            line::<Hex>(cell(0, 0), cell(0, 2)),
            vec![cell(0, 0), cell(0, 1), cell(0, 2)]
        );
        let right = line::<Hex>(cell(9, 1), cell(9, 3));
        assert_eq!((right[0], right[2]), (cell(9, 1), cell(9, 3)));
        assert_eq!(right[1], cell(9, 2));
    }

    #[test]
    fn a_hex_line_steps_between_neighbours_and_keeps_its_ends() {
        for (from, to) in [
            (cell(0, 0), cell(9, 7)),
            (cell(9, 0), cell(0, 7)),
            (cell(2, 6), cell(2, 1)),
            (cell(3, 3), cell(3, 3)),
        ] {
            let mut cells = Vec::new();
            Hex::line(from, to, WIDTH, &mut cells);
            assert_eq!((cells[0], *cells.last().unwrap()), (from, to));
            assert_eq!(cells.len() as u32, Hex::distance(from, to, WIDTH) / 10 + 1);
            for pair in cells.windows(2) {
                assert_eq!(Hex::distance(pair[0], pair[1], WIDTH), 10);
            }
        }
    }
}

#[test]
fn every_neighbour_is_one_step_away_and_steps_count_moves_not_cost() {
    fn check<T: lockstep_spatial::Topology>(name: &str) {
        let map: GridMap<T> = GridMap::new(9, 9);
        let centre = map.index(4, 4);
        let mut neighbours = Vec::new();
        map.neighbours(centre, &mut neighbours);
        for neighbour in &neighbours {
            assert_eq!(map.steps(centre, *neighbour), 1, "{name}");
            // Every neighbour's centre is about one cell width away.
            let gap = map.centre(centre).distance(map.centre(*neighbour));
            let one = lockstep_core::math::Fixed32::ONE;
            let diagonal = lockstep_core::math::Fixed32::from_ratio(1_414, 1_000);
            assert!(
                gap >= one - lockstep_core::math::Fixed32::from_ratio(1, 100),
                "{name}"
            );
            assert!(
                gap <= diagonal + lockstep_core::math::Fixed32::from_ratio(1, 100),
                "{name}"
            );
        }
        assert_eq!(map.steps(centre, centre), 0);
    }
    check::<Square8>("Square8");
    check::<Square4>("Square4");
    #[cfg(feature = "hex")]
    check::<lockstep_spatial::Hex>("Hex");
    let map: GridMap<Square8> = GridMap::new(9, 9);
    assert_eq!(
        map.steps(map.index(0, 0), map.index(3, 2)),
        3,
        "a diagonal is one step"
    );
    let map: GridMap<Square4> = GridMap::new(9, 9);
    assert_eq!(map.steps(map.index(0, 0), map.index(3, 2)), 5);
}

#[cfg(feature = "hex")]
#[test]
fn hexagon_centres_are_a_cell_apart_in_every_direction() {
    let map: GridMap<lockstep_spatial::Hex> = GridMap::new(9, 9);
    for centre in [map.index(4, 4), map.index(4, 3)] {
        let mut neighbours = Vec::new();
        map.neighbours(centre, &mut neighbours);
        assert_eq!(neighbours.len(), 6);
        for neighbour in neighbours {
            let gap = map.centre(centre).distance(map.centre(neighbour));
            let error = (gap - lockstep_core::math::Fixed32::ONE).abs();
            assert!(
                error <= lockstep_core::math::Fixed32::from_raw(8),
                "{gap:?}"
            );
        }
    }
}
