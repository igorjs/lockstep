// SPDX-License-Identifier: Apache-2.0
use crate::curve::{divide_rounded, saturate};
use crate::registry::{AttributeId, Definition, MaximumPolicy, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::Handle;
use serde::{Deserialize, Serialize};

/// A change to an attribute's maximum. The maximum is `(base + every Add) × every Multiply`, and
/// then the last `Override` added wins, so base 100 with +20 and ×1.5 is 180, not 170.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modifier {
    Add(Fixed32),
    Multiply(Fixed32),
    Override(Fixed32),
}

/// Names one modifier on one entity, so its owner removes exactly the one it added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModifierHandle(u32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeEvent {
    /// The current value passed a threshold: downward when it fell below it, upward when it reached
    /// it again.
    Crossed {
        who: Handle,
        attribute: AttributeId,
        threshold: String,
        upward: bool,
    },
    /// The current value reached the minimum.
    Emptied { who: Handle, attribute: AttributeId },
    /// The current value reached the maximum.
    Filled { who: Handle, attribute: AttributeId },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    current: Fixed32,
    minimum: Fixed32,
    maximum: Fixed32,
    /// In the order they were added, which is handle order.
    modifiers: Vec<(ModifierHandle, Modifier)>,
}

impl Attribute {
    pub fn current(&self) -> Fixed32 {
        self.current
    }

    /// The definition's minimum, or higher once a ratchet has raised it.
    pub fn minimum(&self) -> Fixed32 {
        self.minimum
    }

    /// The maximum after modifiers.
    pub fn maximum(&self) -> Fixed32 {
        self.maximum
    }

    /// How full the attribute is: 0 at the minimum, 1 at the maximum.
    pub fn fraction(&self) -> Fixed32 {
        let span = self.maximum.raw() as i128 - self.minimum.raw() as i128;
        if span <= 0 {
            return Fixed32::ONE;
        }
        let filled = (self.current.raw() as i128 - self.minimum.raw() as i128) << 16;
        saturate(divide_rounded(filled, span))
    }

    pub fn modifiers(&self) -> impl Iterator<Item = (ModifierHandle, Modifier)> + '_ {
        self.modifiers.iter().copied()
    }
}

/// Every attribute of one entity, in registry order. Keep it in a `Column<Attributes>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attributes {
    values: Vec<Attribute>,
    next_modifier: u32,
}

impl Attributes {
    /// Every attribute at its starting value, and every derived attribute computed from them.
    pub fn from_registry(registry: &Registry) -> Self {
        let values = registry
            .ids()
            .map(|id| {
                let definition = registry.definition(id);
                Attribute {
                    current: definition.starting,
                    minimum: definition.minimum,
                    maximum: definition.maximum,
                    modifiers: Vec::new(),
                }
            })
            .collect();
        let mut attributes = Attributes {
            values,
            next_modifier: 0,
        };
        attributes.recompute_derived(None, registry, &mut Vec::new());
        attributes
    }

    pub fn get(&self, id: AttributeId) -> &Attribute {
        &self.values[id.0 as usize]
    }

    /// Changes the current value by `delta`, clamped, then recomputes derived attributes. Emits
    /// each threshold crossed, in the order the value passes them. A derived attribute's own value
    /// comes from its curve, so `apply` on it changes nothing.
    pub fn apply(
        &mut self,
        who: Handle,
        id: AttributeId,
        delta: Fixed32,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) {
        if registry.definition(id).derived.is_none() {
            let attribute = &self.values[id.0 as usize];
            let target = attribute.current.raw() as i128 + delta.raw() as i128;
            let next = saturate(target).clamp(attribute.minimum, attribute.maximum);
            self.set_current(who, id, next, registry, events);
        }
        self.recompute_derived(Some(who), registry, events);
    }

    pub fn add_modifier(
        &mut self,
        who: Handle,
        id: AttributeId,
        modifier: Modifier,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) -> ModifierHandle {
        let handle = ModifierHandle(self.next_modifier);
        self.next_modifier = self
            .next_modifier
            .checked_add(1)
            .expect("four billion modifiers");
        self.values[id.0 as usize]
            .modifiers
            .push((handle, modifier));
        self.refresh_maximum(who, id, registry, events);
        self.recompute_derived(Some(who), registry, events);
        handle
    }

    /// Removes one modifier. False when this attribute holds no modifier with that handle.
    pub fn remove_modifier(
        &mut self,
        who: Handle,
        id: AttributeId,
        handle: ModifierHandle,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) -> bool {
        let modifiers = &mut self.values[id.0 as usize].modifiers;
        let Some(index) = modifiers.iter().position(|(held, _)| *held == handle) else {
            return false;
        };
        modifiers.remove(index);
        self.refresh_maximum(who, id, registry, events);
        self.recompute_derived(Some(who), registry, events);
        true
    }

    /// Recomputes the maximum from the modifiers and moves the current value by the policy.
    fn refresh_maximum(
        &mut self,
        who: Handle,
        id: AttributeId,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) {
        let definition = registry.definition(id);
        let attribute = &mut self.values[id.0 as usize];
        let old_maximum = attribute.maximum;
        let maximum = modified_maximum(definition, &attribute.modifiers).max(attribute.minimum);
        attribute.maximum = maximum;
        let (current, minimum) = (attribute.current, attribute.minimum);
        // A derived value comes from its curve, recomputed right after this, so scaling it here
        // would only report crossings the recompute takes back.
        let policy = if definition.derived.is_some() {
            MaximumPolicy::Clamp
        } else {
            definition.on_maximum_change
        };
        let next = match policy {
            MaximumPolicy::Clamp | MaximumPolicy::Ratchet => current,
            MaximumPolicy::ScaleCurrent => {
                let old_span = old_maximum.raw() as i128 - minimum.raw() as i128;
                if old_span <= 0 {
                    current
                } else {
                    let new_span = maximum.raw() as i128 - minimum.raw() as i128;
                    let filled = current.raw() as i128 - minimum.raw() as i128;
                    saturate(minimum.raw() as i128 + divide_rounded(filled * new_span, old_span))
                }
            }
        }
        .clamp(minimum, maximum);
        self.set_current_with_maximum(who, id, next, old_maximum, registry, events);
    }

    fn set_current(
        &mut self,
        who: Handle,
        id: AttributeId,
        next: Fixed32,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) {
        let maximum = self.values[id.0 as usize].maximum;
        self.set_current_with_maximum(who, id, next, maximum, registry, events);
    }

    /// Sets the current value and emits what changed. `old_maximum` is the maximum before this
    /// change, so a lowered maximum that meets the current value counts as filling it.
    fn set_current_with_maximum(
        &mut self,
        who: Handle,
        id: AttributeId,
        next: Fixed32,
        old_maximum: Fixed32,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) {
        let definition = registry.definition(id);
        let attribute = &mut self.values[id.0 as usize];
        let previous = attribute.current;
        attribute.current = next;

        let mut crossed: Vec<usize> = (0..definition.thresholds.len())
            .filter(|index| {
                let at = definition.thresholds[*index].at;
                if next < previous {
                    previous >= at && next < at
                } else {
                    previous < at && next >= at
                }
            })
            .collect();
        let upward = next > previous;
        // Passing order: rising thresholds lowest first, falling ones highest first. Equal
        // thresholds keep definition order.
        crossed.sort_by(|first, second| {
            let (first_at, second_at) = (
                definition.thresholds[*first].at,
                definition.thresholds[*second].at,
            );
            if upward {
                first_at.cmp(&second_at)
            } else {
                second_at.cmp(&first_at)
            }
        });
        for index in crossed {
            let threshold = &definition.thresholds[index];
            if upward && definition.on_maximum_change == MaximumPolicy::Ratchet {
                attribute.minimum = attribute.minimum.max(threshold.at);
            }
            events.push(AttributeEvent::Crossed {
                who,
                attribute: id,
                threshold: threshold.name.clone(),
                upward,
            });
        }
        if next <= attribute.minimum && previous > attribute.minimum {
            events.push(AttributeEvent::Emptied { who, attribute: id });
        }
        if next >= attribute.maximum && previous < old_maximum {
            events.push(AttributeEvent::Filled { who, attribute: id });
        }
    }

    /// Recomputes every derived attribute in registry order. A derived attribute that reads one
    /// declared after it sees that one's value from before this pass.
    fn recompute_derived(
        &mut self,
        who: Option<Handle>,
        registry: &Registry,
        events: &mut Vec<AttributeEvent>,
    ) {
        for id in registry.ids() {
            let Some(derived) = &registry.definition(id).derived else {
                continue;
            };
            let inputs: Vec<Fixed32> = derived
                .inputs
                .iter()
                .map(|input| self.get(*input).current)
                .collect();
            let attribute = self.get(id);
            let next = derived
                .curve
                .evaluate(&inputs)
                .clamp(attribute.minimum, attribute.maximum);
            match who {
                Some(who) => self.set_current(who, id, next, registry, events),
                None => self.values[id.0 as usize].current = next,
            }
        }
    }
}

fn modified_maximum(definition: &Definition, modifiers: &[(ModifierHandle, Modifier)]) -> Fixed32 {
    let limit = |raw: i128| raw.clamp(i32::MIN as i128, i32::MAX as i128);
    let mut sum = definition.maximum.raw() as i128;
    for (_, modifier) in modifiers {
        if let Modifier::Add(amount) = modifier {
            sum = limit(sum + amount.raw() as i128);
        }
    }
    for (_, modifier) in modifiers {
        if let Modifier::Multiply(factor) = modifier {
            sum = limit((sum * factor.raw() as i128) >> 16);
        }
    }
    let overridden = modifiers
        .iter()
        .rev()
        .find_map(|(_, modifier)| match modifier {
            Modifier::Override(value) => Some(*value),
            _ => None,
        });
    overridden.unwrap_or(saturate(sum))
}
