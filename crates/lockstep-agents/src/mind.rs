// SPDX-License-Identifier: Apache-2.0
use crate::perception::distance_metres;
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, Message};
use lockstep_spatial::{Cell, GridMap, Topology};
use serde::{Deserialize, Serialize};

/// Where an agent last knew something to be. It hunts this, not the thing itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastKnown {
    pub position: Cell,
    /// The thing seen, when it was seen; a noise may have no source.
    pub target: Option<Handle>,
    /// From its starting confidence when fresh down to 0 when forgotten.
    pub confidence: Fixed32,
    /// The confidence it started with: 1 for a sighting, less for a noise.
    pub starting_confidence: Fixed32,
    pub age_minutes: Fixed32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Alertness {
    Idle,
    Curious,
    Searching,
    Alert,
}

/// One agent's state of mind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mind {
    pub alertness: Alertness,
    pub memory: Option<LastKnown>,
    /// Game minutes since the target was last seen, while Alert.
    pub unseen_minutes: Fixed32,
}

impl Default for Mind {
    fn default() -> Self {
        Mind {
            alertness: Alertness::Idle,
            memory: None,
            unseen_minutes: Fixed32::ZERO,
        }
    }
}

/// How long things take, in game minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MindRules {
    /// A memory's confidence falls from 1 to 0 over this long, and then it is forgotten.
    pub forget_after_minutes: Fixed32,
    /// Alert turns to Searching once the target has been out of sight this long.
    pub lose_sight_after_minutes: Fixed32,
    /// The confidence a heard noise starts with; a sighting starts at 1.
    pub heard_confidence: Fixed32,
}

/// A home and how far from it an agent attends to anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leash {
    pub home: Cell,
    pub radius_metres: Fixed32,
}

impl Leash {
    /// Whether a cell is within the leash.
    pub fn allows<T: Topology>(&self, map: &GridMap<T>, cell: Cell, cell_metres: Fixed32) -> bool {
        distance_metres(map, self.home, cell, cell_metres) <= self.radius_metres
    }
}

/// What an agent sensed this step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stimulus {
    Heard { at: Cell, source: Option<Handle> },
    Saw { target: Handle, at: Cell },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum MindEvent {
    Changed {
        agent: Handle,
        from: Alertness,
        to: Alertness,
    },
    /// The director kept the agent Curious: the target's Alert budget is spent.
    Held { agent: Handle, target: Handle },
}

/// The director's rule: at most this many agents Alert on one target at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Director {
    pub alert_budget: u16,
}

/// What `think` reads besides the minds.
pub struct Surroundings<'a, T: Topology> {
    pub map: &'a GridMap<T>,
    pub occupancy: &'a lockstep_spatial::Occupancy,
    pub cell_metres: Fixed32,
}

/// Ages every memory by `minutes`, then applies each agent's stimuli in order. Transitions:
/// Idle hears a noise and turns Curious; any agent that sees turns Alert; Alert out of sight
/// long enough turns Searching; an agent whose memory is forgotten turns Idle. A stimulus
/// outside the agent's leash is ignored. Then the director grants Alert to at most its budget of
/// agents per target, nearest the target first (then those already Alert, then by handle); the
/// rest stay Curious (`Held`). Agents are taken in handle order.
#[allow(clippy::too_many_arguments)]
pub fn think<T: Topology>(
    minds: &mut Column<Mind>,
    leashes: &Column<Leash>,
    stimuli: &[(Handle, Stimulus)],
    minutes: Fixed32,
    rules: &MindRules,
    director: &Director,
    surroundings: &Surroundings<'_, T>,
    events: &mut Vec<MindEvent>,
) {
    let (map, cell_metres) = (surroundings.map, surroundings.cell_metres);
    let minutes = minutes.max(Fixed32::ZERO);
    let mut before_of = Vec::new();
    for (agent, mind) in minds.iter_mut() {
        let before = mind.alertness;
        let mut saw = false;
        for (_, stimulus) in stimuli.iter().filter(|(who, _)| *who == agent) {
            let at = match stimulus {
                Stimulus::Heard { at, .. } | Stimulus::Saw { at, .. } => *at,
            };
            if leashes
                .get(agent)
                .is_some_and(|leash| !leash.allows(map, at, cell_metres))
            {
                continue;
            }
            match *stimulus {
                Stimulus::Saw { target, at } => {
                    saw = true;
                    mind.memory = Some(fresh(at, Some(target), Fixed32::ONE));
                    mind.alertness = Alertness::Alert;
                }
                Stimulus::Heard { at, source } => {
                    if mind.alertness == Alertness::Alert {
                        continue;
                    }
                    let stronger = mind
                        .memory
                        .is_none_or(|memory| memory.confidence <= rules.heard_confidence);
                    if stronger {
                        mind.memory = Some(fresh(at, source, rules.heard_confidence));
                    }
                    if mind.alertness == Alertness::Idle {
                        mind.alertness = Alertness::Curious;
                    }
                }
            }
        }
        if saw {
            mind.unseen_minutes = Fixed32::ZERO;
        } else {
            age(mind, minutes, rules);
        }
        before_of.push((agent, before));
    }
    hold_over_budget(minds, director, surroundings, &before_of, events);
    for (agent, before) in before_of {
        let after = minds.get(agent).expect("listed").alertness;
        if after != before {
            events.push(MindEvent::Changed {
                agent,
                from: before,
                to: after,
            });
        }
    }
}

/// Keeps at most the budget Alert on each target: nearest the target first, then those already
/// Alert before this step, then by handle. The rest turn Curious.
fn hold_over_budget<T: Topology>(
    minds: &mut Column<Mind>,
    director: &Director,
    surroundings: &Surroundings<'_, T>,
    before_of: &[(Handle, Alertness)],
    events: &mut Vec<MindEvent>,
) {
    // (target, distance, not already Alert, agent), sorted, so each target's best come first.
    let mut wanting = Vec::new();
    for (agent, mind) in minds.iter() {
        let Some(target) = mind.memory.and_then(|memory| memory.target) else {
            continue;
        };
        if mind.alertness != Alertness::Alert {
            continue;
        }
        let distance = match (surroundings.occupancy.cell_of(agent), mind.memory) {
            (Some(at), Some(memory)) => distance_metres(
                surroundings.map,
                at,
                memory.position,
                surroundings.cell_metres,
            ),
            _ => Fixed32::from_raw(i32::MAX),
        };
        let was_alert = before_of
            .iter()
            .any(|(who, before)| *who == agent && *before == Alertness::Alert);
        wanting.push((target, distance, !was_alert, agent));
    }
    wanting.sort();
    let mut granted = 0;
    let mut current = None;
    for (target, _, _, agent) in wanting {
        if current != Some(target) {
            current = Some(target);
            granted = 0;
        }
        if granted < director.alert_budget {
            granted += 1;
            continue;
        }
        minds.get_mut(agent).expect("listed").alertness = Alertness::Curious;
        events.push(MindEvent::Held { agent, target });
    }
}

fn fresh(position: Cell, target: Option<Handle>, confidence: Fixed32) -> LastKnown {
    LastKnown {
        position,
        target,
        confidence,
        starting_confidence: confidence,
        age_minutes: Fixed32::ZERO,
    }
}

/// One step without a sighting: the memory ages and fades, and the timed transitions run.
fn age(mind: &mut Mind, minutes: Fixed32, rules: &MindRules) {
    if mind.alertness == Alertness::Alert {
        mind.unseen_minutes += minutes;
        if mind.unseen_minutes >= rules.lose_sight_after_minutes {
            mind.alertness = Alertness::Searching;
        }
    }
    if let Some(memory) = &mut mind.memory {
        memory.age_minutes += minutes;
        let forget = rules.forget_after_minutes;
        if memory.age_minutes >= forget {
            mind.memory = None;
        } else {
            // Fades linearly, computed from the age each time, so it never drifts.
            let left = (forget - memory.age_minutes).raw() as i64;
            let faded = memory.starting_confidence.raw() as i64 * left / forget.raw() as i64;
            memory.confidence = Fixed32::from_raw(faded as i32);
        }
    }
    if mind.memory.is_none() {
        mind.alertness = Alertness::Idle;
    }
}
