// SPDX-License-Identifier: Apache-2.0
use lockstep_calendar::CalendarEvent;
use lockstep_core::{decode, encode, hash_of, Clock, Context, Runner, Simulation, Streams};
use lockstep_regions::SpreadEvent;
use orchard::{
    fixture_hash, pace_percent, runner, starting_on, step, Event, Intent, Orchard, DEFAULT_SEED,
    DEFAULT_STEPS, STEPS_PER_DAY,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn infect_gate(runner: &mut Runner<Orchard>) {
    let block = runner.simulation().block("gate");
    step(runner, &[Intent::Infect { block }]);
}

fn days(runner: &mut Runner<Orchard>, days: u64) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..days * STEPS_PER_DAY {
        events.extend(step(runner, &[]));
    }
    events
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/orchard.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate, part one: in early summer the blight spreads from the gate to every block in three
/// simulated days, and only along the links.
#[test]
fn in_summer_a_zone_spreads_across_the_orchard() {
    let mut runner = runner(starting_on(3), 5);
    infect_gate(&mut runner);
    let events = days(&mut runner, 3);
    assert_eq!(runner.simulation().reached(), 6);
    let reached = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::Blight(SpreadEvent::Started { from: Some(_), .. })
            )
        })
        .count();
    // The gate was infected from outside; the other five were reached from a neighbour.
    assert_eq!(reached, 5);
}

/// The gate, part two: winter shortens the days. The clock gets each day's daylight, so at
/// 07:00 the sun is up in midsummer and not in midwinter.
#[test]
fn winter_shortens_the_days_on_the_clock() {
    let summer = runner(starting_on(4), 5);
    let winter = runner(starting_on(10), 5);
    let (mut summer, mut winter) = (summer, winter);
    step(&mut summer, &[]);
    step(&mut winter, &[]);
    assert!(!summer.clock().is_night());
    assert!(winter.clock().is_night());
    let calendar = summer.simulation().calendar();
    let (summer_rise, summer_set) = calendar.daylight(4);
    let (winter_rise, winter_set) = calendar.daylight(10);
    assert!(winter_set - winter_rise < summer_set - summer_rise);
}

#[test]
fn in_midwinter_the_blight_does_not_spread() {
    let mut runner = runner(starting_on(10), 5);
    infect_gate(&mut runner);
    let events = days(&mut runner, 1);
    assert_eq!(runner.simulation().reached(), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, Event::Blight(SpreadEvent::Started { .. }))));
}

#[test]
fn the_pace_follows_the_temperature() {
    let calendar = orchard::calendar();
    // Midsummer afternoon is warm enough for full pace; midwinter night is idle.
    assert_eq!(pace_percent(&calendar, 4, 900), 100);
    assert_eq!(pace_percent(&calendar, 10, 180), 0);
    assert!(pace_percent(&calendar, 1, 900) > 0 && pace_percent(&calendar, 1, 900) < 100);
}

#[test]
fn a_year_reports_each_season_change_once() {
    let mut runner = runner(starting_on(0), 5);
    let seasons: Vec<u16> = days(&mut runner, 12)
        .into_iter()
        .filter_map(|event| match event {
            Event::Season(CalendarEvent::SeasonChanged { season }) => Some(season.0),
            _ => None,
        })
        .collect();
    assert_eq!(seasons, vec![1, 2, 3, 0]);
}

#[test]
fn infecting_a_block_twice_is_ignored() {
    let mut runner = runner(starting_on(3), 5);
    let gate = runner.simulation().block("gate");
    step(&mut runner, &[Intent::Infect { block: gate }]);
    let events = step(&mut runner, &[Intent::Infect { block: gate }]);
    assert_eq!(events, vec![Event::Ignored { block: gate }]);
}

/// Steps a simulation by hand, so a restored copy can run beside the original.
fn drive(
    orchard: &mut Orchard,
    clock: &mut Clock,
    randomness: &mut Streams,
    number: u64,
    intents: &[Intent],
) -> Vec<Event> {
    let mut events = Vec::new();
    let (elapsed_game_minutes, elapsed_minutes) = clock.advance_exactly(&mut Vec::new());
    orchard.step(
        &mut Context {
            clock,
            elapsed_game_minutes,
            elapsed_minutes,
            randomness,
            events: &mut events,
            step_number: number,
            step_seconds: 1.0 / 30.0,
        },
        intents,
    );
    events
}

#[test]
fn a_restored_snapshot_continues_exactly_like_the_one_that_kept_running() {
    let configuration = starting_on(2);
    let mut clock = runner(configuration.clone(), 1).clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = Orchard::create(configuration, &mut randomness);
    let block = kept.block("gate");
    drive(
        &mut kept,
        &mut clock,
        &mut randomness,
        0,
        &[Intent::Infect { block }],
    );
    for number in 1..300 {
        drive(&mut kept, &mut clock, &mut randomness, number, &[]);
    }
    let saved = encode(&kept.snapshot());
    let mut loaded = Orchard::restore(decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 300..20_000 {
        let a = drive(&mut kept, &mut clock, &mut randomness, number, &[]);
        let b = drive(
            &mut loaded,
            &mut loaded_clock,
            &mut loaded_randomness,
            number,
            &[],
        );
        assert_eq!(a, b, "step {number}");
    }
    assert_eq!(hash_of(loaded.world()), hash_of(kept.world()));
}
