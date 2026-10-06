// SPDX-License-Identifier: Apache-2.0
//! The timeline: an append-only log of events, indexed by entity and kind, rebuilt from any
//! recording. It is a projection for journals, achievements and inspectors, never the source of
//! truth, and it is not part of the state hash.

use crate::clock::Clock;
use crate::message::{Indexable, Message};
use crate::replay::{replay_with, Recording, ReplayError, ReplayOutcome};
use crate::simulation::Simulation;
use crate::store::Handle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry<E> {
    /// The step the event happened in.
    pub step: u64,
    pub day: u32,
    /// The whole minute of the day when the step ended.
    pub minute_of_day: u16,
    /// The event's place in the log: entries are in sequence order.
    pub sequence: u32,
    pub event: E,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Timeline<E: Message + Indexable> {
    entries: Vec<Entry<E>>,
    by_entity: BTreeMap<Handle, Vec<u32>>,
    by_kind: BTreeMap<u16, Vec<u32>>,
    next_sequence: u32,
}

impl<E: Message + Indexable> Default for Timeline<E> {
    fn default() -> Self {
        Timeline {
            entries: Vec::new(),
            by_entity: BTreeMap::new(),
            by_kind: BTreeMap::new(),
            next_sequence: 0,
        }
    }
}

impl<E: Message + Indexable> Timeline<E> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one step's events, in order. Call it once per step, with steps in order.
    pub fn append(&mut self, step: u64, clock: &Clock, events: &[E]) {
        // `between` searches by step, so the order is checked in every build.
        assert!(
            self.entries.last().is_none_or(|last| last.step <= step),
            "steps are appended in order"
        );
        for event in events {
            let entry = Entry {
                step,
                day: clock.day(),
                minute_of_day: clock.whole_minute(),
                sequence: self.next_sequence,
                event: event.clone(),
            };
            self.next_sequence = self.next_sequence.wrapping_add(1);
            index(
                &mut self.by_entity,
                &mut self.by_kind,
                &entry.event,
                self.entries.len() as u32,
            );
            self.entries.push(entry);
        }
    }

    pub fn entries(&self) -> &[Entry<E>] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry that mentions `who`, in order.
    pub fn for_entity(&self, who: Handle) -> impl Iterator<Item = &Entry<E>> {
        self.positions(self.by_entity.get(&who))
    }

    /// Every entry of one kind (an enum's variant index), in order.
    pub fn of_kind(&self, kind: u16) -> impl Iterator<Item = &Entry<E>> {
        self.positions(self.by_kind.get(&kind))
    }

    /// The entries from `from_step` up to but not including `to_step`.
    pub fn between(&self, from_step: u64, to_step: u64) -> &[Entry<E>] {
        let start = self.entries.partition_point(|entry| entry.step < from_step);
        let end = self.entries.partition_point(|entry| entry.step < to_step);
        &self.entries[start..end.max(start)]
    }

    /// The most recent entry whose event matches.
    pub fn last<F: Fn(&E) -> bool>(&self, predicate: F) -> Option<&Entry<E>> {
        self.entries
            .iter()
            .rev()
            .find(|entry| predicate(&entry.event))
    }

    /// Rolls the detail before `before_step` into what `summariser` returns for it, for example
    /// one entry per account per day. The summary goes first, in the order returned, and keeps
    /// the sequence numbers the summariser gives it. Panics if the summary is not in step order or
    /// reaches past the first entry kept, because `between` searches by step.
    pub fn compact(&mut self, before_step: u64, summariser: impl Fn(&[Entry<E>]) -> Vec<Entry<E>>) {
        let cut = self
            .entries
            .partition_point(|entry| entry.step < before_step);
        let summary = summariser(&self.entries[..cut]);
        let rest = self.entries.split_off(cut);
        assert!(
            summary.windows(2).all(|pair| pair[0].step <= pair[1].step),
            "a summary is in step order"
        );
        if let (Some(last), Some(first_kept)) = (summary.last(), rest.first()) {
            assert!(
                last.step <= first_kept.step,
                "a summary does not reach past the entries it keeps"
            );
        }
        self.entries = summary;
        self.entries.extend(rest);
        self.by_entity.clear();
        self.by_kind.clear();
        for (position, entry) in self.entries.iter().enumerate() {
            index(
                &mut self.by_entity,
                &mut self.by_kind,
                &entry.event,
                position as u32,
            );
        }
    }

    /// The timeline of a recorded session, built by replaying it, with how the replay went. A
    /// `Diverged` outcome means this build no longer plays the session the same way, and the
    /// timeline stops at the first checkpoint that differs.
    pub fn rebuild_from<S: Simulation<Event = E>>(
        recording: &Recording<S::Intent>,
    ) -> Result<(Self, ReplayOutcome), ReplayError> {
        let mut timeline = Timeline::new();
        let (_, outcome) = replay_with::<S>(recording, &mut |runner, events| {
            timeline.append(runner.step_number() - 1, runner.clock(), events);
        })?;
        Ok((timeline, outcome))
    }

    fn positions<'a>(
        &'a self,
        positions: Option<&'a Vec<u32>>,
    ) -> impl Iterator<Item = &'a Entry<E>> {
        positions
            .into_iter()
            .flatten()
            .map(|position| &self.entries[*position as usize])
    }
}

/// Records an event's position under every entity it mentions (once each) and under its kind.
fn index<E: Indexable>(
    by_entity: &mut BTreeMap<Handle, Vec<u32>>,
    by_kind: &mut BTreeMap<u16, Vec<u32>>,
    event: &E,
    position: u32,
) {
    let mut handles = Vec::new();
    event.handles(&mut handles);
    handles.sort_unstable();
    handles.dedup();
    for handle in handles {
        by_entity.entry(handle).or_default().push(position);
    }
    by_kind.entry(event.kind()).or_default().push(position);
}
