use lockstep_spatial::{line_of_sight, line_of_sight_symmetric, Cell, GridMap, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn sees(map: &GridMap<Square8>, from: Cell, to: Cell, eye: u8) -> bool {
    line_of_sight(map, from, to, eye, &mut Vec::new())
}

#[test]
fn a_cell_sees_itself_and_its_neighbours() {
    let map: GridMap<Square8> = GridMap::new(6, 6);
    let cell = map.index(2, 2);
    assert!(sees(&map, cell, cell, 1));
    assert!(sees(&map, cell, map.index(3, 3), 1));
}

#[test]
fn open_flat_ground_sees_across() {
    let map: GridMap<Square8> = GridMap::new(20, 20);
    assert!(sees(&map, map.index(0, 0), map.index(19, 11), 1));
}

#[test]
fn a_full_wall_blocks_the_view_however_tall_the_viewer() {
    let mut map: GridMap<Square8> = GridMap::new(9, 1);
    map.set_passable(map.index(4, 0), false);
    assert!(!sees(&map, map.index(0, 0), map.index(8, 0), 1));
    assert!(!sees(&map, map.index(0, 0), map.index(8, 0), 200));
    assert!(
        sees(&map, map.index(0, 0), map.index(3, 0), 1),
        "before the wall"
    );
}

#[test]
fn a_wall_between_two_cells_blocks_them_even_when_both_are_high_up() {
    let mut map: GridMap<Square8> = GridMap::new(9, 1);
    for x in 0..9 {
        map.set_elevation(map.index(x, 0), 100);
    }
    map.set_passable(map.index(4, 0), false);
    assert!(
        !sees(&map, map.index(0, 0), map.index(8, 0), 5),
        "a full wall is 255 above its cell"
    );
}

#[test]
fn a_hill_between_blocks_and_a_dip_does_not() {
    let mut map: GridMap<Square8> = GridMap::new(9, 1);
    map.set_elevation(map.index(4, 0), 3);
    assert!(
        !sees(&map, map.index(0, 0), map.index(8, 0), 1),
        "a three step hill against eyes one step up"
    );
    assert!(
        sees(&map, map.index(0, 0), map.index(8, 0), 8),
        "eyes eight steps up see over a three step hill"
    );
    let mut dip: GridMap<Square8> = GridMap::new(9, 1);
    for x in [0, 8] {
        dip.set_elevation(dip.index(x, 0), 2);
    }
    assert!(sees(&dip, dip.index(0, 0), dip.index(8, 0), 1));
}

#[test]
fn the_eye_line_follows_the_elevations_of_both_ends() {
    // A wall of height 2 stands at x = 1 on level ground. From a viewer on a hill at x = 0 the line to a
    // low target passes over it only when the viewer is high enough.
    let mut map: GridMap<Square8> = GridMap::new(5, 1);
    map.set_low_wall(map.index(1, 0), 2);
    assert!(
        !sees(&map, map.index(0, 0), map.index(4, 0), 1),
        "level eyes at one step, wall top at two"
    );
    map.set_elevation(map.index(0, 0), 4);
    assert!(
        sees(&map, map.index(0, 0), map.index(4, 0), 1),
        "eyes five steps up see over a two step wall"
    );
}

#[test]
fn symmetric_sight_needs_both_directions() {
    // The terrace scene: cells 0 to 3 at elevation 3, a wall of height 3 on the ground at cell 4,
    // and eyes two steps up. Cell 3 sees cell 6, but cell 6 does not see cell 3.
    let mut map: GridMap<Square8> = GridMap::new(7, 1);
    for x in 0..4 {
        map.set_elevation(map.index(x, 0), 3);
    }
    map.set_low_wall(map.index(4, 0), 3);
    let (terrace, ground) = (map.index(3, 0), map.index(6, 0));
    let mut line = Vec::new();
    assert!(
        line_of_sight(&map, terrace, ground, 2, &mut line),
        "the terrace sees down"
    );
    assert!(
        !line_of_sight(&map, ground, terrace, 2, &mut line),
        "the ground does not see up"
    );
    assert!(
        !line_of_sight_symmetric(&map, terrace, ground, 2, &mut line),
        "one way is not enough"
    );
    assert!(!line_of_sight_symmetric(
        &map, ground, terrace, 2, &mut line
    ));
    let open: GridMap<Square8> = GridMap::new(7, 7);
    assert!(line_of_sight_symmetric(
        &open,
        open.index(0, 6),
        open.index(5, 1),
        1,
        &mut line
    ));
}

#[test]
fn the_line_buffer_holds_the_cells_from_start_to_end() {
    let map: GridMap<Square8> = GridMap::new(8, 8);
    let mut line = vec![Cell(99)];
    line_of_sight(&map, map.index(0, 0), map.index(5, 0), 1, &mut line);
    assert_eq!(line.len(), 6);
    assert_eq!((line[0], line[5]), (map.index(0, 0), map.index(5, 0)));
}
