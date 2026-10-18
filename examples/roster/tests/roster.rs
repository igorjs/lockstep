// SPDX-License-Identifier: Apache-2.0
use lockstep_meta::{open, seal, Chronicle, ControlEvent, EnvelopeError};
use roster::{
    configuration_from, fixture_hash, fold, run_project, runner, Configuration, Event, Intent,
    Reason, Role, DEFAULT_SEED, DEFAULT_STEPS,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn hire(name: &str) -> Intent {
    Intent::Hire {
        name: name.to_string(),
        role: Role::Planner,
        craft_points: 0,
        reliability_points: 0,
    }
}

fn empty() -> Configuration {
    Configuration {
        retired: Vec::new(),
    }
}

fn hired(events: &[Event], wanted: &str) -> lockstep_core::Handle {
    events
        .iter()
        .find_map(|event| match event {
            Event::Hired { member, name } if name == wanted => Some(*member),
            _ => None,
        })
        .expect("hired")
}

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/roster.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate: the chronicle survives a new project. A name retired in the first project is
/// refused in the second, after the chronicle went through a save.
#[test]
fn a_retired_name_stays_retired_in_the_next_project() {
    let mut first = runner(empty(), 1);
    let events = first.step_once(&[hire("Ada"), hire("Bram")]).events;
    let ada = hired(&events, "Ada");
    let events = first.step_once(&[Intent::Leave { member: ada }]).events;

    let mut chronicle = Chronicle::new();
    let run = chronicle.begin_run();
    fold(&mut chronicle, run, &events);
    let saved = seal(&chronicle);
    let reopened: Chronicle = open(&saved).unwrap();
    assert!(reopened.is_retired("Ada"));
    assert_eq!(reopened.tally("departures"), 1);

    let mut second = runner(configuration_from(&reopened), 2);
    let events = second.step_once(&[hire("Ada"), hire("Cleo")]).events;
    assert!(events.contains(&Event::Refused {
        reason: Reason::NameRetired
    }));
    assert!(events
        .iter()
        .any(|event| matches!(event, Event::Hired { name, .. } if name == "Cleo")));
}

#[test]
fn the_lead_falls_back_to_the_next_hired_and_never_to_the_one_who_left() {
    let mut project = runner(empty(), 1);
    let events = project
        .step_once(&[hire("Ada"), hire("Bram"), hire("Cleo")])
        .events;
    let (ada, bram, cleo) = (
        hired(&events, "Ada"),
        hired(&events, "Bram"),
        hired(&events, "Cleo"),
    );
    assert_eq!(project.simulation().lead(), Some(ada));
    let events = project.step_once(&[Intent::Leave { member: ada }]).events;
    assert!(events.contains(&Event::Control(ControlEvent::Transferred {
        from: ada,
        to: bram
    })));
    assert_eq!(project.simulation().lead(), Some(bram));
    // A member who is not the lead leaving changes nothing about who leads.
    project.step_once(&[Intent::Leave { member: cleo }]);
    assert_eq!(project.simulation().lead(), Some(bram));
    let events = project.step_once(&[Intent::Leave { member: bram }]).events;
    assert!(events.contains(&Event::Control(ControlEvent::Vacant { from: bram })));
    assert_eq!(project.simulation().lead(), None);
}

#[test]
fn a_name_cannot_be_hired_twice_or_after_it_left_in_the_same_project() {
    let mut project = runner(empty(), 1);
    let events = project.step_once(&[hire("Ada"), hire("Ada")]).events;
    assert!(events.contains(&Event::Refused {
        reason: Reason::NameInUse
    }));
    let ada = hired(&events, "Ada");
    project.step_once(&[Intent::Leave { member: ada }]);
    let events = project.step_once(&[hire("Ada")]).events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Reason::NameRetired
        }]
    );
    let events = project.step_once(&[Intent::Leave { member: ada }]).events;
    assert_eq!(
        events,
        vec![Event::Refused {
            reason: Reason::UnknownMember
        }]
    );
}

#[test]
fn hires_spend_the_point_buy_budget_and_no_more() {
    let mut project = runner(empty(), 1);
    let buy = |craft_points, reliability_points| Intent::Hire {
        name: "Ada".to_string(),
        role: Role::Planner,
        craft_points,
        reliability_points,
    };
    // Budget 5: craft costs 1 and reliability costs 2.
    let events = project.step_once(&[buy(1, 2)]).events;
    let ada = hired(&events, "Ada");
    assert_eq!(
        project.simulation().world().ratings.get(ada),
        Some(&vec![4, 6])
    );
    let mut project = runner(empty(), 1);
    assert_eq!(
        project.step_once(&[buy(2, 2)]).events,
        vec![Event::Refused {
            reason: Reason::OverBudget
        }]
    );
    // Craft is at most 8 and a planner starts at 3, so 6 points pass it.
    assert_eq!(
        project.step_once(&[buy(6, 0)]).events,
        vec![Event::Refused {
            reason: Reason::AboveMaximum
        }]
    );
}

#[test]
fn a_damaged_save_is_refused() {
    let mut bytes = seal(&Chronicle::new());
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(open::<Chronicle>(&bytes), Err(EnvelopeError::Corrupt));
}

/// The session hires in both projects, and the second refuses a retired name.
#[test]
fn the_session_hires_refuses_and_hands_over_the_lead_in_both_projects() {
    let (_, first) = run_project(empty(), DEFAULT_SEED, DEFAULT_STEPS);
    let mut chronicle = Chronicle::new();
    fold(&mut chronicle, 1, &first);
    let (_, second) = run_project(
        configuration_from(&chronicle),
        DEFAULT_SEED ^ 1,
        DEFAULT_STEPS,
    );
    for events in [&first, &second] {
        let count =
            |wanted: fn(&Event) -> bool| events.iter().filter(|event| wanted(event)).count();
        assert!(count(|event| matches!(event, Event::Hired { .. })) > 2);
    }
    assert!(second
        .iter()
        .any(|event| matches!(event, Event::Control(ControlEvent::Transferred { .. }))));
    assert!(second.contains(&Event::Refused {
        reason: Reason::NameRetired
    }));
    assert!(chronicle.lost().len() > 2);
}
