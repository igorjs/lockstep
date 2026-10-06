// SPDX-License-Identifier: Apache-2.0
use crater_survey::{
    fixture_hash, plateau, runner, script, Configuration, Event, Intent, RoverStart, Survey,
    ALERT_BUDGET, DEFAULT_SEED, DEFAULT_STEPS, LOW_BATTERY, OUT_OF_REACH,
};
use lockstep_agents::{Alertness, MindEvent, TaskResponse, Weather, WeatherRules};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Handle, Simulation, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const EAST: u16 = 0;

fn still() -> WeatherRules {
    WeatherRules {
        mean_strength: Fixed32::ZERO,
        fronts_per_day: 0,
    }
}

fn open(rovers: Vec<RoverStart>) -> Configuration {
    Configuration {
        width: 120,
        height: 20,
        ridges: Vec::new(),
        rovers,
        weather: still(),
    }
}

fn free(x: u32, y: u32) -> RoverStart {
    RoverStart {
        x,
        y,
        leash_metres: None,
    }
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    survey: &mut Survey,
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
    survey.step(&mut context, intents);
    events
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/crater-survey.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: across the scripted session, no leashed rover ever leaves its leash and no lander
/// ever has more than the budget of rovers alert on it.
#[test]
fn leashes_hold_and_the_budget_is_never_exceeded() {
    let mut runner = runner(plateau(), DEFAULT_SEED);
    let mut streams = Streams::new(DEFAULT_SEED ^ 0x0063_7261_7465);
    let (mut heard, mut held) = (0, 0);
    for number in 0..DEFAULT_STEPS {
        let intents = script(runner.simulation(), &mut streams, number);
        for event in runner.step_once(&intents).events {
            match event {
                Event::Heard { .. } => heard += 1,
                Event::Mind(MindEvent::Held { .. }) => held += 1,
                _ => {}
            }
        }
        let survey = runner.simulation();
        let world = survey.world();
        for rover in survey.rovers() {
            if let Some(leash) = world.leashes.get(rover) {
                let at = survey.cell_of(rover).unwrap();
                assert!(
                    leash.allows(&world.map, at, crater_survey::cell_metres()),
                    "step {number}: a leashed rover left its crater"
                );
            }
        }
        for lander in survey.landers() {
            let alert = survey
                .rovers()
                .iter()
                .filter(|rover| {
                    let mind = world.minds.get(**rover).unwrap();
                    mind.alertness == Alertness::Alert
                        && mind.memory.and_then(|memory| memory.target) == Some(lander)
                })
                .count();
            assert!(
                alert <= ALERT_BUDGET as usize,
                "step {number}: {alert} alert"
            );
        }
    }
    assert!(heard > 0, "landings were heard");
    assert!(held > 0, "the director held rovers back");
}

#[test]
fn a_free_rover_hears_a_landing_drives_toward_it_and_sees_it() {
    let mut runner = runner(open(vec![free(10, 10)]), 1);
    let rover = runner.simulation().rovers()[0];
    let events = runner.step_once(&[Intent::Land { x: 40, y: 10 }]).events;
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Heard { rover: who, .. } if *who == rover)));
    let mut alert = false;
    for _ in 0..600 {
        runner.step_once(&[]);
        alert |= runner
            .simulation()
            .world()
            .minds
            .get(rover)
            .unwrap()
            .alertness
            == Alertness::Alert;
    }
    let (x, _) = runner
        .simulation()
        .world()
        .map
        .coordinates(runner.simulation().cell_of(rover).unwrap());
    assert!(x > 20, "drove toward the lander, now at x = {x}");
    assert!(alert, "and saw it");
}

#[test]
fn the_wind_carries_a_landing_downwind_past_an_upwind_rover_as_far_away() {
    // Rovers 70 metres west and east of the landing; a 10 metre a second wind blows east.
    // Downwind the landing carries 112 metres less a 3 metre threshold; upwind 48 less 3.
    let survey = Survey::create(open(vec![free(5, 10), free(75, 10)]), &mut Streams::new(1));
    let (west, east) = (survey.rovers()[0], survey.rovers()[1]);
    let mut world = survey.world().clone();
    world.weather = Weather::new(EAST, Fixed32::from_int(10));
    world.weather_rules = WeatherRules {
        mean_strength: Fixed32::from_int(10),
        fronts_per_day: 0,
    };
    let mut survey = Survey::restore(world);
    let mut clock = runner(plateau(), 1).clock().clone();
    let events = step(
        &mut survey,
        &mut clock,
        &mut Streams::new(1),
        &[Intent::Land { x: 40, y: 10 }],
        1,
    );
    let heard: Vec<Handle> = events
        .iter()
        .filter_map(|event| match event {
            Event::Heard { rover, .. } => Some(*rover),
            _ => None,
        })
        .collect();
    assert_eq!(
        heard,
        vec![east],
        "the upwind rover at {west:?} heard nothing"
    );
}

#[test]
fn a_rover_low_on_battery_refuses_a_survey_and_says_why() {
    let survey = Survey::create(open(vec![free(5, 10)]), &mut Streams::new(1));
    let rover = survey.rovers()[0];
    let mut world = survey.world().clone();
    world.battery.set(rover, 10);
    let mut survey = Survey::restore(world);
    let mut clock = runner(plateau(), 1).clock().clone();
    let events = step(
        &mut survey,
        &mut clock,
        &mut Streams::new(1),
        &[Intent::Request { rover, x: 8, y: 10 }],
        1,
    );
    assert_eq!(
        events,
        vec![Event::Answered {
            rover,
            response: TaskResponse::Refuse {
                reason: Some(LOW_BATTERY)
            }
        }]
    );
    let mut fresh = Survey::create(open(vec![free(5, 10)]), &mut Streams::new(1));
    let events = step(
        &mut fresh,
        &mut clock,
        &mut Streams::new(1),
        &[Intent::Request { rover, x: 8, y: 10 }],
        2,
    );
    assert_eq!(
        events,
        vec![Event::Answered {
            rover,
            response: TaskResponse::Accept
        }]
    );
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let session = runner(plateau(), DEFAULT_SEED);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Survey::create(plateau(), &mut Streams::new(5));
    let mut streams = Streams::new(77);
    for number in 0..6_000 {
        let intents = script(&kept, &mut streams, number);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = Survey::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 6_000..12_000 {
        let intents = script(&kept, &mut streams, number);
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

#[test]
fn a_request_a_rover_could_never_reach_is_refused_and_a_lander_is_not_asked() {
    // Leashed to 20 metres at (10, 10); asked to survey (40, 10), 60 metres off.
    let leashed = RoverStart {
        x: 10,
        y: 10,
        leash_metres: Some(20),
    };
    let mut runner = runner(open(vec![leashed]), 1);
    let rover = runner.simulation().rovers()[0];
    let events = runner
        .step_once(&[Intent::Request {
            rover,
            x: 40,
            y: 10,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Answered {
            rover,
            response: TaskResponse::Refuse {
                reason: Some(OUT_OF_REACH)
            }
        }]
    );
    assert!(runner.simulation().world().orders.get(rover).is_none());
    let landed = runner.step_once(&[Intent::Land { x: 60, y: 10 }]).events;
    let lander = match landed[0] {
        Event::Landed { lander, .. } => lander,
        ref other => panic!("not landed: {other:?}"),
    };
    let events = runner
        .step_once(&[Intent::Request {
            rover: lander,
            x: 61,
            y: 10,
        }])
        .events;
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Answered { .. })));
}

#[test]
fn the_earliest_lander_departs_first_even_when_slots_are_reused() {
    let mut runner = runner(open(vec![free(5, 5)]), 1);
    let land = |runner: &mut lockstep_core::Runner<Survey>, x| match runner
        .step_once(&[Intent::Land { x, y: 15 }])
        .events[0]
    {
        Event::Landed { lander, .. } => lander,
        ref other => panic!("not landed: {other:?}"),
    };
    let first = land(&mut runner, 30);
    let second = land(&mut runner, 40);
    runner.step_once(&[Intent::Depart { lander: first }]);
    let third = land(&mut runner, 50);
    assert_eq!(runner.simulation().world().arrivals, vec![second, third]);
}

#[test]
fn a_rover_beside_the_lander_it_saw_parks_instead_of_circling() {
    let mut runner = runner(open(vec![free(10, 10)]), 1);
    let rover = runner.simulation().rovers()[0];
    runner.step_once(&[Intent::Land { x: 12, y: 10 }]);
    for _ in 0..600 {
        runner.step_once(&[]);
    }
    let battery = *runner.simulation().world().battery.get(rover).unwrap();
    let mut moves = 0;
    for _ in 0..600 {
        moves += runner
            .step_once(&[])
            .events
            .iter()
            .filter(|event| matches!(event, Event::Moved { .. }))
            .count();
    }
    assert_eq!(moves, 0, "parked beside the lander");
    assert!(*runner.simulation().world().battery.get(rover).unwrap() >= battery);
}
