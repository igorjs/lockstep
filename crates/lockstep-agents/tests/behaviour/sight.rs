// SPDX-License-Identifier: Apache-2.0
use crate::common::{half_metre, metres, senses, Floor, EAST, WEST};
use lockstep_agents::distance_metres;
use lockstep_agents::{perceive, sees, Seen};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_cone_sees_ahead_to_its_range_and_angle() {
    let floor = Floor::new(60, 30, &[(10, 15)]);
    let from = floor.cell(10, 15);
    let mut line = Vec::new();
    let mut sees_at = |x, y| {
        sees(
            &floor.map,
            from,
            EAST,
            &senses(),
            floor.cell(x, y),
            half_metre(),
            &mut line,
        )
    };
    // 12 metres is 24 half-metre cells.
    assert!(sees_at(34, 15), "12 metres ahead");
    assert!(!sees_at(35, 15), "12.5 metres ahead");
    // 45 degrees either side: (20, 5) is on the edge, (19, 5) just outside.
    assert!(sees_at(20, 5));
    assert!(!sees_at(19, 5));
    // Behind, beyond the 4 metres all around: unseen. Within them: seen.
    assert!(!sees_at(1, 15), "4.5 metres behind");
    assert!(sees_at(2, 15), "4 metres behind");
}

#[test]
fn a_wall_hides_a_target_in_the_cone() {
    let mut floor = Floor::new(40, 20, &[(5, 10)]);
    let wall = floor.cell(10, 10);
    floor.map.set_passable(wall, false);
    let mut line = Vec::new();
    let from = floor.cell(5, 10);
    assert!(!sees(
        &floor.map,
        from,
        EAST,
        &senses(),
        floor.cell(15, 10),
        half_metre(),
        &mut line
    ));
    assert!(sees(
        &floor.map,
        from,
        EAST,
        &senses(),
        floor.cell(15, 12),
        half_metre(),
        &mut line
    ));
}

#[test]
fn perceive_checks_only_the_agents_whose_turn_it_is() {
    // Two watchers facing east at a target ahead of both; the target faces west.
    let floor = Floor::new(40, 20, &[(5, 5), (5, 6), (15, 5)]);
    let (first, second, target) = (floor.bodies[0], floor.bodies[1], floor.bodies[2]);
    let facing = |who| if who == target { WEST } else { EAST };
    let mut seen = Vec::new();
    let mut seen_on = |step| {
        perceive(
            &floor.map,
            &floor.occupancy,
            &floor.senses,
            &facing,
            &[target],
            step,
            2,
            half_metre(),
            &mut seen,
        );
        seen.clone()
    };
    let even = seen_on(0);
    let odd = seen_on(1);
    let at = floor.cell(15, 5);
    // Slots 0 and 2 check on even steps, slot 1 on odd steps. The target sees no one: it is not
    // a target itself.
    assert_eq!(
        even,
        vec![Seen {
            agent: first,
            target,
            at
        }]
    );
    assert_eq!(
        odd,
        vec![Seen {
            agent: second,
            target,
            at
        }]
    );
}

#[test]
fn a_target_across_a_huge_map_is_far_not_near() {
    // 4,000 cells of 10 metres: 40,000 metres, past what 16.16 holds. It saturates, never wraps.
    let map: lockstep_spatial::GridMap<lockstep_spatial::Square8> =
        lockstep_spatial::GridMap::new(4_001, 1);
    let far = distance_metres(&map, map.index(0, 0), map.index(4_000, 0), metres(10));
    assert!(far > metres(30_000));
}
