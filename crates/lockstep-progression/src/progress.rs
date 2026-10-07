// SPDX-License-Identifier: Apache-2.0
use crate::graph::{Graph, NodeId, TriggerId};
use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, ModifierHandle, Registry};
use lockstep_core::{Column, Handle, Message};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One owner's points, the nodes it has taken, and the modifiers those nodes hold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub points: u32,
    taken: BTreeSet<NodeId>,
    held: Vec<(NodeId, AttributeId, ModifierHandle)>,
}

impl Progress {
    pub fn has_taken(&self, node: NodeId) -> bool {
        self.taken.contains(&node)
    }

    /// Every node taken, in node order.
    pub fn taken(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.taken.iter().copied()
    }
}

/// The owners' attributes, which taken nodes modify.
pub struct Owners<'a> {
    pub attributes: &'a mut Column<Attributes>,
    pub registry: &'a Registry,
    pub events: &'a mut Vec<AttributeEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// The owner was never enrolled.
    UnknownOwner,
    UnknownNode,
    UnknownTrigger,
    AlreadyTaken,
    NotTaken,
    /// This required node is not taken yet.
    Requires(NodeId),
    /// This taken node excludes the one asked for.
    ExcludedBy(NodeId),
    /// This attribute is below the node's gate.
    Gate(AttributeId),
    Points {
        needed: u32,
        have: u32,
    },
    /// A keystone is never refunded.
    Keystone,
    /// This taken node requires the one asked to refund.
    RequiredBy(NodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum ProgressEvent {
    Awarded {
        owner: Handle,
        trigger: TriggerId,
        points: u32,
    },
    Unlocked {
        owner: Handle,
        node: NodeId,
    },
    Refunded {
        owner: Handle,
        node: NodeId,
        points: u32,
    },
}

/// Makes an entity an owner of progress, with no points and nothing taken. Only enrolled owners
/// earn points and take nodes. Enrolling twice changes nothing.
pub fn enrol(progress: &mut Column<Progress>, owner: Handle) {
    if !progress.has(owner) {
        progress.set(owner, Progress::default());
    }
}

/// Earns an owner a trigger's points.
pub fn award(
    progress: &mut Column<Progress>,
    owner: Handle,
    trigger: TriggerId,
    graph: &Graph,
    events: &mut Vec<ProgressEvent>,
) -> Result<(), Refusal> {
    let points = graph
        .triggers
        .get(trigger.0 as usize)
        .ok_or(Refusal::UnknownTrigger)?
        .points;
    let held = progress.get_mut(owner).ok_or(Refusal::UnknownOwner)?;
    held.points = held.points.saturating_add(points);
    events.push(ProgressEvent::Awarded {
        owner,
        trigger,
        points,
    });
    Ok(())
}

/// Why an owner cannot take a node now, checked in this order: unknown, already taken, a missing
/// requirement, an exclusion, a gate, then points. `None` when it can.
pub fn why_not(
    progress: &Column<Progress>,
    owner: Handle,
    node: NodeId,
    graph: &Graph,
    attributes: &Column<Attributes>,
) -> Option<Refusal> {
    let Some(held) = progress.get(owner) else {
        return Some(Refusal::UnknownOwner);
    };
    let Some(definition) = graph.node(node) else {
        return Some(Refusal::UnknownNode);
    };
    if held.taken.contains(&node) {
        return Some(Refusal::AlreadyTaken);
    }
    if let Some(missing) = definition
        .requires
        .iter()
        .find(|required| !held.taken.contains(required))
    {
        return Some(Refusal::Requires(*missing));
    }
    if let Some(blocker) = definition
        .excludes
        .iter()
        .find(|excluded| held.taken.contains(excluded))
    {
        return Some(Refusal::ExcludedBy(*blocker));
    }
    for gate in &definition.gates {
        let value = attributes
            .get(owner)
            .map(|attributes| attributes.get(gate.attribute).current());
        if value.is_none_or(|value| value < gate.at_least) {
            return Some(Refusal::Gate(gate.attribute));
        }
    }
    if held.points < definition.cost {
        return Some(Refusal::Points {
            needed: definition.cost,
            have: held.points,
        });
    }
    None
}

/// Takes a node: spends its cost and applies its modifiers to the owner's attributes, when it has
/// any. Gates are checked now, once: a gate that later stops holding takes nothing away.
pub fn unlock(
    progress: &mut Column<Progress>,
    owner: Handle,
    node: NodeId,
    graph: &Graph,
    owners: &mut Owners<'_>,
    events: &mut Vec<ProgressEvent>,
) -> Result<(), Refusal> {
    if let Some(refusal) = why_not(progress, owner, node, graph, owners.attributes) {
        return Err(refusal);
    }
    let definition = graph.node(node).expect("checked");
    let held = progress.get_mut(owner).expect("checked");
    held.points -= definition.cost;
    held.taken.insert(node);
    if let Some(attributes) = owners.attributes.get_mut(owner) {
        for (attribute, modifier) in &definition.modifiers {
            let handle = attributes.add_modifier(
                owner,
                *attribute,
                *modifier,
                owners.registry,
                owners.events,
            );
            held.held.push((node, *attribute, handle));
        }
    }
    events.push(ProgressEvent::Unlocked { owner, node });
    Ok(())
}

/// Gives a node back: returns its refund percent of the cost, rounded down, and removes its
/// modifiers. Refused for a keystone, and while another taken node requires it.
pub fn refund(
    progress: &mut Column<Progress>,
    owner: Handle,
    node: NodeId,
    graph: &Graph,
    owners: &mut Owners<'_>,
    events: &mut Vec<ProgressEvent>,
) -> Result<u32, Refusal> {
    let held = progress.get_mut(owner).ok_or(Refusal::UnknownOwner)?;
    let definition = graph.node(node).ok_or(Refusal::UnknownNode)?;
    if !held.taken.contains(&node) {
        return Err(Refusal::NotTaken);
    }
    if definition.keystone {
        return Err(Refusal::Keystone);
    }
    if let Some(dependant) = held.taken.iter().copied().find(|taken| {
        graph
            .node(*taken)
            .is_some_and(|other| other.requires.contains(&node))
    }) {
        return Err(Refusal::RequiredBy(dependant));
    }
    let points = (definition.cost as u64 * definition.refund_percent as u64 / 100) as u32;
    held.taken.remove(&node);
    held.points = held.points.saturating_add(points);
    let mut kept = Vec::with_capacity(held.held.len());
    for (from, attribute, handle) in std::mem::take(&mut held.held) {
        if from == node {
            if let Some(attributes) = owners.attributes.get_mut(owner) {
                attributes.remove_modifier(
                    owner,
                    attribute,
                    handle,
                    owners.registry,
                    owners.events,
                );
            }
        } else {
            kept.push((from, attribute, handle));
        }
    }
    held.held = kept;
    events.push(ProgressEvent::Refunded {
        owner,
        node,
        points,
    });
    Ok(points)
}
