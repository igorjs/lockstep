// SPDX-License-Identifier: Apache-2.0
//! Decision 0035: a save carries the version of its snapshot shape, and an older shape reaches the
//! current one through a chain of conversions, each from one version to the next. A shipped shape
//! stays in the code, frozen, so its bytes can still be read.
//! Alternative rejected: a conversion from every old version straight to the current one. Each new
//! shape would need all of them rewritten, and a missed pair would fail only for the save that
//! needs it.
//! Would change if: a shape can be skipped by a chain (a version with no step to its successor
//! must be an error, never a guess), or loading an old save changes what the next step does.

use lockstep_core::{
    encode, Context, Message, Migrations, Runner, SaveError, Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct TotalOnly {
    total: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 2)]
struct WithSteps {
    total: i64,
    steps: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 3)]
struct WithLabel {
    total: i64,
    steps: u64,
    label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Nothing;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Add(i64);

struct Counter(WithLabel);

impl Simulation for Counter {
    type Intent = Add;
    type Event = Nothing;
    type Snapshot = WithLabel;
    type Configuration = Nothing;

    fn create(_: Nothing, _: &mut Streams) -> Self {
        Counter(WithLabel {
            total: 0,
            steps: 0,
            label: "new".to_string(),
        })
    }

    fn step(&mut self, _: &mut Context<'_, Self>, intents: &[Add]) {
        for Add(amount) in intents {
            self.0.total += amount;
        }
        self.0.steps += 1;
    }

    fn snapshot(&self) -> WithLabel {
        self.0.clone()
    }

    fn restore(snapshot: WithLabel) -> Self {
        Counter(snapshot)
    }
}

fn chain() -> Migrations {
    Migrations::new()
        .step(|old: TotalOnly| WithSteps {
            total: old.total,
            steps: 0,
        })
        .step(|old: WithSteps| WithLabel {
            total: old.total,
            steps: old.steps,
            label: "migrated".to_string(),
        })
}

#[test]
fn an_old_snapshot_walks_the_chain_one_version_at_a_time() {
    let first = encode(&TotalOnly { total: 41 });
    let second = encode(&WithSteps { total: 7, steps: 3 });
    assert_eq!(
        chain().migrate::<WithLabel>(1, &first).unwrap(),
        WithLabel {
            total: 41,
            steps: 0,
            label: "migrated".to_string()
        }
    );
    assert_eq!(
        chain().migrate::<WithLabel>(2, &second).unwrap(),
        WithLabel {
            total: 7,
            steps: 3,
            label: "migrated".to_string()
        }
    );
    let current = WithLabel {
        total: 1,
        steps: 2,
        label: "kept".to_string(),
    };
    assert_eq!(
        chain().migrate::<WithLabel>(3, &encode(&current)).unwrap(),
        current
    );
}

#[test]
fn a_broken_chain_or_a_newer_save_is_an_error() {
    let first = encode(&TotalOnly { total: 1 });
    let only_the_second = Migrations::new().step(|old: WithSteps| WithLabel {
        total: old.total,
        steps: old.steps,
        label: String::new(),
    });
    assert_eq!(
        only_the_second.migrate::<WithLabel>(1, &first),
        Err(SaveError::MissingMigration { from_version: 1 })
    );
    assert_eq!(
        chain().migrate::<WithLabel>(4, &first),
        Err(SaveError::Newer {
            found: 4,
            current: 3
        })
    );
    // Bytes that do not read as the shape the version names are the migration's error.
    assert!(matches!(
        chain().migrate::<WithLabel>(2, &[1, 2, 3]),
        Err(SaveError::Migration {
            from_version: 2,
            ..
        })
    ));
}

#[test]
fn a_runner_loaded_from_an_old_save_steps_like_one_that_never_left() {
    let step = StepConfiguration::default();
    let clock = lockstep_core::ClockConfiguration {
        day_length_real_minutes: 10.0,
        sunrise_minute: 0,
        sunset_minute: 1_440,
        starting_minute: 0,
        starting_day: 0,
    };
    let mut modern = Runner::<Counter>::new(Nothing, 3, step, clock);
    let mut save = modern.save("counter");
    // Write the save as the first shape would have: a total and nothing else.
    save.snapshot_version = 1;
    save.snapshot = encode(&TotalOnly { total: 0 });
    let mut loaded = Runner::<Counter>::from_save(&save, &chain()).unwrap();
    modern.step_once(&[Add(5)]);
    loaded.step_once(&[Add(5)]);
    assert_eq!(loaded.simulation().snapshot().total, 5);
    assert_eq!(loaded.simulation().snapshot().label, "migrated");
    assert_eq!(loaded.simulation().snapshot().steps, 1);
    assert_eq!(loaded.step_number(), modern.step_number());
}

#[test]
#[should_panic(expected = "already has a migration")]
fn two_conversions_from_one_version_are_refused_when_the_chain_is_built() {
    let _ = Migrations::new()
        .step(|old: TotalOnly| WithSteps {
            total: old.total,
            steps: 0,
        })
        .step(|old: TotalOnly| WithSteps {
            total: old.total,
            steps: 1,
        });
}
