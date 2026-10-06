use capsule::{
    default_seed, default_steps, fixture_hash, run_fixture, runner, world_hash, Capsule, Cell,
    Event, Intent,
};
use lockstep_core::Simulation;

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

#[test]
fn the_survivor_walks_horizontally_then_vertically_and_arrives() {
    let mut walker = runner(3);
    let survivor = walker.simulation().survivor().unwrap();
    let start = *walker.simulation().world().positions.get(survivor).unwrap();
    let target = Cell { x: 15, y: 15 };
    let mut arrived = None;
    for step in 0..2_000u64 {
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
            position.x == target.x || position.y == start.y,
            "vertical movement before the horizontal part finished: {position:?}"
        );
        if result
            .events
            .iter()
            .any(|event| matches!(event, Event::Arrived { .. }))
        {
            arrived = Some(step);
            break;
        }
    }
    let cells = (target.x - start.x).abs() + (target.y - start.y).abs();
    assert_eq!(
        arrived,
        Some(cells as u64 * 6 - 1),
        "six steps per walked cell"
    );
    assert_eq!(
        walker.simulation().world().positions.get(survivor),
        Some(&target)
    );
    assert!(walker.simulation().world().walks.is_empty());
}

#[test]
fn running_covers_a_cell_in_half_the_steps() {
    let mut runner = runner(3);
    let survivor = runner.simulation().survivor().unwrap();
    let start = *runner.simulation().world().positions.get(survivor).unwrap();
    let target = Cell {
        x: (start.x + 1) % 16,
        y: start.y,
    };
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
