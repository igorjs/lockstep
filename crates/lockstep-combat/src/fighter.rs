// SPDX-License-Identifier: Apache-2.0
//! Actions with commitment and dodges with invulnerability, as one state machine per fighter.
//! The kernel owns timing; the host animates to it.

use crate::damage::{resolve, DamagePacket, DamageResult, Defence, Tags};
use crate::knockback::{knock_back, Knocked};
use crate::shape::{direction, hits, HitShape};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{Column, Handle, Indexable, Message, SmoothedState, Streams};
use lockstep_spatial::{GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// An action's index in its `Moveset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionId(pub u16);

/// What can interrupt an action during its wind-up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InterruptMask(u8);

impl InterruptMask {
    pub const NONE: InterruptMask = InterruptMask(0);
    /// A hit that staggers.
    pub const STAGGER: InterruptMask = InterruptMask(1);

    pub fn contains(self, other: InterruptMask) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }
}

/// An attack: a telegraphed wind-up that a dodge or a stagger can cancel, an active window whose
/// first step strikes, and a recovery during which the fighter is locked out. Times are seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub windup_seconds: Fixed32,
    pub active_seconds: Fixed32,
    pub recovery_seconds: Fixed32,
    pub shape: HitShape,
    pub stamina_cost: Fixed32,
    pub damage: DamagePacket,
    pub interruptible_by: InterruptMask,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DodgeDefinition {
    /// Vulnerable; a stagger here cancels the dodge.
    pub startup_seconds: Fixed32,
    /// Hits that would land here are dodged instead, unless they grab or cannot be avoided.
    pub invulnerable_seconds: Fixed32,
    pub recovery_seconds: Fixed32,
    /// Cells moved when invulnerability starts; a wall or a body shortens it.
    pub distance_cells: u8,
    pub stamina_cost: Fixed32,
    /// From the press to the next dodge.
    pub cooldown_seconds: Fixed32,
    /// A dodge pressed at most this long before the attack's first active step is perfect.
    pub perfect_window_seconds: Fixed32,
    /// After a perfect dodge, the next attack started within this much free time skips its
    /// wind-up; the window does not run while the fighter is still dodging or recovering.
    pub counter_seconds: Fixed32,
}

/// Every action a kind of fighter can take, and its dodge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moveset {
    pub actions: Vec<ActionDefinition>,
    pub dodge: DodgeDefinition,
}

/// How long an order waits for the fighter to be free.
pub fn buffer_seconds() -> Fixed32 {
    Fixed32::from_ratio(15, 100)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Order {
    Attack { action: ActionId, facing: Turn },
    Dodge { heading: Turn },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Windup {
        action: ActionId,
        left: Fixed32,
    },
    Active {
        action: ActionId,
        left: Fixed32,
        struck: bool,
    },
    Recovery {
        left: Fixed32,
    },
    DodgeStartup {
        heading: Turn,
        left: Fixed32,
    },
    DodgeInvulnerable {
        left: Fixed32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Buffered {
    order: Order,
    age: Fixed32,
}

/// One fighter's combat state. Keep it in a `Column<Fighter>`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub phase: Phase,
    pub facing: Turn,
    pub stamina: Fixed32,
    pub defence: Defence,
    buffered: Option<Buffered>,
    cooldown: Fixed32,
    counter: Fixed32,
    /// Seconds since the current dodge was pressed, while dodging.
    since_dodge: Option<Fixed32>,
    evasion_memory: SmoothedState,
}

impl Fighter {
    pub fn new(stamina: Fixed32, defence: Defence) -> Self {
        Fighter {
            phase: Phase::Ready,
            facing: 0,
            stamina,
            defence,
            buffered: None,
            cooldown: Fixed32::ZERO,
            counter: Fixed32::ZERO,
            since_dodge: None,
            evasion_memory: SmoothedState::default(),
        }
    }

    /// Whether the next attack skips its wind-up, after a perfect dodge.
    pub fn countering(&self) -> bool {
        self.counter > Fixed32::ZERO
    }

    pub fn invulnerable(&self) -> bool {
        matches!(self.phase, Phase::DodgeInvulnerable { .. })
    }
}

/// One body a strike reached.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub target: Handle,
    pub result: DamageResult,
    /// Where a knockback left the target, when there was one.
    pub knocked: Option<Knocked>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    UnknownAction,
    Tired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1, manual_indexable)]
pub enum CombatEvent {
    Started {
        who: Handle,
        action: ActionId,
    },
    /// The first active step struck these bodies.
    Landed {
        who: Handle,
        action: ActionId,
        hits: Vec<Hit>,
    },
    Whiffed {
        who: Handle,
        action: ActionId,
    },
    /// A stagger cancelled the wind-up.
    Interrupted {
        who: Handle,
        action: ActionId,
        by: Handle,
    },
    DodgeStarted {
        who: Handle,
    },
    /// A stagger cancelled the dodge during its startup.
    DodgeBroken {
        who: Handle,
        by: Handle,
    },
    Dodged {
        who: Handle,
        by: Handle,
    },
    /// Dodged within the perfect window: stamina back, and the next attack skips its wind-up.
    PerfectDodge {
        who: Handle,
        by: Handle,
    },
    Refused {
        who: Handle,
        reason: Refusal,
    },
    /// The order waited longer than the buffer and was dropped.
    Expired {
        who: Handle,
    },
}

impl Indexable for CombatEvent {
    fn kind(&self) -> u16 {
        match self {
            CombatEvent::Started { .. } => 0,
            CombatEvent::Landed { .. } => 1,
            CombatEvent::Whiffed { .. } => 2,
            CombatEvent::Interrupted { .. } => 3,
            CombatEvent::DodgeStarted { .. } => 4,
            CombatEvent::DodgeBroken { .. } => 5,
            CombatEvent::Dodged { .. } => 6,
            CombatEvent::PerfectDodge { .. } => 7,
            CombatEvent::Refused { .. } => 8,
            CombatEvent::Expired { .. } => 9,
        }
    }

    fn handles(&self, out: &mut Vec<Handle>) {
        match self {
            CombatEvent::Landed { who, hits, .. } => {
                out.push(*who);
                out.extend(hits.iter().map(|hit| hit.target));
            }
            CombatEvent::Interrupted { who, by, .. }
            | CombatEvent::DodgeBroken { who, by }
            | CombatEvent::Dodged { who, by }
            | CombatEvent::PerfectDodge { who, by } => out.extend([*who, *by]),
            CombatEvent::Started { who, .. }
            | CombatEvent::Whiffed { who, .. }
            | CombatEvent::DodgeStarted { who }
            | CombatEvent::Refused { who, .. }
            | CombatEvent::Expired { who } => out.push(*who),
        }
    }
}

/// Runs one step of combat for every fighter.
///
/// 1. Each order replaces its fighter's buffered order, in the order given.
/// 2. In handle order, each fighter's timers and phase advance by `step_seconds`, then its buffered
///    order starts if the fighter is free (a dodge may also cancel a wind-up), or waits up to
///    0.15 seconds.
/// 3. Every attack whose first active step is now strikes, in handle order: an invulnerable target
///    dodges it unless it grabs or cannot be avoided, any other target takes `resolve`, a stagger
///    interrupts a wind-up whose mask allows it and breaks a dodge still in startup, and knockback
///    moves the target.
#[allow(clippy::too_many_arguments)]
pub fn step_combat<T: Topology>(
    fighters: &mut Column<Fighter>,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    orders: &[(Handle, Order)],
    step_seconds: Fixed32,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) {
    for (who, order) in orders {
        if let Some(fighter) = fighters.get_mut(*who) {
            fighter.buffered = Some(Buffered {
                order: *order,
                age: Fixed32::ZERO,
            });
        }
    }
    let mut strikes = Vec::new();
    for who in fighters.handles() {
        let fighter = fighters.get_mut(who).expect("listed");
        advance(who, fighter, moveset, map, occupancy, step_seconds);
        start_buffered(who, fighter, moveset, step_seconds, events);
        if let Phase::Active {
            action,
            left,
            struck: false,
        } = fighter.phase
        {
            fighter.phase = Phase::Active {
                action,
                left,
                struck: true,
            };
            strikes.push((who, action, fighter.facing));
        }
    }
    for (who, action, facing) in strikes {
        strike(
            who, action, facing, fighters, moveset, map, occupancy, streams, events,
        );
    }
}

fn advance<T: Topology>(
    who: Handle,
    fighter: &mut Fighter,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    step: Fixed32,
) {
    let down = |value: Fixed32| (value - step).max(Fixed32::ZERO);
    fighter.cooldown = down(fighter.cooldown);
    // The counter window runs only while the fighter is free to use it, not during the dodge's
    // own recovery.
    if fighter.phase == Phase::Ready {
        fighter.counter = down(fighter.counter);
    }
    if let Some(since) = fighter.since_dodge.as_mut() {
        *since += step;
    }
    let dodge = &moveset.dodge;
    fighter.phase = match fighter.phase {
        Phase::Ready => Phase::Ready,
        Phase::Windup { action, left } => match down(left) {
            Fixed32::ZERO => Phase::Active {
                action,
                left: moveset.actions[action.0 as usize].active_seconds,
                struck: false,
            },
            left => Phase::Windup { action, left },
        },
        Phase::Active {
            action,
            left,
            struck,
        } => match down(left) {
            Fixed32::ZERO => Phase::Recovery {
                left: moveset.actions[action.0 as usize].recovery_seconds,
            },
            left => Phase::Active {
                action,
                left,
                struck,
            },
        },
        Phase::Recovery { left } => match down(left) {
            Fixed32::ZERO => Phase::Ready,
            left => Phase::Recovery { left },
        },
        Phase::DodgeStartup { heading, left } => match down(left) {
            Fixed32::ZERO => {
                knock_back(map, occupancy, who, heading, dodge.distance_cells);
                Phase::DodgeInvulnerable {
                    left: dodge.invulnerable_seconds,
                }
            }
            left => Phase::DodgeStartup { heading, left },
        },
        Phase::DodgeInvulnerable { left } => match down(left) {
            Fixed32::ZERO => {
                fighter.since_dodge = None;
                Phase::Recovery {
                    left: dodge.recovery_seconds,
                }
            }
            left => Phase::DodgeInvulnerable { left },
        },
    };
}

fn start_buffered(
    who: Handle,
    fighter: &mut Fighter,
    moveset: &Moveset,
    step: Fixed32,
    events: &mut Vec<CombatEvent>,
) {
    let Some(buffered) = fighter.buffered else {
        return;
    };
    let free = match (buffered.order, fighter.phase) {
        (_, Phase::Ready) => true,
        // A dodge may cancel a wind-up.
        (Order::Dodge { .. }, Phase::Windup { .. }) => true,
        _ => false,
    };
    let cooling = matches!(buffered.order, Order::Dodge { .. }) && fighter.cooldown > Fixed32::ZERO;
    if !free || cooling {
        let age = buffered.age + step;
        if age > buffer_seconds() {
            fighter.buffered = None;
            events.push(CombatEvent::Expired { who });
        } else {
            fighter.buffered = Some(Buffered { age, ..buffered });
        }
        return;
    }
    fighter.buffered = None;
    match buffered.order {
        Order::Attack { action, facing } => {
            let Some(definition) = moveset.actions.get(action.0 as usize) else {
                events.push(CombatEvent::Refused {
                    who,
                    reason: Refusal::UnknownAction,
                });
                return;
            };
            if fighter.stamina < definition.stamina_cost {
                events.push(CombatEvent::Refused {
                    who,
                    reason: Refusal::Tired,
                });
                return;
            }
            fighter.stamina -= definition.stamina_cost;
            fighter.facing = facing;
            let skip_windup = fighter.countering() || definition.windup_seconds <= Fixed32::ZERO;
            fighter.counter = Fixed32::ZERO;
            fighter.phase = if skip_windup {
                Phase::Active {
                    action,
                    left: definition.active_seconds,
                    struck: false,
                }
            } else {
                Phase::Windup {
                    action,
                    left: definition.windup_seconds,
                }
            };
            events.push(CombatEvent::Started { who, action });
        }
        Order::Dodge { heading } => {
            let dodge = &moveset.dodge;
            if fighter.stamina < dodge.stamina_cost {
                events.push(CombatEvent::Refused {
                    who,
                    reason: Refusal::Tired,
                });
                return;
            }
            fighter.stamina -= dodge.stamina_cost;
            fighter.cooldown = dodge.cooldown_seconds;
            fighter.since_dodge = Some(Fixed32::ZERO);
            fighter.phase = Phase::DodgeStartup {
                heading,
                left: dodge.startup_seconds,
            };
            events.push(CombatEvent::DodgeStarted { who });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn strike<T: Topology>(
    who: Handle,
    action: ActionId,
    facing: Turn,
    fighters: &mut Column<Fighter>,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) {
    let Some(from) = occupancy.cell_of(who) else {
        return;
    };
    let definition = &moveset.actions[action.0 as usize];
    let mut reached = Vec::new();
    hits(
        definition.shape,
        map,
        occupancy,
        who,
        from,
        facing,
        &mut reached,
    );
    let piercing = definition.damage.tags.contains(Tags::GRAB)
        || definition.damage.tags.contains(Tags::UNAVOIDABLE);
    let mut landed = Vec::new();
    for (cell, target) in reached {
        let mut no_fighter = None;
        let fighter = match fighters.get_mut(target) {
            Some(fighter) => fighter,
            None => no_fighter.insert(Fighter::new(Fixed32::ZERO, Defence::default())),
        };
        if fighter.invulnerable() && !piercing {
            let perfect = fighter
                .since_dodge
                .is_some_and(|since| since <= moveset.dodge.perfect_window_seconds);
            if perfect {
                fighter.stamina += moveset.dodge.stamina_cost;
                fighter.counter = moveset.dodge.counter_seconds;
                events.push(CombatEvent::PerfectDodge {
                    who: target,
                    by: who,
                });
            } else {
                events.push(CombatEvent::Dodged {
                    who: target,
                    by: who,
                });
            }
            continue;
        }
        let result = resolve(
            &definition.damage,
            &fighter.defence,
            &mut fighter.evasion_memory,
            streams,
        );
        if result.staggered {
            match fighter.phase {
                Phase::Windup { action: theirs, .. }
                    if moveset.actions[theirs.0 as usize]
                        .interruptible_by
                        .contains(InterruptMask::STAGGER) =>
                {
                    fighter.phase = Phase::Ready;
                    events.push(CombatEvent::Interrupted {
                        who: target,
                        action: theirs,
                        by: who,
                    });
                }
                Phase::DodgeStartup { .. } => {
                    fighter.phase = Phase::Ready;
                    fighter.since_dodge = None;
                    events.push(CombatEvent::DodgeBroken {
                        who: target,
                        by: who,
                    });
                }
                _ => {}
            }
        }
        let knocked = (result.knockback > 0 && !result.evaded).then(|| {
            knock_back(
                map,
                occupancy,
                target,
                direction(map, from, cell),
                result.knockback,
            )
        });
        landed.push(Hit {
            target,
            result,
            knocked,
        });
    }
    events.push(if landed.is_empty() {
        CombatEvent::Whiffed { who, action }
    } else {
        CombatEvent::Landed {
            who,
            action,
            hits: landed,
        }
    });
}
