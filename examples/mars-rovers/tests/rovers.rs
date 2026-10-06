// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{hash_of, Handle, Runner, Simulation};
use lockstep_spatial::{GridMap, Hex, Square4, Square8};
use mars_rovers::{
    compass, fixture_hash, instructions, opposite, play, runner, Configuration, Event, Intent,
    MarsRovers, Obstacle, RoverTopology, World, DEFAULT_SEED, DEFAULT_STEPS,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn plateau(width: u32, height: u32, rocks: &[(u32, u32)]) -> Configuration {
    Configuration {
        width,
        height,
        rocks: rocks.to_vec(),
    }
}

fn rover<T: RoverTopology>(runner: &Runner<MarsRovers<T>>, index: usize) -> Handle {
    runner.simulation().rovers()[index]
}

fn all_events<T: RoverTopology>(runner: &mut Runner<MarsRovers<T>>, steps: usize) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..steps {
        events.extend(runner.step_once(&[]).events);
    }
    events
}

/// Lands rovers (one step), then programs them (one step), and returns the runner before any
/// instruction has run.
fn ready<T: RoverTopology>(
    configuration: Configuration,
    landings: &[(u32, u32, u8)],
    programs: &[(usize, &str)],
) -> Runner<MarsRovers<T>> {
    let mut runner = runner::<T>(configuration, DEFAULT_SEED);
    let lands: Vec<Intent> = landings
        .iter()
        .map(|(x, y, heading)| Intent::Land {
            x: *x,
            y: *y,
            heading: *heading,
        })
        .collect();
    runner.step_once(&lands);
    let rovers = runner.simulation().rovers();
    let programs: Vec<Intent> = programs
        .iter()
        .map(|(index, letters)| Intent::Program {
            rover: rovers[*index],
            instructions: instructions(letters),
        })
        .collect();
    runner.step_once(&programs);
    runner
}

// ---------------------------------------------------------------- the classic kata

/// The classic answers on a topology with north, east, south and west headings: the first rover
/// runs its whole program, then the second.
fn classic<T: RoverTopology>() {
    let (north, east) = (compass::NORTH, compass::east::<T>());
    let mut runner = runner::<T>(plateau(6, 6, &[]), DEFAULT_SEED);
    runner.step_once(&[
        Intent::Land {
            x: 1,
            y: 2,
            heading: north,
        },
        Intent::Land {
            x: 3,
            y: 3,
            heading: east,
        },
    ]);
    let (first, second) = (rover(&runner, 0), rover(&runner, 1));

    runner.step_once(&[Intent::Program {
        rover: first,
        instructions: instructions("LMLMLMLMM"),
    }]);
    all_events(&mut runner, 9);
    assert_eq!(runner.simulation().position_of(first), Some((1, 3)));
    assert_eq!(runner.simulation().heading_of(first), Some(north));

    runner.step_once(&[Intent::Program {
        rover: second,
        instructions: instructions("MMRMMRMRRM"),
    }]);
    all_events(&mut runner, 10);
    assert_eq!(runner.simulation().position_of(second), Some((5, 1)));
    assert_eq!(runner.simulation().heading_of(second), Some(east));
}

#[test]
fn the_classic_kata_answers_are_unchanged_on_square4() {
    classic::<Square4>();
}

#[test]
fn the_classic_kata_answers_are_unchanged_on_square8() {
    classic::<Square8>();
}

// ------------------------------------------------------------ the same scenarios everywhere

/// The plateau position of the neighbour of `centre` in a direction. The tests use a nine by nine
/// plateau, and the kata's y grows northward while the grid's grows southward.
fn neighbour_of<T: RoverTopology>(centre: (u32, u32), direction: usize) -> (u32, u32) {
    let map: GridMap<T> = GridMap::new(11, 11);
    let mut neighbours = Vec::new();
    map.neighbours(map.index(centre.0 + 1, 9 - centre.1), &mut neighbours);
    let (x, y) = map.coordinates(neighbours[direction]);
    (x - 1, 9 - y)
}

/// The landing that puts a rover at `from_direction` of `centre`, facing it.
fn approach<T: RoverTopology>(centre: (u32, u32), from_direction: usize) -> (u32, u32, u8) {
    let (x, y) = neighbour_of::<T>(centre, from_direction);
    (x, y, opposite::<T>(from_direction as u8))
}

fn scenarios<T: RoverTopology>() {
    let centre = (4, 4);
    let count = T::NEIGHBOURS;
    let open = || plateau(9, 9, &[]);

    // Head-on: two rovers face each other, side by side. Neither can move.
    let a = approach::<T>(centre, 0);
    let b = (centre.0, centre.1, opposite::<T>(a.2));
    let mut runner = ready::<T>(open(), &[a, b], &[(0, "M"), (1, "M")]);
    let events = all_events(&mut runner, 1);
    let (low, high) = (rover(&runner, 0), rover(&runner, 1));
    assert_eq!(
        events,
        vec![
            Event::Blocked {
                rover: low,
                obstacle: Obstacle::Rover
            },
            Event::Blocked {
                rover: high,
                obstacle: Obstacle::Rover
            }
        ]
    );
    assert_eq!(runner.simulation().position_of(low), Some((a.0, a.1)));
    assert_eq!(runner.simulation().position_of(high), Some(centre));

    // Contested cell: two rovers approach the same empty cell from opposite sides.
    let (first, second) = (approach::<T>(centre, 0), approach::<T>(centre, count / 2));
    let mut runner = ready::<T>(open(), &[first, second], &[(0, "M"), (1, "M")]);
    let events = all_events(&mut runner, 1);
    let (low, high) = (rover(&runner, 0), rover(&runner, 1));
    assert_eq!(
        runner.simulation().position_of(low),
        Some(centre),
        "the lower handle moved in"
    );
    assert_eq!(
        runner.simulation().position_of(high),
        Some((second.0, second.1)),
        "the higher handle was blocked"
    );
    assert!(events.contains(&Event::Blocked {
        rover: high,
        obstacle: Obstacle::Rover
    }));

    // Rock: a rover facing a rock does not move.
    let start = approach::<T>(centre, 0);
    let mut runner = ready::<T>(plateau(9, 9, &[centre]), &[start], &[(0, "M")]);
    let events = all_events(&mut runner, 1);
    assert_eq!(
        events,
        vec![Event::Blocked {
            rover: rover(&runner, 0),
            obstacle: Obstacle::Rock
        }]
    );
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some((start.0, start.1))
    );

    // Following into a vacated cell. The follower has the higher handle, so it moves into the cell
    // the leader left in the same step.
    let heading = opposite::<T>(0);
    let leader_cell = neighbour_of::<T>(centre, 0);
    let follower_cell = neighbour_of::<T>(leader_cell, 0);
    let mut runner = ready::<T>(
        open(),
        &[
            (leader_cell.0, leader_cell.1, heading),
            (follower_cell.0, follower_cell.1, heading),
        ],
        &[(0, "M"), (1, "M")],
    );
    all_events(&mut runner, 1);
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 1)),
        Some(leader_cell),
        "the follower took the vacated cell"
    );
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some(centre)
    );

    // A follower with the lower handle is blocked first, and moves on the next step.
    let mut runner = ready::<T>(
        open(),
        &[
            (follower_cell.0, follower_cell.1, heading),
            (leader_cell.0, leader_cell.1, heading),
        ],
        &[(0, "MM"), (1, "M")],
    );
    all_events(&mut runner, 1);
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some(follower_cell),
        "blocked: the leader had not moved yet"
    );
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 1)),
        Some(centre)
    );
    all_events(&mut runner, 1);
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some(leader_cell),
        "the next step it follows"
    );
}

/// A rover on the rim facing off the plateau does not move.
fn edge<T: RoverTopology>() {
    let map: GridMap<T> = GridMap::new(11, 11);
    let rim = (0u32, 4u32);
    let mut neighbours = Vec::new();
    map.neighbours(map.index(rim.0 + 1, 9 - rim.1), &mut neighbours);
    let outward = neighbours
        .iter()
        .position(|cell| map.coordinates(*cell).0 == 0)
        .expect("a way off the plateau") as u8;
    let mut runner = ready::<T>(plateau(9, 9, &[]), &[(rim.0, rim.1, outward)], &[(0, "M")]);
    let events = all_events(&mut runner, 1);
    assert_eq!(
        events,
        vec![Event::Blocked {
            rover: rover(&runner, 0),
            obstacle: Obstacle::Edge
        }]
    );
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some(rim)
    );
}

#[test]
fn the_edge_rock_head_on_contested_and_following_scenarios_hold_on_square4() {
    scenarios::<Square4>();
    edge::<Square4>();
}

#[test]
fn the_same_scenarios_hold_on_square8() {
    scenarios::<Square8>();
    edge::<Square8>();
}

#[test]
fn the_same_scenarios_hold_on_hex() {
    scenarios::<Hex>();
    edge::<Hex>();
}

fn wrecked_landings<T: RoverTopology>() {
    let mut runner = runner::<T>(plateau(5, 5, &[(2, 2)]), DEFAULT_SEED);
    let events = runner
        .step_once(&[
            Intent::Land {
                x: 1,
                y: 1,
                heading: 0,
            },
            Intent::Land {
                x: 1,
                y: 1,
                heading: 0,
            },
            Intent::Land {
                x: 2,
                y: 2,
                heading: 0,
            },
            Intent::Land {
                x: 9,
                y: 9,
                heading: 0,
            },
        ])
        .events;
    let rovers = runner.simulation().rovers();
    assert_eq!(rovers.len(), 4, "a wrecked rover still exists");
    assert!(matches!(events[0], Event::Landed { x: 1, y: 1, .. }));
    assert_eq!(
        events[1],
        Event::Wrecked {
            rover: rovers[1],
            obstacle: Obstacle::Rover
        }
    );
    assert_eq!(
        events[2],
        Event::Wrecked {
            rover: rovers[2],
            obstacle: Obstacle::Rock
        }
    );
    assert_eq!(
        events[3],
        Event::Wrecked {
            rover: rovers[3],
            obstacle: Obstacle::Edge
        }
    );
    assert!(rovers[1..]
        .iter()
        .all(|wreck| runner.simulation().is_wrecked(*wreck)));
    assert_eq!(
        runner.simulation().position_of(rovers[1]),
        None,
        "a wreck holds no cell"
    );

    // A wreck never moves, and it does not block the cell it failed to land on.
    runner.step_once(&[Intent::Program {
        rover: rovers[1],
        instructions: instructions("MMM"),
    }]);
    assert!(all_events(&mut runner, 3).is_empty());
    assert_eq!(runner.simulation().position_of(rovers[0]), Some((1, 1)));
}

#[test]
fn a_wrecked_landing_holds_no_cell_and_never_moves_on_every_topology() {
    wrecked_landings::<Square4>();
    wrecked_landings::<Square8>();
    wrecked_landings::<Hex>();
}

fn full_circle<T: RoverTopology>(turns: usize) {
    let mut runner = ready::<T>(plateau(5, 5, &[]), &[(2, 2, 0)], &[(0, &"R".repeat(turns))]);
    let heading =
        |runner: &Runner<MarsRovers<T>>| runner.simulation().heading_of(rover(runner, 0)).unwrap();
    let mut seen = vec![heading(&runner)];
    for _ in 0..turns {
        runner.step_once(&[]);
        seen.push(heading(&runner));
    }
    assert_eq!(
        seen.first(),
        seen.last(),
        "a full circle returns to the start"
    );
    let mut distinct = seen.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        turns,
        "every heading on the way is different"
    );
}

#[test]
fn turning_right_cycles_through_the_headings_and_left_undoes_it() {
    full_circle::<Square4>(4);
    full_circle::<Square8>(4); // ninety degrees at a time: four turns of the eight directions
    full_circle::<Hex>(6);
    let mut runner = ready::<Square8>(plateau(5, 5, &[]), &[(2, 2, 2)], &[(0, "RLLR")]);
    all_events(&mut runner, 4);
    assert_eq!(runner.simulation().heading_of(rover(&runner, 0)), Some(2));
}

#[test]
fn a_snapshot_saves_and_restores_to_an_identical_world() {
    let mut runner = ready::<Square8>(
        plateau(9, 9, &[(5, 5)]),
        &[(1, 1, 2), (2, 1, 2)],
        &[(0, "MMRMMLMM"), (1, "MMMM")],
    );
    all_events(&mut runner, 3);
    let snapshot = runner.snapshot();
    let bytes = bincode::serialize(&snapshot).unwrap();
    let decoded: World<Square8> = bincode::deserialize(&bytes).unwrap();
    assert_eq!(decoded, snapshot);
    assert_eq!(hash_of(&decoded), hash_of(&snapshot));
    let restored = <MarsRovers<Square8> as Simulation>::restore(decoded);
    assert_eq!(restored.snapshot(), snapshot);
    for rover in runner.simulation().rovers() {
        assert_eq!(
            restored.position_of(rover),
            runner.simulation().position_of(rover)
        );
    }
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/mars-rovers.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn the_same_seed_gives_the_same_hash_and_running_longer_changes_it() {
    assert_eq!(fixture_hash(1, 40), fixture_hash(1, 40));
    assert_ne!(fixture_hash(1, 1), fixture_hash(1, 40));
}

#[test]
fn play_runs_a_scripted_scenario() {
    let runner = play::<Square4>(
        plateau(6, 6, &[]),
        DEFAULT_SEED,
        &[(1, 2, compass::NORTH)],
        &[(0, "LMLMLMLMM")],
        12,
    );
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some((1, 3))
    );
}

#[test]
fn a_moved_event_reports_the_plateau_position_the_rover_now_holds() {
    let mut runner = ready::<Square4>(plateau(6, 6, &[]), &[(1, 2, compass::NORTH)], &[(0, "MRM")]);
    let rover = rover(&runner, 0);
    let first = runner.step_once(&[]).events;
    assert_eq!(
        first,
        vec![Event::Moved { rover, x: 1, y: 3 }],
        "north is y + 1 in kata numbers"
    );
    assert_eq!(runner.simulation().position_of(rover), Some((1, 3)));
    runner.step_once(&[]);
    let third = runner.step_once(&[]).events;
    assert_eq!(third, vec![Event::Moved { rover, x: 2, y: 3 }]);
    assert_eq!(runner.simulation().position_of(rover), Some((2, 3)));
}

#[test]
fn a_restored_world_rebuilds_who_stands_where() {
    // Two rovers side by side; save, restore, and ask the first to move into the second. Only a
    // rebuilt occupancy can refuse it.
    let runner = ready::<Square8>(
        plateau(9, 9, &[]),
        &[(2, 2, compass::east::<Square8>()), (3, 2, 0)],
        &[],
    );
    let (first, second) = (rover(&runner, 0), rover(&runner, 1));
    let world: World<Square8> =
        bincode::deserialize(&bincode::serialize(&runner.snapshot()).unwrap()).unwrap();
    let mut restored = <MarsRovers<Square8> as Simulation>::restore(world);
    let clock = lockstep_core::Clock::new(mars_rovers_clock(), 1.0 / 30.0);
    let mut randomness = lockstep_core::Streams::new(0);
    let mut events = Vec::new();
    let mut step =
        |simulation: &mut MarsRovers<Square8>, intents: &[Intent], events: &mut Vec<Event>| {
            let mut context = lockstep_core::Context {
                clock: &clock,
                elapsed_game_minutes: 0.0,
                elapsed_minutes: lockstep_core::math::Fixed32::ZERO,
                randomness: &mut randomness,
                events,
                step_number: 0,
                step_seconds: 1.0 / 30.0,
            };
            simulation.step(&mut context, intents);
        };
    step(
        &mut restored,
        &[Intent::Program {
            rover: first,
            instructions: instructions("M"),
        }],
        &mut events,
    );
    step(&mut restored, &[], &mut events);
    assert_eq!(
        events,
        vec![Event::Blocked {
            rover: first,
            obstacle: Obstacle::Rover
        }]
    );
    assert_eq!(restored.position_of(second), Some((3, 2)));
}

fn mars_rovers_clock() -> lockstep_core::ClockConfiguration {
    lockstep_core::ClockConfiguration {
        day_length_real_minutes: 1440.0,
        sunrise_minute: 0,
        sunset_minute: 1440,
        starting_minute: 0,
        starting_day: 0,
    }
}

#[test]
fn a_heading_past_the_neighbour_count_wraps_around() {
    let mut runner = ready::<Hex>(plateau(9, 9, &[]), &[(4, 4, 7)], &[(0, "M")]);
    assert_eq!(
        runner.simulation().heading_of(rover(&runner, 0)),
        Some(1),
        "seven of six wraps to one"
    );
    let events = runner.step_once(&[]).events;
    assert!(
        matches!(events[0], Event::Moved { .. }),
        "and the rover moves, it does not panic"
    );
}

#[test]
fn a_rock_outside_the_plateau_is_ignored() {
    let mut runner = ready::<Square4>(
        plateau(4, 4, &[(9, 9), (4, 0), (0, 4)]),
        &[(0, 3, compass::east::<Square4>())],
        &[(0, "MMM")],
    );
    all_events(&mut runner, 3);
    assert_eq!(
        runner.simulation().position_of(rover(&runner, 0)),
        Some((3, 3)),
        "the rocks outside changed nothing"
    );
}

#[test]
fn west_faces_east_on_every_topology() {
    fn check<T: RoverTopology>() {
        assert_eq!(compass::west::<T>(), opposite::<T>(compass::east::<T>()));
        assert_eq!(compass::south::<T>(), opposite::<T>(compass::NORTH));
    }
    check::<Square4>();
    check::<Square8>();
    check::<Hex>();
    assert_eq!(
        compass::west::<Hex>(),
        4,
        "west on a hexagon, not south-west"
    );
}

// ------------------------------------------------------------------------------- ramming

fn ramming<T: RoverTopology>() {
    let centre = (4, 4);
    for from in 0..T::NEIGHBOURS {
        let rammer = approach::<T>(centre, from);
        let heading = rammer.2 as usize;
        let beyond = neighbour_of::<T>(centre, heading);

        // Open ground: the other rover moves one cell the way the rammer faces, and the rammer
        // takes its cell.
        let mut runner = ready::<T>(
            plateau(9, 9, &[]),
            &[rammer, (centre.0, centre.1, 0)],
            &[(0, "X")],
        );
        let (first, second) = (rover(&runner, 0), rover(&runner, 1));
        let events = runner.step_once(&[]).events;
        assert_eq!(
            events[0],
            Event::Rammed {
                rover: first,
                other: second,
                pushed: true
            },
            "{} from {from}",
            std::any::type_name::<T>()
        );
        assert_eq!(runner.simulation().position_of(first), Some(centre));
        assert_eq!(runner.simulation().position_of(second), Some(beyond));

        // A rock beyond: nobody moves.
        let mut runner = ready::<T>(
            plateau(9, 9, &[beyond]),
            &[rammer, (centre.0, centre.1, 0)],
            &[(0, "X")],
        );
        let (first, second) = (rover(&runner, 0), rover(&runner, 1));
        let events = runner.step_once(&[]).events;
        assert_eq!(
            events,
            vec![Event::Rammed {
                rover: first,
                other: second,
                pushed: false
            }],
            "{} from {from}: a rock stops the push",
            std::any::type_name::<T>()
        );
        assert_eq!(
            runner.simulation().position_of(first),
            Some((rammer.0, rammer.1))
        );
        assert_eq!(runner.simulation().position_of(second), Some(centre));

        // A third rover beyond: rams do not chain.
        let mut runner = ready::<T>(
            plateau(9, 9, &[]),
            &[rammer, (centre.0, centre.1, 0), (beyond.0, beyond.1, 0)],
            &[(0, "X")],
        );
        let (second, third) = (rover(&runner, 1), rover(&runner, 2));
        runner.step_once(&[]);
        assert_eq!(runner.simulation().position_of(second), Some(centre));
        assert_eq!(runner.simulation().position_of(third), Some(beyond));
    }
}

#[test]
fn a_ram_pushes_the_rover_ahead_one_cell_unless_something_is_beyond_it_on_every_topology() {
    ramming::<Square4>();
    ramming::<Square8>();
    ramming::<Hex>();
}

#[test]
fn a_ram_into_the_edge_pushes_nobody_and_a_ram_into_open_ground_is_a_move() {
    let east = compass::east::<Square4>();
    let mut runner = ready::<Square4>(plateau(4, 4, &[]), &[(2, 0, east), (3, 0, 0)], &[(0, "X")]);
    let (first, second) = (rover(&runner, 0), rover(&runner, 1));
    let events = runner.step_once(&[]).events;
    assert_eq!(
        events,
        vec![Event::Rammed {
            rover: first,
            other: second,
            pushed: false
        }]
    );
    assert_eq!(runner.simulation().position_of(second), Some((3, 0)));

    let mut runner = ready::<Square4>(plateau(4, 4, &[]), &[(0, 0, east)], &[(0, "XX")]);
    let first = rover(&runner, 0);
    let events = all_events(&mut runner, 2);
    assert_eq!(
        events,
        vec![
            Event::Moved {
                rover: first,
                x: 1,
                y: 0
            },
            Event::Moved {
                rover: first,
                x: 2,
                y: 0
            },
        ]
    );
    // And at the edge it is blocked like a move.
    let mut runner = ready::<Square4>(plateau(4, 4, &[]), &[(3, 0, east)], &[(0, "X")]);
    let first = rover(&runner, 0);
    assert_eq!(
        runner.step_once(&[]).events,
        vec![Event::Blocked {
            rover: first,
            obstacle: Obstacle::Edge
        }]
    );
}
