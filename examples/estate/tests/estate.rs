// SPDX-License-Identifier: Apache-2.0
use estate::{
    fixture_hash, grounds, reservations, runner, templates, Event, Intent, CLOISTERS, DEFAULT_SEED,
    DEFAULT_STEPS,
};
use lockstep_procgen::chunk_seed;
use lockstep_structures::{Refusal, StructureEvent};
use std::collections::BTreeSet;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_fixture_hash_matches_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../fixtures/estate.hash").trim();
    assert_eq!(
        format!("{:016x}", fixture_hash(DEFAULT_SEED, DEFAULT_STEPS)),
        committed
    );
}

/// The gate, a valley rolled from chunks: the fixed chunks are the cloister, every other chunk
/// is drawn from the garden pool, and a seed rolls the same valley each time.
#[test]
fn the_valley_keeps_its_cloisters_and_draws_the_rest_from_the_pool() {
    let templates = templates();
    let cloister = templates.template_id("cloister").unwrap();
    let garden: Vec<_> = ["lawn", "orchard", "pond", "hedge"]
        .iter()
        .map(|name| templates.template_id(name).unwrap())
        .collect();
    let valley = runner(grounds(), DEFAULT_SEED);
    let layout = &valley.simulation().world().layout;
    for y in 0..layout.height() {
        for x in 0..layout.width() {
            let drawn = layout.template_at(x, y).unwrap();
            if CLOISTERS.contains(&(x, y)) {
                assert_eq!(drawn, cloister);
            } else {
                assert!(garden.contains(&drawn), "chunk ({x}, {y})");
            }
        }
    }
    assert_eq!(reservations(&templates).len(), CLOISTERS.len());
    let again = runner(grounds(), DEFAULT_SEED);
    assert_eq!(layout, &again.simulation().world().layout);
    let other = runner(grounds(), DEFAULT_SEED + 1);
    assert_ne!(layout, &other.simulation().world().layout);
}

/// The gate, passability updates dirty chunks: a barrier raised across a chunk border reports
/// both chunks to redraw and blocks its cells; when it gives way, both are reported again and
/// the cells open.
#[test]
fn a_barrier_blocks_and_a_collapse_opens_and_both_report_the_chunks() {
    let mut runner = runner(grounds(), DEFAULT_SEED);
    // Row 12 from column 30 to 33 is cloister ground and crosses from chunk 0 to chunk 1.
    let raised = runner
        .step_once(&[Intent::Raise {
            x: 30,
            y: 12,
            length: 4,
        }])
        .events;
    assert!(raised.contains(&Event::Redrawn { chunks: vec![0, 1] }));
    let estate = runner.simulation();
    assert!((30..34).all(|x| !estate.world().map.is_passable(estate.cell(x, 12))));
    let worn = runner.step_once(&[Intent::Surge { force: 15 }]).events;
    assert!(!worn
        .iter()
        .any(|event| matches!(event, Event::Structure(StructureEvent::Collapsed { .. }))));
    let collapsed = runner.step_once(&[Intent::Surge { force: 1 }]).events;
    assert!(collapsed.contains(&Event::Redrawn { chunks: vec![0, 1] }));
    let estate = runner.simulation();
    assert!((30..34).all(|x| estate.world().map.is_passable(estate.cell(x, 12))));
    assert!(estate.world().structures.is_empty());
}

#[test]
fn a_barrier_over_a_wall_or_off_the_map_is_refused_and_a_mended_one_lasts() {
    let mut runner = runner(grounds(), DEFAULT_SEED);
    let refused = runner
        .step_once(&[Intent::Raise {
            x: 46,
            y: 12,
            length: 4,
        }])
        .events;
    assert!(refused.iter().any(|event| matches!(
        event,
        Event::Refused {
            reason: Refusal::OffMap(_)
        }
    )));
    runner.step_once(&[Intent::Raise {
        x: 30,
        y: 12,
        length: 2,
    }]);
    runner.step_once(&[Intent::Surge { force: 7 }]);
    runner.step_once(&[Intent::Mend { amount: 100 }]);
    runner.step_once(&[Intent::Surge { force: 7 }]);
    assert_eq!(runner.simulation().world().structures.len(), 1);
}

#[test]
fn a_full_row_of_barrier_closes_the_estate_until_it_gives_way() {
    let mut runner = runner(grounds(), DEFAULT_SEED);
    assert!(runner.simulation().is_open());
    // Column 24 runs the full height of the estate: blocking every open cell of it cuts the
    // west from the east, whatever the garden holds.
    for y in 0..32 {
        runner.step_once(&[Intent::Raise {
            x: 24,
            y,
            length: 1,
        }]);
    }
    // Rows that were walls already refuse the barrier; every other cell of the column is blocked.
    let estate = runner.simulation();
    assert!((0..32).all(|y| !estate.world().map.is_passable(estate.cell(24, y))));
    assert!(!estate.is_open());
    runner.step_once(&[Intent::Surge { force: 100 }]);
    assert!(runner.simulation().is_open());
}

#[test]
fn seeds_of_the_estate_chunks_do_not_collide() {
    let seeds: BTreeSet<u64> = (0..6)
        .flat_map(|x| (0..4).map(move |y| chunk_seed(DEFAULT_SEED, x, y)))
        .collect();
    assert_eq!(seeds.len(), 24);
}

/// The fixture session touches every branch: barriers go up, some are refused, some collapse, and
/// surveys find the estate both open and closed.
#[test]
fn the_session_raises_refuses_collapses_and_surveys() {
    let mut runner = runner(grounds(), DEFAULT_SEED);
    let mut streams = lockstep_core::Streams::new(DEFAULT_SEED ^ 0x0000_6573_7461_7465);
    let (mut built, mut refused, mut collapsed) = (0, 0, 0);
    let mut surveys = BTreeSet::new();
    for _ in 0..DEFAULT_STEPS {
        let intents = estate::script(runner.simulation(), &mut streams);
        for event in runner.step_once(&intents).events {
            match event {
                Event::Structure(StructureEvent::Built { .. }) => built += 1,
                Event::Structure(StructureEvent::Collapsed { .. }) => collapsed += 1,
                Event::Refused { .. } => refused += 1,
                Event::Surveyed { open } => {
                    surveys.insert(open);
                }
                _ => {}
            }
        }
    }
    assert!(
        built > 10 && refused > 5 && collapsed > 5,
        "{built} {refused} {collapsed}"
    );
    assert_eq!(surveys.len(), 2, "surveys should see both open and closed");
}
