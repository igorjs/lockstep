// SPDX-License-Identifier: Apache-2.0
use crate::attribute::{AttributeEvent, Attributes, Modifier, ModifierHandle};
use crate::registry::{AttributeId, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle};
use serde::{Deserialize, Serialize};

/// What an effect is, for stacking, removal and display: Bleeding, Blessed, Fever.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EffectTag(pub String);

impl EffectTag {
    pub fn new(name: &str) -> Self {
        EffectTag(name.to_string())
    }
}

/// What happens when an entity gains an effect whose tag it already has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stacking {
    /// Every application is its own effect: two bleeds drain twice.
    Independent,
    /// The modifier applies once; a new application restarts the timer.
    RefreshDuration,
    /// A new application removes the old one first.
    Replace,
    /// Each application is its own effect, but the tag's total change on the entity is capped per
    /// game day: prayer's hope. Past the cap it gives nothing until the next day.
    DailyBudget { cap_per_day: Fixed32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub attribute: AttributeId,
    /// Held while the effect is active.
    pub modifier: Option<Modifier>,
    /// Applied every tick, scaled by the elapsed game minutes.
    pub per_minute: Option<Fixed32>,
    /// `None` lasts until removed.
    pub remaining_minutes: Option<Fixed32>,
    pub tag: EffectTag,
    pub stacking: Stacking,
}

/// Names one active effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EffectHandle(u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectEvent {
    Applied {
        who: Handle,
        tag: EffectTag,
    },
    /// Its time ran out.
    Expired {
        who: Handle,
        tag: EffectTag,
    },
    /// Removed by `remove_by_tag`, `remove` or a `Replace`.
    Removed {
        who: Handle,
        tag: EffectTag,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Active {
    handle: EffectHandle,
    who: Handle,
    effect: Effect,
    modifier: Option<ModifierHandle>,
    /// Game minutes since it was applied or refreshed, in raw 16.16 units, capped at the duration.
    elapsed: i64,
    /// The drain applied so far, in raw units. Each tick applies the change in the cumulative
    /// total, so splitting the same minutes into more ticks changes nothing.
    applied: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Budget {
    who: Handle,
    tag: EffectTag,
    day: u32,
    /// The magnitude of change given today, in raw units.
    used: i64,
}

/// Every active effect of every entity, in the order they were applied.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effects {
    active: Vec<Active>,
    budgets: Vec<Budget>,
    next_handle: u64,
}

/// Everything an effect touches, passed together.
pub struct EffectContext<'a> {
    pub attributes: &'a mut Column<Attributes>,
    pub registry: &'a Registry,
    pub attribute_events: &'a mut Vec<AttributeEvent>,
    pub effect_events: &'a mut Vec<EffectEvent>,
}

impl Effects {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies an effect to `who`, following its stacking rule. A `RefreshDuration` effect the entity
    /// already has restarts its timer with the new duration and returns the existing handle; it
    /// keeps its attribute, modifier and rate, so a stronger effect needs `Replace`. Give the entity
    /// its attributes first: an effect added before them never gets its modifier.
    pub fn add(
        &mut self,
        who: Handle,
        effect: Effect,
        context: &mut EffectContext<'_>,
    ) -> EffectHandle {
        match effect.stacking {
            Stacking::RefreshDuration => {
                let existing = self.active.iter_mut().find(|active| {
                    active.who == who
                        && active.effect.tag == effect.tag
                        && active.effect.stacking == Stacking::RefreshDuration
                });
                if let Some(active) = existing {
                    active.elapsed = 0;
                    active.applied = 0;
                    active.effect.remaining_minutes = effect.remaining_minutes;
                    context.effect_events.push(EffectEvent::Applied {
                        who,
                        tag: effect.tag,
                    });
                    return active.handle;
                }
            }
            Stacking::Replace => {
                self.remove_matching(
                    |active| active.who == who && active.effect.tag == effect.tag,
                    context,
                );
            }
            Stacking::Independent | Stacking::DailyBudget { .. } => {}
        }
        let handle = EffectHandle(self.next_handle);
        self.next_handle += 1;
        let modifier = match (effect.modifier, context.attributes.get_mut(who)) {
            (Some(modifier), Some(attributes)) => Some(attributes.add_modifier(
                who,
                effect.attribute,
                modifier,
                context.registry,
                context.attribute_events,
            )),
            _ => None,
        };
        context.effect_events.push(EffectEvent::Applied {
            who,
            tag: effect.tag.clone(),
        });
        self.active.push(Active {
            handle,
            who,
            effect,
            modifier,
            elapsed: 0,
            applied: 0,
        });
        handle
    }

    /// Removes one effect. False when it is no longer active.
    pub fn remove(&mut self, handle: EffectHandle, context: &mut EffectContext<'_>) -> bool {
        self.remove_matching(|active| active.handle == handle, context) > 0
    }

    /// Removes every effect with this tag from `who`, as a bandage removes all Bleeding. Returns how
    /// many it removed.
    pub fn remove_by_tag(
        &mut self,
        who: Handle,
        tag: &EffectTag,
        context: &mut EffectContext<'_>,
    ) -> usize {
        self.remove_matching(
            |active| active.who == who && &active.effect.tag == tag,
            context,
        )
    }

    /// Forgets every effect and budget of an entity that no longer exists, without events.
    pub fn remove_entity(&mut self, who: Handle) {
        self.active.retain(|active| active.who != who);
        self.budgets.retain(|budget| budget.who != who);
    }

    /// Advances every effect by `elapsed_minutes` of game time on game day `day`, in the order they
    /// were applied: drains change their attribute, and finished effects expire. An effect on its
    /// own depends only on the total minutes, so one tick of a minute and 1,800 shorter ticks agree.
    /// Effects that meet at a clamp or share a budget are settled tick by tick, so for them the
    /// tick size can matter.
    pub fn tick(&mut self, elapsed_minutes: Fixed32, day: u32, context: &mut EffectContext<'_>) {
        assert!(
            elapsed_minutes >= Fixed32::ZERO,
            "time does not run backwards"
        );
        self.budgets.retain(|budget| budget.day >= day);
        let mut index = 0;
        while index < self.active.len() {
            let active = &mut self.active[index];
            let duration = active
                .effect
                .remaining_minutes
                .map(|minutes| minutes.raw().max(0) as i64);
            let mut elapsed = active.elapsed + elapsed_minutes.raw() as i64;
            if let Some(duration) = duration {
                elapsed = elapsed.min(duration);
            }
            active.elapsed = elapsed;
            if let Some(per_minute) = active.effect.per_minute {
                let total = ((per_minute.raw() as i128 * elapsed as i128) >> 16) as i64;
                let mut change = total - active.applied;
                active.applied = total;
                let (who, attribute) = (active.who, active.effect.attribute);
                // A budget is spent by what actually lands, after the attribute clamps, so a
                // prayer at full hope spends nothing.
                let budget = match active.effect.stacking {
                    Stacking::DailyBudget { cap_per_day } => {
                        let index = Self::budget(&mut self.budgets, who, &active.effect.tag, day);
                        let budget = &self.budgets[index];
                        let left = (cap_per_day.raw().max(0) as i64 - budget.used).max(0);
                        change = change.signum() * change.abs().min(left);
                        Some(index)
                    }
                    _ => None,
                };
                if change != 0 {
                    if let Some(attributes) = context.attributes.get_mut(who) {
                        let before = attributes.get(attribute).current();
                        let change = Fixed32::from_raw(
                            change.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                        );
                        attributes.apply(
                            who,
                            attribute,
                            change,
                            context.registry,
                            context.attribute_events,
                        );
                        let landed =
                            attributes.get(attribute).current().raw() as i64 - before.raw() as i64;
                        if let Some(index) = budget {
                            self.budgets[index].used += landed.abs();
                        }
                    }
                }
            }
            let active = &self.active[index];
            if duration.is_some_and(|duration| active.elapsed >= duration) {
                let finished = self.active.remove(index);
                Self::release(&finished, context);
                context.effect_events.push(EffectEvent::Expired {
                    who: finished.who,
                    tag: finished.effect.tag,
                });
            } else {
                index += 1;
            }
        }
    }

    /// Effects active on `who`, in the order they were applied, with the minutes each has left.
    pub fn on(
        &self,
        who: Handle,
    ) -> impl Iterator<Item = (EffectHandle, &Effect, Option<Fixed32>)> + '_ {
        self.active
            .iter()
            .filter(move |active| active.who == who)
            .map(|active| {
                let left = active.effect.remaining_minutes.map(|minutes| {
                    Fixed32::from_raw((minutes.raw() as i64 - active.elapsed).max(0) as i32)
                });
                (active.handle, &active.effect, left)
            })
    }

    pub fn has(&self, who: Handle, tag: &EffectTag) -> bool {
        self.active
            .iter()
            .any(|active| active.who == who && &active.effect.tag == tag)
    }

    /// The index of today's budget for this entity and tag, created or restarted as needed.
    fn budget(budgets: &mut Vec<Budget>, who: Handle, tag: &EffectTag, day: u32) -> usize {
        let index = match budgets
            .iter()
            .position(|budget| budget.who == who && &budget.tag == tag)
        {
            Some(index) => index,
            None => {
                budgets.push(Budget {
                    who,
                    tag: tag.clone(),
                    day,
                    used: 0,
                });
                budgets.len() - 1
            }
        };
        let budget = &mut budgets[index];
        if budget.day != day {
            budget.day = day;
            budget.used = 0;
        }
        index
    }

    fn remove_matching(
        &mut self,
        matches: impl Fn(&Active) -> bool,
        context: &mut EffectContext<'_>,
    ) -> usize {
        let mut removed = 0;
        let mut index = 0;
        while index < self.active.len() {
            if matches(&self.active[index]) {
                let gone = self.active.remove(index);
                Self::release(&gone, context);
                context.effect_events.push(EffectEvent::Removed {
                    who: gone.who,
                    tag: gone.effect.tag,
                });
                removed += 1;
            } else {
                index += 1;
            }
        }
        removed
    }

    /// Takes back the modifier an effect held.
    fn release(active: &Active, context: &mut EffectContext<'_>) {
        if let (Some(modifier), Some(attributes)) =
            (active.modifier, context.attributes.get_mut(active.who))
        {
            attributes.remove_modifier(
                active.who,
                active.effect.attribute,
                modifier,
                context.registry,
                context.attribute_events,
            );
        }
    }
}
