// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{Message, Migrations};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A line that was lost: the name it carried, the run it was lost in, and how many members it
/// had in all.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRecord {
    pub name: String,
    pub run: u32,
    pub members: u32,
}

/// The cross-run record. It lives outside any one simulation's state: a run reads it when it
/// starts, the host folds the run's events into it when the run ends, and the next run starts
/// from it. Save it with `seal`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 2)]
pub struct Chronicle {
    runs: u32,
    lost: Vec<LineRecord>,
    tallies: BTreeMap<String, i64>,
    notes: BTreeMap<String, String>,
}

/// The chronicle as version 1 saved it, before notes. Frozen: it stays so a saved version 1 can
/// still be read and converted (decision 0084).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct ChronicleV1 {
    pub runs: u32,
    pub lost: Vec<LineRecord>,
    pub tallies: BTreeMap<String, i64>,
}

/// The conversions that carry a saved chronicle forward to the current version.
pub fn chronicle_migrations() -> Migrations {
    Migrations::new().step(|old: ChronicleV1| Chronicle {
        runs: old.runs,
        lost: old.lost,
        tallies: old.tallies,
        notes: BTreeMap::new(),
    })
}

impl Chronicle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a run and returns its number, counting from 1.
    pub fn begin_run(&mut self) -> u32 {
        self.runs += 1;
        self.runs
    }

    pub fn runs(&self) -> u32 {
        self.runs
    }

    /// Retires a name for good. A name already retired is kept as first recorded. Returns whether
    /// it was newly retired.
    pub fn retire(&mut self, name: &str, run: u32, members: u32) -> bool {
        if self.is_retired(name) {
            return false;
        }
        self.lost.push(LineRecord {
            name: name.to_string(),
            run,
            members,
        });
        true
    }

    pub fn is_retired(&self, name: &str) -> bool {
        self.lost.iter().any(|record| record.name == name)
    }

    /// The lost lines in the order they were lost.
    pub fn lost(&self) -> &[LineRecord] {
        &self.lost
    }

    /// The retired names, in the order they were lost.
    pub fn retired_names(&self) -> Vec<String> {
        self.lost.iter().map(|record| record.name.clone()).collect()
    }

    /// Adds to a named tally, which starts at zero.
    pub fn add(&mut self, key: &str, amount: i64) {
        let tally = self.tallies.entry(key.to_string()).or_insert(0);
        *tally = tally.saturating_add(amount);
    }

    pub fn tally(&self, key: &str) -> i64 {
        self.tallies.get(key).copied().unwrap_or(0)
    }

    /// Keeps a named note, replacing any earlier one under the same key.
    pub fn set_note(&mut self, key: &str, text: &str) {
        self.notes.insert(key.to_string(), text.to_string());
    }

    pub fn note(&self, key: &str) -> Option<&str> {
        self.notes.get(key).map(String::as_str)
    }
}
