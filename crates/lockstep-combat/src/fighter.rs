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

/// Which `Moveset` a fighter uses: its index in the slice `step_combat` is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MovesetId(pub u16);

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

/// How many steps a duration lasts at `rate` steps a second: the nearest whole number, ties up.
/// Counting whole steps keeps a 0.2 second wind-up at 6 steps at 30 a second and 12 at 60, where a
/// rounded step length would drift.
pub fn steps_for(seconds: Fixed32, rate: u32) -> u32 {
    let scaled = seconds.raw().max(0) as u64 * rate as u64;
    ((scaled + 32_768) / 65_536) as u32
}

/// Whether `steps` at `rate` steps a second last no longer than `seconds`, compared exactly.
fn within(steps: u32, rate: u32, seconds: Fixed32) -> bool {
    steps as u64 * 65_536 <= seconds.raw().max(0) as u64 * rate as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Order {
    Attack { action: ActionId, facing: Turn },
    Dodge { heading: Turn },
}

/// Where a fighter is in its commitment. `left` counts the steps still to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Windup {
        action: ActionId,
        left: u32,
    },
    Active {
        action: ActionId,
        left: u32,
        struck: bool,
    },
    Recovery {
        left: u32,
    },
    DodgeStartup {
        heading: Turn,
        left: u32,
    },
    DodgeInvulnerable {
        left: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Buffered {
    order: Order,
    /// Steps it has waited.
    age: u32,
}

/// One fighter's combat state. Keep it in a `Column<Fighter>`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub moveset: MovesetId,
    pub phase: Phase,
    pub facing: Turn,
    pub stamina: Fixed32,
    pub maximum_stamina: Fixed32,
    pub defence: Defence,
    buffered: Option<Buffered>,
    /// Steps until the next dodge may start.
    cooldown: u32,
    /// Free steps left in which the next attack skips its wind-up.
    counter: u32,
    /// Steps since the current dodge was pressed, until a strike uses its perfect window.
    since_dodge: Option<u32>,
    evasion_memory: SmoothedState,
}

impl Fighter {
    /// A ready fighter with full stamina.
    pub fn new(moveset: MovesetId, stamina: Fixed32, defence: Defence) -> Self {
        Fighter {
            moveset,
            phase: Phase::Ready,
            facing: 0,
            stamina,
            maximum_stamina: stamina,
            defence,
            buffered: None,
            cooldown: 0,
            counter: 0,
            since_dodge: None,
            evasion_memory: SmoothedState::default(),
        }
    }

    /// Whether the next attack skips its wind-up, after a perfect dodge.
    pub fn countering(&self) -> bool {
        self.counter > 0
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
    UnknownMoveset,
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
    /// A projectile struck a body and stopped.
    ProjectileHit {
        by: Handle,
        projectile: Handle,
        hit: Hit,
    },
    /// A projectile stopped without hitting a body.
    ProjectileStopped {
        projectile: Handle,
        at: lockstep_spatial::Cell,
        reason: crate::projectile::Stopped,
    },
    /// Something loud passed this cell: heard within `loudness` steps.
    Sounded {
        by: Handle,
        at: lockstep_spatial::Cell,
        loudness: u8,
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
            CombatEvent::ProjectileHit { .. } => 10,
            CombatEvent::ProjectileStopped { .. } => 11,
            CombatEvent::Sounded { .. } => 12,
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
            // A projectile's own handle comes from a separate store and could equal an entity's,
            // so only the entities it involves are indexed.
            CombatEvent::ProjectileHit { by, hit, .. } => out.extend([*by, hit.target]),
            CombatEvent::ProjectileStopped { .. } => {}
            CombatEvent::Sounded { by, .. } => out.push(*by),
        }
    }
}

/// Runs one step of combat for every fighter, at `steps_per_second` (30 for the default runner).
///
/// 1. Each order replaces its fighter's buffered order, in the order given.
/// 2. In handle order, each fighter's timers and phase advance one step, then its buffered order
///    starts if the fighter is free (a dodge may also cancel a wind-up), or waits up to 0.15
///    seconds.
/// 3. Every attack on its first active step reads its targets, all before any of them resolves, so
///    two fighters trading blows on the same step both land.
/// 4. The strikes resolve in handle order: an invulnerable target dodges unless the attack grabs or
///    cannot be avoided, any other target takes `resolve`, a stagger interrupts a wind-up whose mask
///    allows it and breaks a dodge still in startup, and knockback moves the target.
#[allow(clippy::too_many_arguments)]
pub fn step_combat<T: Topology>(
    fighters: &mut Column<Fighter>,
    movesets: &[Moveset],
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    orders: &[(Handle, Order)],
    steps_per_second: u32,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) {
    let rate = steps_per_second.max(1);
    for (who, order) in orders {
        if let Some(fighter) = fighters.get_mut(*who) {
            fighter.buffered = Some(Buffered {
                order: *order,
                age: 0,
            });
        }
    }
    let mut strikes = Vec::new();
    for who in fighters.handles() {
        let fighter = fighters.get_mut(who).expect("listed");
        let Some(moveset) = movesets.get(fighter.moveset.0 as usize) else {
            if fighter.buffered.take().is_some() {
                events.push(CombatEvent::Refused {
                    who,
                    reason: Refusal::UnknownMoveset,
                });
            }
            fighter.phase = Phase::Ready;
            continue;
        };
        advance(who, fighter, moveset, map, occupancy, rate);
        start_buffered(who, fighter, moveset, map, occupancy, rate, events);
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
            strikes.push((who, fighter.moveset, action, fighter.facing));
        }
    }
    // Read every strike's targets before any resolves or moves anyone.
    let reached: Vec<_> = strikes
        .into_iter()
        .filter_map(|(who, moveset, action, facing)| {
            let from = occupancy.cell_of(who)?;
            let definition = &movesets[moveset.0 as usize].actions[action.0 as usize];
            let mut targets = Vec::new();
            hits(
                definition.shape,
                map,
                occupancy,
                who,
                from,
                facing,
                &mut targets,
            );
            Some((who, moveset, action, from, targets))
        })
        .collect();
    for (who, moveset, action, from, targets) in reached {
        strike(
            who,
            &movesets[moveset.0 as usize],
            action,
            from,
            targets,
            fighters,
            movesets,
            map,
            occupancy,
            rate,
            streams,
            events,
        );
    }
}

/// The phase that follows a wind-up: the action's active window, at least one step so it strikes.
fn active(moveset: &Moveset, action: ActionId, rate: u32) -> Phase {
    Phase::Active {
        action,
        left: steps_for(moveset.actions[action.0 as usize].active_seconds, rate).max(1),
        struck: false,
    }
}

/// A recovery of `seconds`, or ready at once when it lasts no step.
fn recovery(seconds: Fixed32, rate: u32) -> Phase {
    match steps_for(seconds, rate) {
        0 => Phase::Ready,
        left => Phase::Recovery { left },
    }
}

/// Starts the dodge's invulnerability: moves the body and counts the invulnerable steps, going
/// straight to recovery when there are none.
fn invulnerable<T: Topology>(
    who: Handle,
    heading: Turn,
    fighter: &mut Fighter,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    rate: u32,
) -> Phase {
    let dodge = &moveset.dodge;
    knock_back(map, occupancy, who, heading, dodge.distance_cells);
    match steps_for(dodge.invulnerable_seconds, rate) {
        0 => {
            fighter.since_dodge = None;
            recovery(dodge.recovery_seconds, rate)
        }
        left => Phase::DodgeInvulnerable { left },
    }
}

fn advance<T: Topology>(
    who: Handle,
    fighter: &mut Fighter,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    rate: u32,
) {
    fighter.cooldown = fighter.cooldown.saturating_sub(1);
    // The counter window runs only while the fighter is free to use it, not during the dodge's
    // own recovery.
    if fighter.phase == Phase::Ready {
        fighter.counter = fighter.counter.saturating_sub(1);
    }
    if let Some(since) = fighter.since_dodge.as_mut() {
        *since += 1;
    }
    fighter.phase = match fighter.phase {
        Phase::Ready => Phase::Ready,
        Phase::Windup { action, left } => match left.saturating_sub(1) {
            0 => active(moveset, action, rate),
            left => Phase::Windup { action, left },
        },
        Phase::Active {
            action,
            left,
            struck,
        } => match left.saturating_sub(1) {
            0 => recovery(moveset.actions[action.0 as usize].recovery_seconds, rate),
            left => Phase::Active {
                action,
                left,
                struck,
            },
        },
        Phase::Recovery { left } => match left.saturating_sub(1) {
            0 => Phase::Ready,
            left => Phase::Recovery { left },
        },
        Phase::DodgeStartup { heading, left } => match left.saturating_sub(1) {
            0 => invulnerable(who, heading, fighter, moveset, map, occupancy, rate),
            left => Phase::DodgeStartup { heading, left },
        },
        Phase::DodgeInvulnerable { left } => match left.saturating_sub(1) {
            0 => {
                fighter.since_dodge = None;
                recovery(moveset.dodge.recovery_seconds, rate)
            }
            left => Phase::DodgeInvulnerable { left },
        },
    };
}

#[allow(clippy::too_many_arguments)]
fn start_buffered<T: Topology>(
    who: Handle,
    fighter: &mut Fighter,
    moveset: &Moveset,
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    rate: u32,
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
    let cooling = matches!(buffered.order, Order::Dodge { .. }) && fighter.cooldown > 0;
    if !free || cooling {
        let age = buffered.age + 1;
        if within(age, rate, buffer_seconds()) {
            fighter.buffered = Some(Buffered { age, ..buffered });
        } else {
            fighter.buffered = None;
            events.push(CombatEvent::Expired { who });
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
            let windup = steps_for(definition.windup_seconds, rate);
            fighter.phase = if fighter.countering() || windup == 0 {
                active(moveset, action, rate)
            } else {
                Phase::Windup {
                    action,
                    left: windup,
                }
            };
            fighter.counter = 0;
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
            fighter.cooldown = steps_for(dodge.cooldown_seconds, rate);
            fighter.since_dodge = Some(0);
            events.push(CombatEvent::DodgeStarted { who });
            fighter.phase = match steps_for(dodge.startup_seconds, rate) {
                0 => invulnerable(who, heading, fighter, moveset, map, occupancy, rate),
                left => Phase::DodgeStartup { heading, left },
            };
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn strike<T: Topology>(
    who: Handle,
    moveset: &Moveset,
    action: ActionId,
    from: lockstep_spatial::Cell,
    targets: Vec<(lockstep_spatial::Cell, Handle)>,
    fighters: &mut Column<Fighter>,
    movesets: &[Moveset],
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    rate: u32,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) {
    let packet = &moveset.actions[action.0 as usize].damage;
    let landed: Vec<Hit> = targets
        .into_iter()
        .filter_map(|(cell, target)| {
            let heading = direction(map, from, cell);
            hit_target(
                who, target, heading, packet, fighters, movesets, map, occupancy, rate, streams,
                events,
            )
        })
        .collect();
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

/// One packet against one target, from `who`: an invulnerable fighter dodges it unless it grabs or
/// cannot be avoided (emitting `Dodged` or `PerfectDodge` and returning `None`); otherwise it
/// resolves against the target's defence (a default one for a body with no `Fighter`), a stagger
/// interrupts a wind-up whose mask allows it or breaks a dodge in startup, and a knockback pushes
/// the target along `heading`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hit_target<T: Topology>(
    who: Handle,
    target: Handle,
    heading: Turn,
    packet: &DamagePacket,
    fighters: &mut Column<Fighter>,
    movesets: &[Moveset],
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    rate: u32,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) -> Option<Hit> {
    let piercing = packet.tags.contains(Tags::GRAB) || packet.tags.contains(Tags::UNAVOIDABLE);
    let mut no_fighter = None;
    let fighter = match fighters.get_mut(target) {
        Some(fighter) => fighter,
        None => no_fighter.insert(Fighter::new(
            MovesetId(0),
            Fixed32::ZERO,
            Defence::default(),
        )),
    };
    let theirs = movesets.get(fighter.moveset.0 as usize);
    if fighter.invulnerable() && !piercing {
        let window = theirs.map(|set| set.dodge.perfect_window_seconds);
        let perfect = matches!((fighter.since_dodge, window), (Some(since), Some(window)) if within(since, rate, window));
        if perfect {
            let dodge = &theirs.expect("a perfect window comes from a moveset").dodge;
            // One refund per dodge: the window closes once used.
            fighter.since_dodge = None;
            fighter.stamina = (fighter.stamina + dodge.stamina_cost).min(fighter.maximum_stamina);
            fighter.counter = steps_for(dodge.counter_seconds, rate);
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
        return None;
    }
    let result = resolve(
        packet,
        &fighter.defence,
        &mut fighter.evasion_memory,
        streams,
    );
    if result.staggered {
        match fighter.phase {
            Phase::Windup {
                action: their_action,
                ..
            } if theirs.is_some_and(|set| {
                set.actions[their_action.0 as usize]
                    .interruptible_by
                    .contains(InterruptMask::STAGGER)
            }) =>
            {
                fighter.phase = Phase::Ready;
                events.push(CombatEvent::Interrupted {
                    who: target,
                    action: their_action,
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
    let knocked = (result.knockback > 0 && !result.evaded)
        .then(|| knock_back(map, occupancy, target, heading, result.knockback));
    Some(Hit {
        target,
        result,
        knocked,
    })
}
