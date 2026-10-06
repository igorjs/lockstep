// SPDX-License-Identifier: Apache-2.0
use crowd::{fixture_hash, run_fixture, runner, Crowd, Event, World, DEFAULT_SEED, DEFAULT_STEPS};
use lockstep_core::{hash_of, Simulation};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    // `just determinism` also checks this with the `parallel` feature on, where the batch runs on a
    // thread pool: the committed value is the same either way.
    let committed = include_str!("../fixtures/crowd.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn most_of_the_crowd_reaches_the_goal_and_nobody_shares_a_cell() {
    let mut runner = runner(DEFAULT_SEED);
    let start = runner.simulation().walking();
    let mut arrived = 0;
    for _ in 0..DEFAULT_STEPS {
        arrived += runner
            .step_once(&[])
            .events
            .iter()
            .filter(|event| matches!(event, Event::Arrived { .. }))
            .count();
        let world = runner.simulation().world();
        let mut cells: Vec<u32> = world.positions.iter().map(|(_, cell)| cell.0).collect();
        let before = cells.len();
        cells.sort_unstable();
        cells.dedup();
        assert_eq!(cells.len(), before, "two bodies on one cell");
        assert!(
            world
                .positions
                .iter()
                .all(|(_, cell)| world.map.is_passable(*cell)),
            "a body on a wall"
        );
    }
    assert_eq!(
        arrived + runner.simulation().walking(),
        start,
        "every body either arrived or is still walking"
    );
    assert!(arrived * 2 > start, "only {arrived} of {start} arrived");
}

#[test]
fn a_saved_crowd_continues_exactly_like_the_original() {
    let mut original = run_fixture(DEFAULT_SEED, 40);
    let saved: World = bincode_round_trip(&original.snapshot());
    let mut restored = Crowd::restore(saved);
    assert_eq!(restored.snapshot(), original.snapshot());
    // Step the restored world through the simulation directly, and the original through its runner.
    let clock = lockstep_core::Clock::new(
        lockstep_core::ClockConfiguration {
            day_length_real_minutes: 1440.0,
            sunrise_minute: 0,
            sunset_minute: 1440,
            starting_minute: 0,
            starting_day: 0,
        },
        1.0 / 30.0,
    );
    let mut randomness = lockstep_core::Streams::new(0);
    for step in 40..80 {
        let mut events = Vec::new();
        let mut context = lockstep_core::Context {
            clock: &clock,
            elapsed_game_minutes: 0.0,
            elapsed_minutes: lockstep_core::math::Fixed32::ZERO,
            randomness: &mut randomness,
            events: &mut events,
            step_number: step,
            step_seconds: 1.0 / 30.0,
        };
        restored.step(&mut context, &[]);
        original.step_once(&[]);
        assert_eq!(
            hash_of(&restored.snapshot()),
            hash_of(&original.snapshot()),
            "step {step}"
        );
    }
}

fn bincode_round_trip(world: &World) -> World {
    bincode::deserialize(&bincode::serialize(world).unwrap()).unwrap()
}
