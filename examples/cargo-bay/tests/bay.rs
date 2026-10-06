// SPDX-License-Identifier: Apache-2.0
use cargo_bay::{
    bay, fixture_hash, order, runner, CargoBay, Event, Intent, DEFAULT_SEED, DEFAULT_STEPS,
    FRESH_MEAL, SPOILED_MEAL,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{hash_of, Clock, Context, Handle, Runner, Simulation, Streams};
use lockstep_inventory::{InventoryEvent, KindId, Refusal, SlotId};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn kind(runner: &Runner<CargoBay>, name: &str) -> KindId {
    runner.simulation().catalogue().kind_id(name).unwrap()
}

/// The first crew member, and the rack, cold store and airlock locker.
fn places(runner: &Runner<CargoBay>) -> (Handle, Handle, Handle, Handle) {
    let (crew, stores) = (runner.simulation().crew(), runner.simulation().stores());
    (crew[0], stores[0], stores[1], stores[2])
}

fn deliver(runner: &mut Runner<CargoBay>, name: &str, count: u16, to: Handle) -> Handle {
    let kind = kind(runner, name);
    let events = runner
        .step_once(&[Intent::Deliver { kind, count, to }])
        .events;
    match events[0] {
        Event::Delivered { item, .. } => item,
        ref other => panic!("not delivered: {other:?}"),
    }
}

fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/cargo-bay.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

#[test]
fn the_scripted_session_eats_spoils_faults_repairs_and_cuts_power() {
    let mut runner = runner(bay(), DEFAULT_SEED, 30);
    let mut script = Streams::new(DEFAULT_SEED ^ 0x0063_6172_676f);
    let mut events = Vec::new();
    for _ in 0..DEFAULT_STEPS {
        let intents: Vec<Intent> = order(runner.simulation(), &mut script)
            .into_iter()
            .collect();
        events.extend(runner.step_once(&intents).events);
    }
    let count = |test: &dyn Fn(&Event) -> bool| events.iter().filter(|event| test(event)).count();
    assert!(count(&|event| matches!(event, Event::Ate { spoiled: true, .. })) > 0);
    assert!(count(&|event| matches!(event, Event::Ate { spoiled: false, .. })) > 0);
    assert!(count(&|event| matches!(event, Event::Inventory(InventoryEvent::Spoiled { .. }))) > 0);
    assert!(count(&|event| matches!(event, Event::Faulted { .. })) > 0);
    assert!(count(&|event| matches!(event, Event::Repaired { .. })) > 0);
    assert!(
        count(&|event| matches!(
            event,
            Event::Refused {
                reason: Refusal::Bound
            }
        )) > 0
    );
    assert!(count(&|event| matches!(event, Event::Powered { .. })) > 0);
}

/// The gate: rations on the rack and in the cold store, with the cold store losing power after
/// two game hours. At 30 and at 60 steps a second they spoil on the same game minute.
#[test]
fn spoilage_does_not_depend_on_the_step_rate() {
    let run = |rate: u32| {
        let mut runner = runner(bay(), 1, rate);
        let (_, rack, cold, _) = places(&runner);
        let ration = kind(&runner, "ration");
        let events = runner
            .step_once(&[
                Intent::Deliver {
                    kind: ration,
                    count: 2,
                    to: rack,
                },
                Intent::Deliver {
                    kind: ration,
                    count: 2,
                    to: cold,
                },
            ])
            .events;
        let (racked, chilled) = match events[..] {
            [Event::Delivered { item: racked, .. }, Event::Delivered { item: chilled, .. }] => {
                (racked, chilled)
            }
            _ => panic!("not delivered: {events:?}"),
        };
        let mut spoiled = Vec::new();
        // One step has run; 600 game minutes in all.
        for step in 1..(600 * rate as u64) {
            let intents = if step == 120 * rate as u64 {
                vec![Intent::Power {
                    store: cold,
                    degrees: 22,
                }]
            } else {
                Vec::new()
            };
            for event in runner.step_once(&intents).events {
                if let Event::Inventory(InventoryEvent::Spoiled { item }) = event {
                    // Steps run so far, counting this one.
                    spoiled.push((item == racked, item == chilled, step + 1));
                }
            }
        }
        // Steps to whole game minutes: one game minute a real second.
        spoiled
            .into_iter()
            .map(|(on_rack, in_cold, steps)| {
                assert_eq!(steps % rate as u64, 0, "spoils on a whole game minute");
                (on_rack, in_cold, steps / rate as u64)
            })
            .collect::<Vec<_>>()
    };
    let slow = run(30);
    assert_eq!(slow, run(60));
    // 360 minutes on the rack at 100 percent. The cold store: 120 minutes at 20 percent count as
    // 24, and the other 336 at 100 percent come at minute 456.
    assert_eq!(slow, vec![(true, false, 360), (false, true, 456)]);
}

#[test]
fn a_fresh_ration_nourishes_and_a_spoiled_one_makes_the_crew_sick() {
    let mut runner = runner(bay(), 1, 30);
    let (who, rack, _, _) = places(&runner);
    let rations = deliver(&mut runner, "ration", 2, rack);
    let start = runner.simulation().value(who, "nourishment").unwrap().0;
    let events = runner
        .step_once(&[Intent::Eat { who, item: rations }])
        .events;
    assert!(events.contains(&Event::Ate {
        who,
        spoiled: false
    }));
    let after = runner.simulation().value(who, "nourishment").unwrap().0;
    assert_eq!(after, start + whole(FRESH_MEAL));
    // 360 game minutes on the rack spoil the other.
    for _ in 0..360 * 30 {
        runner.step_once(&[]);
    }
    let events = runner
        .step_once(&[Intent::Eat { who, item: rations }])
        .events;
    assert!(events.contains(&Event::Ate { who, spoiled: true }));
    assert_eq!(
        runner.simulation().value(who, "nourishment").unwrap().0,
        after + whole(SPOILED_MEAL)
    );
    assert!(
        runner
            .simulation()
            .world()
            .inventory
            .item(rations)
            .is_none(),
        "both eaten"
    );
    let water = deliver(&mut runner, "water", 1, rack);
    let events = runner.step_once(&[Intent::Eat { who, item: water }]).events;
    assert_eq!(events, vec![Event::NotFood { item: water }]);
}

#[test]
fn spoiled_meals_cross_the_hungry_threshold() {
    let mut runner = runner(bay(), 1, 30);
    let (who, rack, _, _) = places(&runner);
    let rations = deliver(&mut runner, "ration", 6, rack);
    for _ in 0..360 * 30 {
        runner.step_once(&[]);
    }
    // 60, then four spoiled meals: 50, 40, 30, then 20, below hungry at 30.
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(
            runner
                .step_once(&[Intent::Eat { who, item: rations }])
                .events,
        );
    }
    assert!(events.contains(&Event::Crossed {
        who,
        attribute: "nourishment".into(),
        threshold: "hungry".into(),
        upward: false,
    }));
}

#[test]
fn gear_taken_off_can_be_stowed_again() {
    let mut runner = runner(bay(), 1, 30);
    let (who, rack, cold, _) = places(&runner);
    let drill = deliver(&mut runner, "drill", 1, rack);
    runner.step_once(&[Intent::Equip { who, item: drill }]);
    runner.step_once(&[Intent::Unequip {
        who,
        slot: SlotId(1),
    }]);
    let events = runner
        .step_once(&[Intent::Move {
            item: drill,
            to: cold,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Moved {
            item: drill,
            into: cold
        }]
    );
}

#[test]
fn a_faulty_seal_binds_a_suit_until_it_is_repaired() {
    let mut runner = runner(bay(), 1, 30);
    let (who, rack, _, _) = places(&runner);
    let suit = deliver(&mut runner, "suit", 1, rack);
    runner.step_once(&[Intent::Equip { who, item: suit }]);
    assert_eq!(
        runner.simulation().value(who, "oxygen").unwrap().1,
        whole(150)
    );
    runner.step_once(&[Intent::Fault { item: suit }]);
    assert_eq!(
        runner.simulation().value(who, "oxygen").unwrap().1,
        whole(120)
    );
    let slot = SlotId(0);
    let events = runner.step_once(&[Intent::Unequip { who, slot }]).events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Refusal::Bound
        }]
    );
    runner.step_once(&[Intent::Repair { item: suit }]);
    let events = runner.step_once(&[Intent::Unequip { who, slot }]).events;
    assert_eq!(events, vec![Event::Unequipped { who, item: suit }]);
    assert_eq!(
        runner.simulation().value(who, "oxygen").unwrap().1,
        whole(100)
    );
}

#[test]
fn a_refused_delivery_leaves_nothing_behind() {
    let mut runner = runner(bay(), 1, 30);
    let (_, _, _, locker) = places(&runner);
    // The airlock locker holds 40: three suits are 36, a fourth is too heavy.
    for _ in 0..3 {
        deliver(&mut runner, "suit", 1, locker);
    }
    let before = runner.simulation().world().inventory.items().count();
    let suit = kind(&runner, "suit");
    let events = runner
        .step_once(&[Intent::Deliver {
            kind: suit,
            count: 1,
            to: locker,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Refusal::TooHeavy
        }]
    );
    assert_eq!(
        runner.simulation().world().inventory.items().count(),
        before
    );
    let events = runner
        .step_once(&[Intent::Deliver {
            kind: KindId(99),
            count: 1,
            to: locker,
        }])
        .events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Refusal::Count
        }]
    );
}

/// Steps a simulation outside a runner, the way the runner does.
fn step(
    simulation: &mut CargoBay,
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
    let session = runner(bay(), DEFAULT_SEED, 30);
    let mut clock = session.clock().clone();
    let mut randomness = Streams::new(5);
    let mut kept = CargoBay::create(bay(), &mut Streams::new(5));
    let mut script = Streams::new(77);
    for number in 0..20_000 {
        let intents: Vec<Intent> = order(&kept, &mut script).into_iter().collect();
        step(&mut kept, &mut clock, &mut randomness, &intents, number);
    }
    // Saved and loaded, so a field lost on save would show.
    let saved = lockstep_core::encode(&kept.snapshot());
    let mut loaded = CargoBay::restore(lockstep_core::decode(&saved).unwrap());
    let (mut loaded_clock, mut loaded_randomness) = (clock.clone(), randomness.clone());
    for number in 20_000..40_000 {
        let intents: Vec<Intent> = order(&kept, &mut script).into_iter().collect();
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
