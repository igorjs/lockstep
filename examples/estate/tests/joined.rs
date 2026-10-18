// SPDX-License-Identifier: Apache-2.0
//! Paths and fences that join across chunk borders, checked by walking them.

use estate::joined_templates;
use lockstep_procgen::{Layout, Reservation, RollError, Templates, EAST, NORTH, SOUTH, WEST};
use lockstep_spatial::{GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Square4};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const WIDTH: u32 = 8;
const HEIGHT: u32 = 6;

fn built(templates: &Templates, layout: &Layout) -> GridMap<Square4> {
    let side = templates.side();
    let mut map = GridMap::new(layout.width() * side, layout.height() * side);
    layout.apply(templates, &mut map).unwrap();
    map
}

fn reachable(map: &GridMap<Square4>, from: (u32, u32), to: (u32, u32)) -> bool {
    let mut path = Vec::new();
    let result = Pathfinder::new(map).find(
        map,
        &Occupancy::new(map),
        map.index(from.0, from.1),
        map.index(to.0, to.1),
        PathOptions::default(),
        &mut path,
    );
    matches!(result, PathResult::Found { .. })
}

fn is_gate(templates: &Templates, layout: &Layout, x: u32, y: u32, side: usize) -> bool {
    let template = templates
        .template(layout.template_at(x, y).unwrap())
        .unwrap();
    templates.socket_name(template.socket(side).unwrap()) == Some("gate")
}

/// Chunks that share a gate, found by flooding over the sockets alone.
fn components(templates: &Templates, layout: &Layout) -> Vec<Option<usize>> {
    let (width, height) = (layout.width(), layout.height());
    let gated = |x, y| (0..4).any(|side| is_gate(templates, layout, x, y, side));
    let mut label: Vec<Option<usize>> = vec![None; (width * height) as usize];
    let mut next = 0;
    for start in 0..width * height {
        let (start_x, start_y) = (start % width, start / width);
        if label[start as usize].is_some() || !gated(start_x, start_y) {
            continue;
        }
        let mut stack = vec![(start_x, start_y)];
        label[start as usize] = Some(next);
        while let Some((x, y)) = stack.pop() {
            let mut steps = Vec::new();
            if is_gate(templates, layout, x, y, EAST) && x + 1 < width {
                steps.push((x + 1, y));
            }
            if is_gate(templates, layout, x, y, SOUTH) && y + 1 < height {
                steps.push((x, y + 1));
            }
            if is_gate(templates, layout, x, y, WEST) && x > 0 {
                steps.push((x - 1, y));
            }
            if is_gate(templates, layout, x, y, NORTH) && y > 0 {
                steps.push((x, y - 1));
            }
            for (next_x, next_y) in steps {
                let slot = &mut label[(next_y * width + next_x) as usize];
                if slot.is_none() {
                    *slot = Some(next);
                    stack.push((next_x, next_y));
                }
            }
        }
        next += 1;
    }
    label
}

/// The gate: for many seeds every seam is open on both sides or walled on both sides, and the
/// walker reaches exactly the chunks of the same path network, never one of another.
#[test]
fn paths_and_fences_join_across_every_chunk_border() {
    let templates = joined_templates();
    let pool = templates.pool_id("paths").unwrap();
    let side = templates.side();
    let mut networks_seen = 0;
    for seed in 0..6u64 {
        let layout = templates
            .roll_matched(seed, WIDTH, HEIGHT, pool, &[])
            .unwrap();
        let map = built(&templates, &layout);
        for y in 0..map.height() {
            for x in 0..map.width() {
                let here = map.is_passable(map.index(x, y));
                if x % side == side - 1 && x + 1 < map.width() {
                    assert_eq!(
                        here,
                        map.is_passable(map.index(x + 1, y)),
                        "seed {seed} seam at ({x}, {y})"
                    );
                }
                if y % side == side - 1 && y + 1 < map.height() {
                    assert_eq!(
                        here,
                        map.is_passable(map.index(x, y + 1)),
                        "seed {seed} seam at ({x}, {y})"
                    );
                }
            }
        }
        let label = components(&templates, &layout);
        let centre = |index: u32| ((index % WIDTH) * side + 3, (index / WIDTH) * side + 3);
        let gated: Vec<u32> = (0..WIDTH * HEIGHT)
            .filter(|index| label[*index as usize].is_some())
            .collect();
        networks_seen += label.iter().flatten().max().map_or(0, |top| top + 1);
        for &a in &gated {
            for &b in &gated {
                assert_eq!(
                    reachable(&map, centre(a), centre(b)),
                    label[a as usize] == label[b as usize],
                    "seed {seed} chunks {a} and {b}"
                );
            }
        }
    }
    // The check bites only if some layouts hold more than one network.
    assert!(networks_seen > 6, "{networks_seen} networks over 6 seeds");
}

#[test]
fn a_matched_roll_is_repeatable_and_follows_the_seed() {
    let templates = joined_templates();
    let pool = templates.pool_id("paths").unwrap();
    let first = templates.roll_matched(9, WIDTH, HEIGHT, pool, &[]).unwrap();
    assert_eq!(
        first,
        templates.roll_matched(9, WIDTH, HEIGHT, pool, &[]).unwrap()
    );
    assert_ne!(
        first,
        templates
            .roll_matched(10, WIDTH, HEIGHT, pool, &[])
            .unwrap()
    );
}

#[test]
fn reserved_chunks_are_kept_and_their_neighbours_join_to_them() {
    let templates = joined_templates();
    let pool = templates.pool_id("paths").unwrap();
    let crossing = templates.template_id("crossing").unwrap();
    let reserved = [Reservation {
        x: 3,
        y: 2,
        template: crossing,
    }];
    let layout = templates
        .roll_matched(4, WIDTH, HEIGHT, pool, &reserved)
        .unwrap();
    assert_eq!(layout.template_at(3, 2), Some(crossing));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let socket = |x, y, side| {
                let id = layout.template_at(x, y).unwrap();
                templates.template(id).unwrap().socket(side)
            };
            if x + 1 < WIDTH {
                assert_eq!(socket(x, y, EAST), socket(x + 1, y, WEST), "({x}, {y})");
            }
            if y + 1 < HEIGHT {
                assert_eq!(socket(x, y, SOUTH), socket(x, y + 1, NORTH), "({x}, {y})");
            }
        }
    }
    let map = built(&templates, &layout);
    let side = templates.side();
    // Every side of a crossing is a gate, so each neighbour is open where it meets it.
    assert!(map.is_passable(map.index(3 * side - 1, 2 * side + 3)));
    assert!(map.is_passable(map.index(4 * side, 2 * side + 3)));
}

#[test]
fn a_set_of_reservations_no_layout_can_join_is_refused() {
    let templates = joined_templates();
    let pool = templates.pool_id("paths").unwrap();
    let crossing = templates.template_id("crossing").unwrap();
    let plot = templates.template_id("plot").unwrap();
    // A gate meets a fence on the shared edge.
    let side_by_side = [
        Reservation {
            x: 1,
            y: 1,
            template: crossing,
        },
        Reservation {
            x: 2,
            y: 1,
            template: plot,
        },
    ];
    assert_eq!(
        templates.roll_matched(1, WIDTH, HEIGHT, pool, &side_by_side),
        Err(RollError::NoLayout)
    );
    // The chunk between them would need a west gate and an east fence at once.
    let one_apart = [
        Reservation {
            x: 1,
            y: 1,
            template: crossing,
        },
        Reservation {
            x: 3,
            y: 1,
            template: plot,
        },
    ];
    assert_eq!(
        templates.roll_matched(1, WIDTH, HEIGHT, pool, &one_apart),
        Err(RollError::NoLayout)
    );
}

#[test]
fn a_search_that_cannot_finish_in_its_budget_says_so() {
    let templates = joined_templates();
    let pool = templates.pool_id("paths").unwrap();
    let crossing = templates.template_id("crossing").unwrap();
    let plot = templates.template_id("plot").unwrap();
    // The contradiction sits at the end, behind a long row-major prefix of free chunks.
    let reserved = [
        Reservation {
            x: 12,
            y: 15,
            template: crossing,
        },
        Reservation {
            x: 15,
            y: 15,
            template: plot,
        },
    ];
    assert_eq!(
        templates.roll_matched(2, 16, 16, pool, &reserved),
        Err(RollError::SearchLimit)
    );
}

#[test]
fn templates_without_sockets_cannot_be_matched() {
    let plain = estate::templates();
    let pool = plain.pool_id("garden").unwrap();
    assert_eq!(
        plain.roll_matched(1, 2, 2, pool, &[]),
        Err(RollError::NoSockets)
    );
}
