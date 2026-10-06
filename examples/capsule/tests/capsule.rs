use capsule::{
    build_room, default_seed, default_steps, fixture_hash, run_fixture, runner, world_hash,
    Capsule, Cell, Event, Intent, STEPS_PER_CELL_WALKING, WALL_COLUMN, WALL_FIRST_ROW,
    WALL_LAST_ROW,
};
use lockstep_core::Simulation;
use lockstep_spatial::{Occupancy, PathOptions, PathResult, Pathfinder};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn collect_events(
    steps: u64,
    intents_at: impl Fn(u64) -> Vec<Intent>,
    seed: u64,
) -> (lockstep_core::Runner<Capsule>, Vec<Event>) {
    let mut runner = runner(seed);
    let mut events = Vec::new();
    for step in 0..steps {
        events.extend(runner.step_once(&intents_at(step)).events);
    }
    (runner, events)
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/capsule.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(default_seed(), default_steps())),
        committed
    );
}

#[test]
fn the_same_seed_and_script_give_the_same_hash_and_another_seed_does_not() {
    assert_eq!(fixture_hash(1, 2_000), fixture_hash(1, 2_000));
    assert_ne!(fixture_hash(1, 2_000), fixture_hash(2, 2_000));
}

/// A seed whose survivor spawns in the given column range, so a test can aim across the wall.
fn seed_spawning_in(columns: std::ops::RangeInclusive<i32>) -> u64 {
    (0..200)
        .find(|seed| {
            let runner = runner(*seed);
            let survivor = runner.simulation().survivor().unwrap();
            columns.contains(
                &runner
                    .simulation()
                    .world()
                    .positions
                    .get(survivor)
                    .unwrap()
                    .x,
            )
        })
        .expect("some seed spawns there")
}

fn is_wall(cell: Cell) -> bool {
    cell.x == WALL_COLUMN && (WALL_FIRST_ROW..=WALL_LAST_ROW).contains(&cell.y)
}

#[test]
fn nobody_spawns_on_the_wall() {
    for seed in 0..300 {
        let runner = runner(seed);
        let survivor = runner.simulation().survivor().unwrap();
        let spawn = *runner.simulation().world().positions.get(survivor).unwrap();
        assert!(
            !is_wall(spawn),
            "seed {seed} spawned on the wall at {spawn:?}"
        );
    }
}

#[test]
fn the_survivor_walks_around_the_wall_through_a_gap_and_arrives_when_the_path_ends() {
    let seed = seed_spawning_in(1..=5);
    let mut walker = runner(seed);
    let survivor = walker.simulation().survivor().unwrap();
    let start = *walker.simulation().world().positions.get(survivor).unwrap();
    let target = Cell { x: 14, y: 6 };

    // The cheapest path, found on its own, says how long the walk should take.
    let room = build_room();
    let (from, to) = (
        room.index(start.x as u32, start.y as u32),
        room.index(14, 6),
    );
    let mut expected = Vec::new();
    let found = Pathfinder::new(&room).find(
        &room,
        &Occupancy::new(&room),
        from,
        to,
        PathOptions::default(),
        &mut expected,
    );
    assert!(matches!(found, PathResult::Found { .. }));
    assert!(
        expected.len() as i32 > (target.x - start.x),
        "the way around is longer than a straight line"
    );

    let mut arrived = None;
    let mut visited = vec![start];
    for step in 0..3_000u64 {
        let intents = if step == 0 {
            vec![Intent::MoveTo {
                entity: survivor,
                cell: target,
                run: false,
            }]
        } else {
            vec![]
        };
        let result = walker.step_once(&intents);
        let position = *walker.simulation().world().positions.get(survivor).unwrap();
        assert!(
            !is_wall(position),
            "the survivor stood on the wall at {position:?} on step {step}"
        );
        if visited.last() != Some(&position) {
            visited.push(position);
        }
        if result
            .events
            .iter()
            .any(|event| matches!(event, Event::Arrived { .. }))
        {
            arrived = Some(step);
            break;
        }
    }
    assert_eq!(
        arrived,
        Some(expected.len() as u64 * STEPS_PER_CELL_WALKING as u64 - 1),
        "six steps per cell of the path"
    );
    assert_eq!(
        walker.simulation().world().positions.get(survivor),
        Some(&target)
    );
    assert!(walker.simulation().world().walks.is_empty());
    assert!(
        visited.iter().any(
            |cell| cell.x == WALL_COLUMN && (cell.y < WALL_FIRST_ROW || cell.y > WALL_LAST_ROW)
        ),
        "the survivor crossed the wall column through a gap"
    );
}

#[test]
fn a_walk_on_the_same_side_of_the_wall_goes_straight_there() {
    let seed = seed_spawning_in(0..=4);
    let mut walker = runner(seed);
    let survivor = walker.simulation().survivor().unwrap();
    let start = *walker.simulation().world().positions.get(survivor).unwrap();
    let target = Cell {
        x: (start.x + 3).min(7),
        y: start.y,
    };
    let cells = (target.x - start.x).unsigned_abs() as u64;
    let mut arrived_at = None;
    for step in 0..500u64 {
        let intents = if step == 0 {
            vec![Intent::MoveTo {
                entity: survivor,
                cell: target,
                run: false,
            }]
        } else {
            vec![]
        };
        if walker
            .step_once(&intents)
            .events
            .iter()
            .any(|event| matches!(event, Event::Arrived { .. }))
        {
            arrived_at = Some(step);
            break;
        }
    }
    let expected = if cells == 0 {
        5
    } else {
        cells * STEPS_PER_CELL_WALKING as u64 - 1
    };
    assert_eq!(arrived_at, Some(expected));
}

#[test]
fn running_covers_a_cell_in_half_the_steps() {
    let mut runner = runner(3);
    let survivor = runner.simulation().survivor().unwrap();
    let start = *runner.simulation().world().positions.get(survivor).unwrap();
    // Aim at an open neighbour that is not the wall.
    let target = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| Cell {
            x: start.x + dx,
            y: start.y + dy,
        })
        .find(|cell| (0..16).contains(&cell.x) && (0..16).contains(&cell.y) && !is_wall(*cell))
        .unwrap();
    runner.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: target,
        run: true,
    }]);
    runner.step_once(&[]);
    let result = runner.step_once(&[]);
    assert!(result
        .events
        .iter()
        .any(|event| matches!(event, Event::Arrived { .. })));
}

#[test]
fn a_move_onto_the_wall_is_rejected_and_the_survivor_stays_put() {
    let mut runner = runner(3);
    let survivor = runner.simulation().survivor().unwrap();
    let start = *runner.simulation().world().positions.get(survivor).unwrap();
    let wall = Cell {
        x: WALL_COLUMN,
        y: 5,
    };
    let result = runner.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: wall,
        run: false,
    }]);
    assert_eq!(result.events, vec![Event::Rejected { entity: survivor }]);
    for _ in 0..200 {
        runner.step_once(&[]);
    }
    assert_eq!(
        runner.simulation().world().positions.get(survivor),
        Some(&start)
    );
}

#[test]
fn a_destination_walled_in_is_rejected_as_unreachable() {
    // Seal the target cell inside a ring of walls using the room builder's own map, then check
    // the survivor's search refuses it: build the world, wall in a cell, and restore from it.
    let mut world = runner(3).snapshot();
    for (x, y) in [
        (13, 13),
        (14, 13),
        (15, 13),
        (13, 14),
        (15, 14),
        (13, 15),
        (14, 15),
        (15, 15),
    ] {
        world.room.set_passable(world.room.index(x, y), false);
    }
    // The centre cell (14, 14) is now enclosed: restore the world and ask the survivor to go there.
    let mut simulation = <Capsule as Simulation>::restore(world);
    let survivor = simulation.survivor().unwrap();
    let mut randomness = lockstep_core::Streams::new(0);
    let mut events = Vec::new();
    let clock = lockstep_core::Clock::new(capsule::clock_configuration(), 1.0 / 30.0);
    let mut context = lockstep_core::Context {
        clock: &clock,
        elapsed_game_minutes: 0.0,
        randomness: &mut randomness,
        events: &mut events,
        step_number: 0,
        step_seconds: 1.0 / 30.0,
    };
    simulation.step(
        &mut context,
        &[Intent::MoveTo {
            entity: survivor,
            cell: Cell { x: 14, y: 14 },
            run: false,
        }],
    );
    assert_eq!(events, vec![Event::Rejected { entity: survivor }]);
}

#[test]
fn a_move_outside_the_room_is_rejected_not_obeyed() {
    let (runner, events) = collect_events(
        5,
        |step| {
            if step == 0 {
                vec![Intent::MoveTo {
                    entity: lockstep_core::Handle::from_raw(0),
                    cell: Cell { x: 99, y: 0 },
                    run: false,
                }]
            } else {
                vec![]
            }
        },
        3,
    );
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], Event::Rejected { .. }));
    assert!(runner.simulation().world().walks.is_empty());
}

#[test]
fn the_survivor_starves_in_order_and_leaves_no_orphaned_components() {
    // Thirst drains faster than hunger, so the survivor is starving before the hunger warning.
    let mut resting = runner(default_seed());
    let survivor = resting.simulation().survivor().unwrap();
    resting.set_clock_multiplier(20.0);
    let mut kinds = Vec::new();
    for _ in 0..default_steps() {
        for event in resting.step_once(&[]).events {
            kinds.push(match event {
                Event::Starving { .. } => "starving",
                Event::Hungry { .. } => "hungry",
                Event::Died { .. } => "died",
                other => panic!("unexpected event {other:?}"),
            });
        }
    }
    assert_eq!(kinds, vec!["starving", "hungry", "died"]);
    assert!(!resting.simulation().world().entities.contains(survivor));
    assert!(resting.simulation().world().orphans().is_empty());
}

#[test]
fn the_scripted_session_ends_in_death_with_a_clean_world() {
    let session = run_fixture(default_seed(), default_steps());
    assert!(session.simulation().survivor().is_none());
    assert!(session.simulation().world().orphans().is_empty());
}

#[test]
fn needs_drain_with_game_time_so_a_rest_drains_them_twenty_times_faster() {
    let mut slow = runner(9);
    let mut fast = runner(9);
    fast.set_clock_multiplier(20.0);
    let survivor = slow.simulation().survivor().unwrap();
    for _ in 0..300 {
        slow.step_once(&[]);
        fast.step_once(&[]);
    }
    let slow_loss = 100.0
        - slow
            .simulation()
            .world()
            .needs
            .get(survivor)
            .unwrap()
            .hunger;
    let fast_loss = 100.0
        - fast
            .simulation()
            .world()
            .needs
            .get(survivor)
            .unwrap()
            .hunger;
    // Needs sit near 100, where a 32-bit float has coarse steps, so allow about one percent.
    assert!(
        (fast_loss / slow_loss - 20.0).abs() < 0.25,
        "ratio {}",
        fast_loss / slow_loss
    );
}

#[test]
fn a_snapshot_restores_to_an_identical_world() {
    let session = run_fixture(default_seed(), 1_000);
    let snapshot = session.snapshot();
    let restored = Capsule::restore(snapshot.clone());
    assert_eq!(restored.world(), &snapshot);
    assert_eq!(world_hash(restored.world()), world_hash(&snapshot));
}

#[test]
fn a_new_move_during_a_walk_paths_again_from_where_the_survivor_stands() {
    let seed = seed_spawning_in(0..=3);
    let mut walker = runner(seed);
    let survivor = walker.simulation().survivor().unwrap();
    walker.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: Cell { x: 14, y: 14 },
        run: false,
    }]);
    for _ in 0..20 {
        walker.step_once(&[]);
    }
    let midway = *walker.simulation().world().positions.get(survivor).unwrap();
    let new_target = Cell { x: 0, y: 15 };
    walker.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: new_target,
        run: false,
    }]);
    let walk = walker
        .simulation()
        .world()
        .walks
        .get(survivor)
        .unwrap()
        .clone();
    assert_eq!(walk.destination, new_target);
    // The new path starts next to where the survivor stood, not from the spawn.
    let room = build_room();
    let first = room.coordinates(lockstep_spatial::Cell(walk.path[0]));
    assert!((first.0 as i32 - midway.x).abs() <= 1 && (first.1 as i32 - midway.y).abs() <= 1);
    let mut arrived = false;
    for _ in 0..2_000 {
        if walker
            .step_once(&[])
            .events
            .iter()
            .any(|event| matches!(event, Event::Arrived { .. }))
        {
            arrived = true;
            break;
        }
    }
    assert!(arrived);
    assert_eq!(
        walker.simulation().world().positions.get(survivor),
        Some(&new_target)
    );
}

#[test]
fn a_stop_then_a_move_starts_a_fresh_walk() {
    let seed = seed_spawning_in(0..=3);
    let mut walker = runner(seed);
    let survivor = walker.simulation().survivor().unwrap();
    walker.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: Cell { x: 14, y: 14 },
        run: false,
    }]);
    for _ in 0..13 {
        walker.step_once(&[]);
    }
    walker.step_once(&[Intent::Stop { entity: survivor }]);
    assert!(walker.simulation().world().walks.is_empty());
    let stopped = *walker.simulation().world().positions.get(survivor).unwrap();
    for _ in 0..30 {
        walker.step_once(&[]);
    }
    assert_eq!(
        walker.simulation().world().positions.get(survivor),
        Some(&stopped),
        "a stopped survivor stays put"
    );
    let target = Cell {
        x: stopped.x,
        y: (stopped.y + 2).min(15),
    };
    walker.step_once(&[Intent::MoveTo {
        entity: survivor,
        cell: target,
        run: true,
    }]);
    let mut arrived = false;
    for _ in 0..200 {
        if walker
            .step_once(&[])
            .events
            .iter()
            .any(|event| matches!(event, Event::Arrived { .. }))
        {
            arrived = true;
            break;
        }
    }
    assert!(arrived);
}
