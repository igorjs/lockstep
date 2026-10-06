// SPDX-License-Identifier: Apache-2.0
use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Noted, Start};
use lockstep_core::{Clock, Entry, Handle, Indexable, Message, Recorder, StableVector, Timeline};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
enum Happened {
    Paid {
        from: Handle,
        to: Handle,
        amount: i64,
    },
    Opened {
        account: Handle,
    },
    /// What compaction leaves: how many of one kind an account had.
    Summary {
        account: Handle,
        kind: u16,
        count: u32,
    },
}

fn clock() -> Clock {
    Clock::new(clock_configuration(), 1.0 / 30.0)
}

fn accounts() -> (Handle, Handle, Handle) {
    let mut store = StableVector::new();
    (store.insert(()), store.insert(()), store.insert(()))
}

/// Opens three accounts at step 0, then pays around the ring once a step.
fn sample() -> (Timeline<Happened>, [Handle; 3]) {
    let (a, b, c) = accounts();
    let ring = [a, b, c];
    let mut timeline = Timeline::new();
    let mut clock = clock();
    let opened: Vec<_> = ring
        .iter()
        .map(|account| Happened::Opened { account: *account })
        .collect();
    timeline.append(0, &clock, &opened);
    for step in 1..=30u64 {
        clock.advance(&mut Vec::new());
        let from = ring[step as usize % 3];
        let to = ring[(step as usize + 1) % 3];
        timeline.append(
            step,
            &clock,
            &[Happened::Paid {
                from,
                to,
                amount: step as i64,
            }],
        );
    }
    (timeline, ring)
}

#[test]
fn entries_are_found_by_entity_kind_step_range_and_predicate() {
    let (timeline, [a, b, c]) = sample();
    assert_eq!(timeline.len(), 33);
    assert!(timeline
        .entries()
        .windows(2)
        .all(|pair| pair[0].sequence < pair[1].sequence));
    // Every account opens once and appears in two of every three payments.
    for account in [a, b, c] {
        assert_eq!(timeline.for_entity(account).count(), 1 + 20);
    }
    let paid_kind = Happened::Paid {
        from: a,
        to: b,
        amount: 0,
    }
    .kind();
    assert_eq!(timeline.of_kind(paid_kind).count(), 30);
    assert_eq!(timeline.of_kind(1).count(), 3);
    let window: Vec<u64> = timeline
        .between(10, 13)
        .iter()
        .map(|entry| entry.step)
        .collect();
    assert_eq!(window, [10, 11, 12]);
    assert!(timeline.between(40, 50).is_empty());
    assert!(timeline.between(13, 10).is_empty());
    let last_from_a = timeline
        .last(|event| matches!(event, Happened::Paid { from, .. } if *from == a))
        .unwrap();
    assert_eq!(last_from_a.step, 30);
}

#[test]
fn compaction_keeps_the_counts_it_summarises() {
    let (mut timeline, ring) = sample();
    let count_before = |timeline: &Timeline<Happened>, account: Handle| -> u32 {
        timeline
            .for_entity(account)
            .map(|entry| match entry.event {
                Happened::Summary { count, .. } => count,
                _ => 1,
            })
            .sum()
    };
    let before: Vec<u32> = ring
        .iter()
        .map(|account| count_before(&timeline, *account))
        .collect();
    // Roll everything before step 20 into one summary per account and kind.
    timeline.compact(20, |old: &[Entry<Happened>]| {
        let mut counts: std::collections::BTreeMap<(Handle, u16), u32> = Default::default();
        for entry in old {
            let mut handles = Vec::new();
            entry.event.handles(&mut handles);
            handles.sort_unstable();
            handles.dedup();
            for account in handles {
                *counts.entry((account, entry.event.kind())).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .enumerate()
            .map(|(index, ((account, kind), count))| Entry {
                step: 0,
                day: 0,
                minute_of_day: 0,
                sequence: index as u32,
                event: Happened::Summary {
                    account,
                    kind,
                    count,
                },
            })
            .collect()
    });
    let after: Vec<u32> = ring
        .iter()
        .map(|account| count_before(&timeline, *account))
        .collect();
    assert_eq!(after, before);
    assert_eq!(timeline.between(20, 31).len(), 11, "recent detail stays");
    assert!(timeline.len() < 33);
}

#[test]
fn a_timeline_rebuilt_from_a_recording_equals_the_live_one() {
    let mut recorder = Recorder::<Honest>::new(
        "tally",
        Start { total: 0 },
        11,
        step_configuration(),
        clock_configuration(),
        100,
    );
    let mut live: Timeline<Noted> = Timeline::new();
    for step in 0..2_000 {
        if step == 700 {
            recorder.set_clock_multiplier(20.0);
        }
        let advanced = recorder.step_once(&[Add(step)]);
        let runner = recorder.runner();
        live.append(runner.step_number() - 1, runner.clock(), &advanced.events);
    }
    assert!(
        live.len() > 20,
        "the bonus roll fires about 3 percent of the time"
    );
    let rebuilt = Timeline::rebuild_from::<Honest>(recorder.recording()).unwrap();
    assert_eq!(rebuilt, live);
}
