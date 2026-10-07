// SPDX-License-Identifier: Apache-2.0
use crate::common::{whole, Crew};
use lockstep_core::{Column, StableVector};
use lockstep_progression::{why_not, ProgressEvent, Refusal};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn triggers_earn_points_and_nodes_spend_them() {
    let mut crew = Crew::new(0);
    assert_eq!(
        crew.unlock("basics"),
        Err(Refusal::Points { needed: 1, have: 0 })
    );
    crew.award("shift").unwrap();
    crew.award("incident").unwrap();
    assert_eq!(crew.points(), 4);
    crew.unlock("basics").unwrap();
    assert_eq!(crew.points(), 3);
    assert_eq!(crew.unlock("basics"), Err(Refusal::AlreadyTaken));
    let basics = crew.node("basics");
    assert!(crew.events.contains(&ProgressEvent::Unlocked {
        owner: crew.who,
        node: basics
    }));
}

#[test]
fn requirements_come_first_and_hold_their_dependants_in_place() {
    let mut crew = Crew::new(20);
    let basics = crew.node("basics");
    assert_eq!(crew.unlock("electrical"), Err(Refusal::Requires(basics)));
    crew.unlock("basics").unwrap();
    crew.unlock("electrical").unwrap();
    let electrical = crew.node("electrical");
    assert_eq!(crew.refund("basics"), Err(Refusal::RequiredBy(electrical)));
    assert_eq!(crew.refund("mechanical"), Err(Refusal::NotTaken));
}

#[test]
fn a_refund_returns_its_percent_rounded_down_and_removes_modifiers() {
    let mut crew = Crew::new(20);
    crew.unlock("basics").unwrap();
    crew.unlock("electrical").unwrap();
    assert_eq!(crew.value("repair").1, whole(110));
    let before = crew.points();
    // Electrical cost 2 at 50 percent: 1 back.
    assert_eq!(crew.refund("electrical"), Ok(1));
    assert_eq!(crew.points(), before + 1);
    assert_eq!(crew.value("repair").1, whole(100));
    // Diagnostics cost 3 at 100 percent: all 3 back. Basics cost 1 at 50 percent: 0.
    crew.unlock("diagnostics").unwrap();
    assert_eq!(crew.refund("diagnostics"), Ok(3));
    assert_eq!(crew.refund("basics"), Ok(0));
}

#[test]
fn why_not_names_the_first_reason_and_an_owner_not_enrolled_is_refused() {
    let crew = Crew::new(0);
    let senior = crew.node("senior");
    let diagnostics = crew.node("diagnostics");
    assert_eq!(
        why_not(
            &crew.progress,
            crew.who,
            senior,
            &crew.graph,
            &crew.attributes
        ),
        Some(Refusal::Requires(diagnostics))
    );
    // A second slot: a fresh store's first handle would be the technician's own.
    let mut store = StableVector::new();
    store.insert(());
    let stranger = store.insert(());
    assert_eq!(
        why_not(
            &crew.progress,
            stranger,
            senior,
            &crew.graph,
            &crew.attributes
        ),
        Some(Refusal::UnknownOwner)
    );
    let mut empty: Column<lockstep_progression::Progress> = Column::new();
    let trigger = crew.graph.trigger_id("shift").unwrap();
    assert_eq!(
        lockstep_progression::award(&mut empty, crew.who, trigger, &crew.graph, &mut Vec::new()),
        Err(Refusal::UnknownOwner)
    );
}

#[test]
fn progress_saves_and_loads_to_an_equal_one() {
    let mut crew = Crew::new(20);
    crew.unlock("basics").unwrap();
    crew.unlock("electrical").unwrap();
    let saved = lockstep_core::encode(&crew.progress);
    let loaded: Column<lockstep_progression::Progress> = lockstep_core::decode(&saved).unwrap();
    assert_eq!(loaded, crew.progress);
}

#[test]
fn why_not_checks_exclusions_then_gates_then_points() {
    let mut crew = Crew::new(4);
    crew.unlock("basics").unwrap();
    crew.unlock("diagnostics").unwrap();
    // No points left and only 10 experience: the gate is named before the points.
    crew.set("experience", 10);
    let senior = crew.node("senior");
    let experience = crew.registry.id("experience").unwrap();
    assert_eq!(
        why_not(
            &crew.progress,
            crew.who,
            senior,
            &crew.graph,
            &crew.attributes
        ),
        Some(Refusal::Gate(experience))
    );
    crew.set("experience", 60);
    assert_eq!(
        why_not(
            &crew.progress,
            crew.who,
            senior,
            &crew.graph,
            &crew.attributes
        ),
        Some(Refusal::Points { needed: 3, have: 0 })
    );
    // With electrical refused by mechanical and no points either, the exclusion comes first.
    let mut other = Crew::new(3);
    other.unlock("basics").unwrap();
    other.unlock("mechanical").unwrap();
    let (electrical, mechanical) = (other.node("electrical"), other.node("mechanical"));
    assert_eq!(
        why_not(
            &other.progress,
            other.who,
            electrical,
            &other.graph,
            &other.attributes
        ),
        Some(Refusal::ExcludedBy(mechanical))
    );
}
