// SPDX-License-Identifier: Apache-2.0
use crate::kinds::{GroupId, Kinds, Relation, RelationId};
use lockstep_core::math::Fixed32;
use lockstep_core::{Handle, Message};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Raw 16.16 game minutes in a day, times the 16.16 scale of a rate: the divisor that turns a
/// rate per day times raw minutes into raw points.
const DAY_SCALE: i128 = 1_440 * 65_536;

/// Who a standing is toward: another entity, or a whole group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Target {
    Entity(Handle),
    Group(GroupId),
}

/// One standing: its value at the last change, and the exact game minutes since, from which the
/// value now is computed afresh each tick, so decay never drifts with the step rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub value: Fixed32,
    base: Fixed32,
    /// Raw 16.16 game minutes since the last change.
    elapsed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Key {
    relation: RelationId,
    from: Handle,
    to: Target,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum RelationEvent {
    /// A standing passed a threshold: downward when it fell below it, upward when it reached it.
    Crossed {
        relation: RelationId,
        from: Handle,
        to: Target,
        threshold: String,
        upward: bool,
    },
}

/// Every standing, directional: how `from` stands toward `to`, which says nothing about how
/// `to` stands toward `from`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relations {
    standings: BTreeMap<Key, Standing>,
}

impl Relations {
    pub fn new() -> Self {
        Self::default()
    }

    /// How `from` stands toward `to`: the starting value until something changes it. An unknown
    /// relation is zero.
    pub fn get(&self, kinds: &Kinds, relation: RelationId, from: Handle, to: Target) -> Fixed32 {
        let Some(definition) = kinds.relation(relation) else {
            return Fixed32::ZERO;
        };
        self.standings
            .get(&Key { relation, from, to })
            .map_or(definition.starting, |standing| standing.value)
    }

    /// How `from` stands toward an entity all told: the pair standing plus, for each group the
    /// entity belongs to, how far the standing toward that group has moved from the starting
    /// value, held within the relation's bounds. An untouched group adds nothing.
    pub fn toward(
        &self,
        kinds: &Kinds,
        relation: RelationId,
        from: Handle,
        to: Handle,
        groups_of_to: &[GroupId],
    ) -> Fixed32 {
        let Some(definition) = kinds.relation(relation) else {
            return Fixed32::ZERO;
        };
        let mut total = self.get(kinds, relation, from, Target::Entity(to)).raw() as i64;
        for group in groups_of_to {
            let group = self.get(kinds, relation, from, Target::Group(*group)).raw() as i64;
            total += group - definition.starting.raw() as i64;
        }
        let clamped = total.clamp(
            definition.minimum.raw() as i64,
            definition.maximum.raw() as i64,
        );
        Fixed32::from_raw(clamped as i32)
    }

    /// Changes a standing by `delta`, held within the relation's bounds, and restarts its decay
    /// from the new value. Emits each threshold crossed, in the order the value passes them.
    pub fn change(
        &mut self,
        kinds: &Kinds,
        relation: RelationId,
        from: Handle,
        to: Target,
        delta: Fixed32,
        events: &mut Vec<RelationEvent>,
    ) {
        let Some(definition) = kinds.relation(relation) else {
            return;
        };
        let key = Key { relation, from, to };
        let before = self
            .standings
            .get(&key)
            .map_or(definition.starting, |standing| standing.value);
        let target = before.raw() as i64 + delta.raw() as i64;
        let after = Fixed32::from_raw(target.clamp(
            definition.minimum.raw() as i64,
            definition.maximum.raw() as i64,
        ) as i32);
        self.standings.insert(
            key,
            Standing {
                value: after,
                base: after,
                elapsed: 0,
            },
        );
        crossed(definition, key, before, after, events);
    }

    /// Lets `minutes` of game time pass: every standing that has been changed drifts toward its
    /// relation's rest by its decay a day, computed from the exact minutes since its last change,
    /// and emits the thresholds it crosses. A standing nobody has changed stays at the starting
    /// value: drift begins with the first change.
    pub fn tick(&mut self, kinds: &Kinds, minutes: Fixed32, events: &mut Vec<RelationEvent>) {
        let minutes = minutes.raw().max(0) as u64;
        for (key, standing) in self.standings.iter_mut() {
            let Some(definition) = kinds.relation(key.relation) else {
                continue;
            };
            if definition.decay_per_day == Fixed32::ZERO {
                continue;
            }
            standing.elapsed = standing.elapsed.saturating_add(minutes);
            let drift = (definition.decay_per_day.raw() as i128 * standing.elapsed as i128
                / DAY_SCALE)
                .min(i32::MAX as i128) as i64;
            let (base, rest) = (standing.base.raw() as i64, definition.rest.raw() as i64);
            let now = if base > rest {
                (base - drift).max(rest)
            } else {
                (base + drift).min(rest)
            };
            let before = standing.value;
            standing.value = Fixed32::from_raw(now as i32);
            crossed(definition, *key, before, standing.value, events);
        }
    }

    /// Drops every standing from or toward an entity, for when it leaves the simulation.
    pub fn forget(&mut self, who: Handle) {
        self.standings
            .retain(|key, _| key.from != who && key.to != Target::Entity(who));
    }
}

fn crossed(
    definition: &Relation,
    key: Key,
    before: Fixed32,
    after: Fixed32,
    events: &mut Vec<RelationEvent>,
) {
    let event = |threshold: &str, upward| RelationEvent::Crossed {
        relation: key.relation,
        from: key.from,
        to: key.to,
        threshold: threshold.to_string(),
        upward,
    };
    if after > before {
        for threshold in &definition.thresholds {
            if before < threshold.at && threshold.at <= after {
                events.push(event(&threshold.name, true));
            }
        }
    } else if after < before {
        // Highest first, keeping definition order among equal ones, as attributes do.
        let mut falling: Vec<&crate::kinds::Threshold> = definition
            .thresholds
            .iter()
            .filter(|threshold| after < threshold.at && threshold.at <= before)
            .collect();
        falling.sort_by_key(|threshold| std::cmp::Reverse(threshold.at));
        for threshold in falling {
            events.push(event(&threshold.name, false));
        }
    }
}
