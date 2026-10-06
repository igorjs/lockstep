// SPDX-License-Identifier: Apache-2.0
use lockstep_combat::{CombatEvent, Gait, MovementEvent, Phase};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Handle, Runner, Simulation, Streams};
use sparring::{
    coach, duel, fixture_hash, runner, Configuration, Event, Intent, Sparring, DEFAULT_SEED,
    DEFAULT_STEPS, HEALTH, IMPACT_DAMAGE, STICK, SWEEP,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const WEST: u16 = 32_768;

fn pair(at: [(u32, u32); 2]) -> Configuration {
    Configuration {
        width: 12,
        height: 7,
        pillars: Vec::new(),
        partners: at.to_vec(),
    }
}

fn partners(runner: &Runner<Sparring>) -> (Handle, Handle) {
    let all = runner.simulation().partners();
    (all[0], all[1])
}

fn wait(runner: &mut Runner<Sparring>, steps: u32) -> Vec<Event> {
    (0..steps)
        .flat_map(|_| runner.step_once(&[]).events)
        .collect()
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/sparring.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn the_scripted_duel_has_dodges_on_both_sides_strikes_throws_and_yields() {
    let mut runner = runner(duel(), DEFAULT_SEED);
    let (first, second) = partners(&runner);
    let mut script = Streams::new(DEFAULT_SEED ^ 0x7370_6172);
    let mut events = Vec::new();
    for step in 0..DEFAULT_STEPS {
        let intents = coach(runner.simulation(), &mut script, step);
        events.extend(runner.step_once(&intents).events);
    }
    for who in [first, second] {
        assert!(events.contains(&Event::Combat(CombatEvent::DodgeStarted { who })));
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Combat(CombatEvent::Landed { who: by, .. }) if *by == who)));
        assert!(events.contains(&Event::Threw { who }));
    }
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Combat(CombatEvent::ProjectileHit { .. }))));
    let yields = events
        .iter()
        .filter(|event| matches!(event, Event::Yielded { .. }))
        .count() as u32;
    let world = runner.simulation().world();
    let points: u32 = [first, second]
        .iter()
        .map(|who| world.points.get(*who).copied().unwrap_or(0))
        .sum();
    assert!(yields > 0);
    assert_eq!(points, yields, "every yield is a point for the other");
}

#[test]
fn a_sweep_into_the_edge_of_the_floor_deals_impact_damage() {
    // The struck partner stands on the west edge; the sweep pushes it west, into the edge.
    let mut runner = runner(pair([(0, 3), (1, 3)]), 3);
    let (struck, sweeper) = partners(&runner);
    let mut events = runner
        .step_once(&[Intent::Attack {
            who: sweeper,
            action: SWEEP,
            facing: WEST,
        }])
        .events;
    events.extend(wait(&mut runner, 40));
    let hurt = events.iter().find_map(|event| match event {
        Event::Hurt { who, health, .. } if *who == struck => Some(*health),
        _ => None,
    });
    let health = hurt.expect("the sweep landed");
    assert!(events.contains(&Event::Impact {
        who: struck,
        health: health - Fixed32::from_int(IMPACT_DAMAGE)
    }));
    assert_eq!(
        runner.simulation().cell_of(struck),
        Some(runner.simulation().world().map.index(0, 3)),
        "the edge stopped the push"
    );
}

#[test]
fn a_sweep_in_the_open_pushes_without_impact() {
    let mut runner = runner(pair([(4, 3), (5, 3)]), 3);
    let (struck, sweeper) = partners(&runner);
    let mut events = runner
        .step_once(&[Intent::Attack {
            who: sweeper,
            action: SWEEP,
            facing: WEST,
        }])
        .events;
    events.extend(wait(&mut runner, 40));
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Hurt { who, .. } if *who == struck)));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Impact { .. })));
    assert_eq!(
        runner.simulation().cell_of(struck),
        Some(runner.simulation().world().map.index(3, 3))
    );
}

#[test]
fn a_partner_winding_up_does_not_walk() {
    let mut runner = runner(pair([(2, 3), (9, 3)]), 3);
    let (walker, _) = partners(&runner);
    runner.step_once(&[Intent::Move {
        who: walker,
        x: 8,
        y: 3,
        gait: Gait::Walk,
    }]);
    let events = runner
        .step_once(&[Intent::Attack {
            who: walker,
            action: STICK,
            facing: 0,
        }])
        .events;
    assert!(events.contains(&Event::Combat(CombatEvent::Started {
        who: walker,
        action: STICK
    })));
    let mut moved = false;
    for _ in 0..60 {
        let fighter_busy = runner.simulation().fighter(walker).unwrap().phase != Phase::Ready;
        let events = runner.step_once(&[]).events;
        moved |= fighter_busy
            && events.iter().any(|event| {
                matches!(event, Event::Movement(MovementEvent::Moved { who, .. }) if *who == walker)
            });
    }
    assert!(!moved, "no step while busy");
}

#[test]
fn a_move_sent_with_an_attack_waits_for_the_attack_to_finish() {
    let mut runner = runner(pair([(2, 3), (9, 3)]), 3);
    let (walker, _) = partners(&runner);
    let start = runner.simulation().cell_of(walker);
    runner.step_once(&[
        Intent::Attack {
            who: walker,
            action: STICK,
            facing: 0,
        },
        Intent::Move {
            who: walker,
            x: 8,
            y: 3,
            gait: Gait::Walk,
        },
    ]);
    for _ in 0..30 {
        assert_eq!(runner.simulation().cell_of(walker), start);
        runner.step_once(&[]);
    }
}

#[test]
fn a_throw_needs_a_ready_thrower() {
    let mut runner = runner(pair([(2, 3), (9, 3)]), 3);
    let (thrower, other) = partners(&runner);
    runner.step_once(&[Intent::Attack {
        who: thrower,
        action: STICK,
        facing: 0,
    }]);
    let events = runner
        .step_once(&[Intent::Throw {
            who: thrower,
            facing: 0,
        }])
        .events;
    assert!(events.contains(&Event::ThrowRefused { who: thrower }));
    let events = runner
        .step_once(&[Intent::Throw {
            who: other,
            facing: WEST,
        }])
        .events;
    assert!(events.contains(&Event::Threw { who: other }));
}

#[test]
fn a_yield_restores_health_in_full() {
    let mut runner = runner(duel(), DEFAULT_SEED);
    let mut script = Streams::new(DEFAULT_SEED ^ 0x7370_6172);
    for step in 0..DEFAULT_STEPS {
        let intents = coach(runner.simulation(), &mut script, step);
        let events = runner.step_once(&intents).events;
        for event in &events {
            if let Event::Yielded { who, .. } = event {
                let hit_again = events
                    .iter()
                    .skip_while(|later| *later != event)
                    .any(|later| matches!(later, Event::Hurt { who: hurt, .. } if hurt == who));
                if !hit_again {
                    assert_eq!(
                        runner.simulation().health_of(*who),
                        Some(Fixed32::from_int(HEALTH))
                    );
                }
            }
        }
    }
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    simulation: &mut Sparring,
    clock: &mut Clock,
    randomness: &mut Streams,
    intents: &[Intent],
    number: u64,
) -> Vec<Event> {
    let (minutes, exact) = clock.advance_exactly(&mut Vec::new());
    let mut events = Vec::new();
    let mut context = Context {
        clock,
        elapsed_game_minutes: minutes,
        elapsed_minutes: exact,
        randomness,
        events: &mut events,
        step_number: number,
        step_seconds: 1.0 / 30.0,
    };
    simulation.step(&mut context, intents);
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    // The kept one has paths cached from its first steps; the restored one starts fresh.
    let session = runner(duel(), DEFAULT_SEED);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Sparring::create(duel(), &mut Streams::new(5));
    let mut script = Streams::new(77);
    for number in 0..1_500 {
        let intents = coach(&kept, &mut script, number);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let mut loaded = Sparring::restore(kept.snapshot());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 1_500..3_000 {
        let intents = coach(&kept, &mut script, number);
        let a = step(&mut kept, &mut clock, &mut randomness, &intents, number);
        let b = step(
            &mut loaded,
            &mut loaded_clock,
            &mut loaded_randomness,
            &intents,
            number,
        );
        assert_eq!(a, b, "step {number}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
}
