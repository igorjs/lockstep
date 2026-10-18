// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{replay, ReplayOutcome};
use rates::{
    fixture_hash, record_fixture, run_fixture, runner, swap_to, Event, Intent, Rates, Reason,
    BROKEN_RATES, DEFAULT_SEED, DEFAULT_STEPS, RISEN_RATES, WITHOUT_FREIGHT,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn quote(service: &str, grams: u32) -> Intent {
    Intent::Quote {
        service: service.to_string(),
        grams,
    }
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/rates.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn every_started_kilogram_is_charged_on_top_of_the_base() {
    let mut desk = runner(1);
    let events = desk
        .step_once(&[
            quote("economy", 1),
            quote("economy", 1000),
            quote("economy", 1001),
        ])
        .events;
    let prices: Vec<i64> = events
        .iter()
        .map(|event| match event {
            Event::Quoted { price_minor, .. } => *price_minor,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(prices, vec![620, 620, 740]);
}

#[test]
fn a_swap_changes_the_next_quote_and_not_the_one_before() {
    let mut desk = runner(1);
    let before = desk.step_once(&[quote("express", 2000)]).events;
    let swapped = desk.step_once(&[swap_to(RISEN_RATES)]).events;
    let after = desk.step_once(&[quote("express", 2000)]).events;
    assert_eq!(
        before,
        vec![Event::Quoted {
            service: "express".into(),
            price_minor: 900 + 2 * 260
        }]
    );
    assert!(matches!(
        swapped.as_slice(),
        [Event::RatesSwapped { rows: 3, .. }]
    ));
    assert_eq!(
        after,
        vec![Event::Quoted {
            service: "express".into(),
            price_minor: 1000 + 2 * 300
        }]
    );
}

#[test]
fn a_broken_table_is_refused_and_the_old_rates_stay() {
    let mut desk = runner(1);
    let digest_before = desk.simulation().desk().rates.digest();
    let events = desk.step_once(&[swap_to(BROKEN_RATES)]).events;
    assert!(matches!(events.as_slice(), [Event::SwapRefused { .. }]));
    assert_eq!(desk.simulation().desk().rates.digest(), digest_before);
    assert_eq!(desk.simulation().desk().swaps_refused, 1);
    // The refusal also covers text that is not a table, and a table the desk does not have.
    let events = desk
        .step_once(&[
            swap_to("not json"),
            Intent::SwapRates(lockstep_tables::TableSwap {
                table: "other".into(),
                json: RISEN_RATES.into(),
            }),
        ])
        .events;
    assert!(events
        .iter()
        .all(|event| matches!(event, Event::SwapRefused { .. })));
    assert_eq!(desk.simulation().desk().rates.digest(), digest_before);
}

#[test]
fn the_same_rates_in_another_order_change_nothing() {
    let mut desk = runner(1);
    desk.step_once(&[swap_to(RISEN_RATES)]);
    let hash = desk.hash();
    // The same content as RISEN_RATES, listed in yet another order.
    let reordered = r#"{"rows":[
      {"name":"economy","base_minor":550,"per_kilogram_minor":130},
      {"name":"freight","base_minor":2200,"per_kilogram_minor":90},
      {"name":"express","base_minor":1000,"per_kilogram_minor":300}]}"#;
    let events = desk.step_once(&[swap_to(reordered)]).events;
    assert_eq!(events, vec![Event::RatesUnchanged]);
    assert_eq!(desk.simulation().desk().swaps_applied, 1);
    // Only the step counter moved: the rates and counters are as they were.
    let rates_after = desk.simulation().desk().rates.digest();
    assert_eq!(
        rates_after,
        lockstep_tables::Table::<rates::Rate>::from_json(RISEN_RATES)
            .unwrap()
            .digest()
    );
    assert_ne!(desk.hash(), hash, "the step counter is part of the hash");
}

#[test]
fn a_service_dropped_by_a_swap_is_refused_afterwards() {
    let mut desk = runner(1);
    desk.step_once(&[swap_to(WITHOUT_FREIGHT)]);
    let events = desk.step_once(&[quote("freight", 5000)]).events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Reason::UnknownService
        }]
    );
}

#[test]
fn the_scripted_session_swaps_four_times_and_refuses_one() {
    let session = run_fixture(DEFAULT_SEED, DEFAULT_STEPS);
    let desk = Rates::desk(session.simulation());
    // The rise and the dropped service apply; the broken table is refused; the repeat changes
    // nothing.
    assert_eq!(desk.swaps_applied, 2);
    assert_eq!(desk.swaps_refused, 1);
    assert!(desk.quotes > 0 && desk.refusals > 0 && desk.revenue_minor > 0);
}

#[test]
fn a_recording_of_the_session_replays_every_swap_at_its_step() {
    let recording = record_fixture(DEFAULT_SEED, DEFAULT_STEPS, 250, true);
    let swaps = recording
        .steps
        .iter()
        .flat_map(|step| step.intents.iter())
        .filter(|intent| matches!(intent, Intent::SwapRates(_)))
        .count();
    assert_eq!(swaps, 4);
    let (runner, outcome) = replay::<Rates>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(runner.hash(), fixture_hash(DEFAULT_SEED, DEFAULT_STEPS));
}
