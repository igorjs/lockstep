// SPDX-License-Identifier: Apache-2.0
use dispatch::{
    depot, fixture_hash, runner, script, Dispatch, Event, Intent, DEFAULT_SEED, DEFAULT_STEPS,
    DELAY_MINUTES, FATIGUE, TRUST, TRUST_NEEDED,
};
use lockstep_agents::TaskResponse;
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Handle, Runner, Simulation, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn drivers(runner: &Runner<Dispatch>) -> Vec<Handle> {
    runner.simulation().drivers()
}

fn assign(runner: &mut Runner<Dispatch>, driver: Handle) -> TaskResponse {
    match runner.step_once(&[Intent::Assign { driver }]).events[0] {
        Event::Answered { response, .. } => response,
        ref other => panic!("no answer: {other:?}"),
    }
}

fn pay(runner: &mut Runner<Dispatch>, driver: Handle, on_time: bool) {
    runner.step_once(&[Intent::Pay { driver, on_time }]);
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/dispatch.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: across the session, a route offered to a driver whose trust in the dispatcher is
/// below 10 is refused naming trust, and a refusal naming trust comes only below 10; both happen.
#[test]
fn a_driver_refuses_below_the_trust_threshold_and_says_so() {
    let mut runner = runner(depot(), DEFAULT_SEED);
    let mut streams = Streams::new(DEFAULT_SEED ^ 0x0064_6973_7061);
    let (mut below, mut above) = (0, 0);
    for step in 0..DEFAULT_STEPS {
        let intents = script(runner.simulation(), &mut streams);
        // Trust as the driver weighs it: before this step's intents (at most one a step).
        let before: Vec<(Handle, Fixed32)> = runner
            .simulation()
            .drivers()
            .into_iter()
            .map(|driver| (driver, runner.simulation().trust_in_dispatcher(driver)))
            .collect();
        for event in runner.step_once(&intents).events {
            let Event::Answered { driver, response } = event else {
                continue;
            };
            let trust = before.iter().find(|(who, _)| *who == driver).unwrap().1;
            let refused_for_trust = response
                == TaskResponse::Refuse {
                    reason: Some(TRUST),
                };
            if trust < Fixed32::from_int(TRUST_NEEDED) {
                below += 1;
                assert!(refused_for_trust, "step {step}: {response:?} at {trust:?}");
            } else {
                above += 1;
                assert!(
                    !refused_for_trust,
                    "step {step}: refused for trust at {trust:?}"
                );
            }
        }
    }
    assert!(
        below > 0 && above > 0,
        "routes offered both sides: {below} below, {above} above"
    );
}

#[test]
fn ten_is_the_line() {
    let mut runner = runner(depot(), 1);
    let ama = drivers(&runner)[0];
    // Two late pays from 20: 20 less 24 is -4; then on-time pays climb back by 4.
    pay(&mut runner, ama, false);
    pay(&mut runner, ama, false);
    let mut answers = Vec::new();
    for _ in 0..5 {
        pay(&mut runner, ama, true);
        let trust = runner.simulation().trust_in_dispatcher(ama);
        answers.push((
            trust >= Fixed32::from_int(TRUST_NEEDED),
            assign(&mut runner, ama),
        ));
    }
    for (at_or_above, answer) in answers {
        let refused_for_trust = answer
            == TaskResponse::Refuse {
                reason: Some(TRUST),
            };
        assert_eq!(refused_for_trust, !at_or_above, "{answer:?}");
    }
}

#[test]
fn an_order_for_the_dispatcher_or_a_stranger_is_unknown() {
    let mut runner = runner(depot(), 1);
    let dispatcher = runner.simulation().dispatcher();
    let events = runner
        .step_once(&[
            Intent::Assign { driver: dispatcher },
            Intent::Pay {
                driver: dispatcher,
                on_time: true,
            },
        ])
        .events;
    assert_eq!(events, vec![Event::Unknown, Event::Unknown]);
}

#[test]
fn a_driver_paid_late_too_often_refuses_and_says_it_is_trust() {
    let mut runner = runner(depot(), 1);
    let ama = drivers(&runner)[0];
    // Trust starts at 20: a fresh driver takes the route.
    assert_eq!(assign(&mut runner, ama), TaskResponse::Accept);
    // Two late payments: 20 less 24 is -4, well below the 10 trust needs.
    pay(&mut runner, ama, false);
    pay(&mut runner, ama, false);
    assert_eq!(
        assign(&mut runner, ama),
        TaskResponse::Refuse {
            reason: Some(TRUST)
        }
    );
}

#[test]
fn a_tired_driver_delays_then_refuses_for_fatigue() {
    let mut runner = runner(depot(), 1);
    let bix = drivers(&runner)[1];
    // Trust 20 counts 10 for; each route today counts 5 against: three routes, then delays.
    assert_eq!(assign(&mut runner, bix), TaskResponse::Accept);
    assert_eq!(assign(&mut runner, bix), TaskResponse::Accept);
    assert_eq!(assign(&mut runner, bix), TaskResponse::Accept);
    assert_eq!(
        assign(&mut runner, bix),
        TaskResponse::Delay {
            minutes: DELAY_MINUTES
        }
    );
    // A delay adds no route, so the answer holds at a delay. Ten on-time pays push trust just
    // under 60 (a step of drift each time), read as 59, so bix takes ten routes; two late
    // payments then read as 35, so ten routes count 50 against 25 for, and the refusal names
    // fatigue, the weaker reason.
    for _ in 0..10 {
        pay(&mut runner, bix, true);
    }
    while assign(&mut runner, bix) == TaskResponse::Accept {}
    pay(&mut runner, bix, false);
    pay(&mut runner, bix, false);
    assert_eq!(
        assign(&mut runner, bix),
        TaskResponse::Refuse {
            reason: Some(FATIGUE)
        }
    );
}

#[test]
fn late_pay_sours_the_shift_mates_on_the_dispatchers_shift() {
    let mut runner = runner(depot(), 1);
    let [ama, bix, col, _] = drivers(&runner)[..] else {
        panic!("four drivers")
    };
    let trust = |runner: &Runner<Dispatch>, who| runner.simulation().trust_in_dispatcher(who);
    pay(&mut runner, ama, false);
    // Bix shares the day shift with ama: 3 less toward the dispatcher's shift (a step of drift
    // back toward 20 already applied). Col is on nights and untouched; ama lost 12 herself.
    let near = |value: Fixed32, whole: i32| {
        (value - Fixed32::from_int(whole)).raw().abs() < Fixed32::from_ratio(1, 100).raw()
    };
    assert!(near(trust(&runner, bix), 17), "{:?}", trust(&runner, bix));
    assert_eq!(trust(&runner, col), Fixed32::from_int(20));
    assert!(near(trust(&runner, ama), 8), "{:?}", trust(&runner, ama));
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    dispatch: &mut Dispatch,
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
    dispatch.step(&mut context, intents);
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let session = runner(depot(), DEFAULT_SEED);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Dispatch::create(depot(), &mut Streams::new(5));
    let mut streams = Streams::new(77);
    for number in 0..12_000 {
        let intents = script(&kept, &mut streams);
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = Dispatch::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 12_000..24_000 {
        let intents = script(&kept, &mut streams);
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
