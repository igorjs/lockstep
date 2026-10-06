// SPDX-License-Identifier: Apache-2.0
//! Movement with weight: walk, run and sneak speeds, stamina that running spends, noise per step,
//! sliding around a body in the way, and facings in eight directions.

use crate::fighter::Fighter;
use crate::shape::{angle_between, direction};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{Column, Handle, Message};
use lockstep_spatial::{Cell, GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Topology};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gait {
    Walk,
    Run,
    /// Slow, and half as loud.
    Sneak,
}

/// The numbers movement runs on. `Default` is the reference's, on half-metre cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovementRules {
    pub walk_cells_per_second: Fixed32,
    pub run_cells_per_second: Fixed32,
    pub sneak_cells_per_second: Fixed32,
    /// Stamina running spends a second.
    pub run_drain_per_second: Fixed32,
    /// Stamina walking and standing restore a second.
    pub regenerate_per_second: Fixed32,
    /// Once stamina runs out, running waits until it is back to this.
    pub recover_at: Fixed32,
    /// A body blocked this long finds a new path around the bodies in its way.
    pub blocked_repath_seconds: Fixed32,
}

impl Default for MovementRules {
    /// Walk 1.8, run 4.2 and sneak 0.9 metres a second on half-metre cells; running drains 6
    /// stamina a second, walking and standing restore 4; an empty pool forces a walk until 15.
    fn default() -> Self {
        MovementRules {
            walk_cells_per_second: Fixed32::from_ratio(36, 10),
            run_cells_per_second: Fixed32::from_ratio(84, 10),
            sneak_cells_per_second: Fixed32::from_ratio(18, 10),
            run_drain_per_second: Fixed32::from_int(6),
            regenerate_per_second: Fixed32::from_int(4),
            recover_at: Fixed32::from_int(15),
            blocked_repath_seconds: Fixed32::HALF,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum MoveOrder {
    /// Sent again every step while held: a new target replaces the old one without losing the
    /// progress toward the next cell.
    MoveTo {
        target: Cell,
        gait: Gait,
    },
    Stop,
}

/// One body's movement state. Keep it in a `Column<Mover>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mover {
    pub target: Option<Cell>,
    pub gait: Gait,
    /// Rounded to the nearest of eight directions.
    pub facing: Turn,
    /// Cells still to walk, the next one last.
    path: Vec<Cell>,
    /// Progress toward the next cell, in sixty-five-thousand-five-hundred-thirty-sixths of a cell
    /// times the step rate, so whole steps add exactly.
    progress: u64,
    /// Steps spent blocked by a body.
    blocked: u32,
    /// Ran out of stamina: walks until it recovers.
    pub exhausted: bool,
}

impl Default for Mover {
    fn default() -> Self {
        Mover {
            target: None,
            gait: Gait::Walk,
            facing: 0,
            path: Vec::new(),
            progress: 0,
            blocked: 0,
            exhausted: false,
        }
    }
}

impl Mover {
    pub fn moving(&self) -> bool {
        self.target.is_some()
    }

    /// The cells still to walk, the next one first.
    pub fn remaining(&self) -> impl Iterator<Item = Cell> + '_ {
        self.path.iter().rev().copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum MovementEvent {
    /// A step from one cell to the next, heard within `noise_metres`.
    Moved {
        who: Handle,
        from: Cell,
        to: Cell,
        gait: Gait,
        noise_metres: Fixed32,
    },
    Arrived {
        who: Handle,
        at: Cell,
    },
    /// No path to the target.
    Unreachable {
        who: Handle,
    },
    /// Blocked long enough to find a new path around the bodies in the way.
    Repathed {
        who: Handle,
    },
    Exhausted {
        who: Handle,
    },
    Recovered {
        who: Handle,
    },
}

/// The facing nearest an angle, among eight directions.
pub fn quantise_facing(angle: Turn) -> Turn {
    (angle.wrapping_add(4_096) / 8_192).wrapping_mul(8_192)
}

/// Everything one movement step reads besides the movers.
pub struct MovementWorld<'a, T: Topology> {
    pub map: &'a GridMap<T>,
    pub occupancy: &'a mut Occupancy,
    pub pathfinder: &'a mut Pathfinder,
    pub rules: &'a MovementRules,
    pub steps_per_second: u32,
    /// How far one walking step on a cell is heard, in metres: 6 on stone, 4 on grass, 9 on gravel.
    pub walking_noise: &'a dyn Fn(Cell) -> Fixed32,
}

/// Runs one step of movement: orders first, then each mover in handle order spends or restores
/// stamina (from its `Fighter`, when it has one) and moves as far as its speed allows, sliding
/// around a body in the way or finding a new path once blocked long enough.
pub fn step_movement<T: Topology>(
    movers: &mut Column<Mover>,
    fighters: &mut Column<Fighter>,
    world: &mut MovementWorld<'_, T>,
    orders: &[(Handle, MoveOrder)],
    events: &mut Vec<MovementEvent>,
) {
    let rate = world.steps_per_second.max(1) as u64;
    for (who, order) in orders {
        let Some(mover) = movers.get_mut(*who) else {
            continue;
        };
        match order {
            MoveOrder::Stop => {
                mover.target = None;
                mover.path.clear();
                mover.progress = 0;
            }
            MoveOrder::MoveTo { target, gait } => {
                mover.gait = *gait;
                if mover.target == Some(*target) {
                    continue;
                }
                let was_idle = mover.target.is_none();
                mover.target = Some(*target);
                if !plan(*who, mover, world, false) {
                    events.push(MovementEvent::Unreachable { who: *who });
                    mover.target = None;
                    continue;
                }
                if was_idle {
                    // The first cell is paid for up front, so it is taken on the step the order
                    // arrives: one step of latency.
                    mover.progress = first_cost(*who, mover, world, rate);
                }
            }
        }
    }
    for who in movers.handles() {
        let mover = movers.get_mut(who).expect("listed");
        let running = mover.moving() && mover.gait == Gait::Run && !mover.exhausted;
        if let Some(fighter) = fighters.get_mut(who) {
            spend_stamina(who, mover, fighter, world.rules, rate, running, events);
        }
        let gait = if mover.gait == Gait::Run && mover.exhausted {
            Gait::Walk
        } else {
            mover.gait
        };
        walk(who, mover, gait, world, rate, events);
    }
}

/// What the first cell of a fresh path costs, in progress units.
fn first_cost<T: Topology>(
    who: Handle,
    mover: &Mover,
    world: &MovementWorld<'_, T>,
    rate: u64,
) -> u64 {
    match (world.occupancy.cell_of(who), mover.path.last()) {
        (Some(from), Some(next)) => world.map.distance(from, *next) as u64 * 65_536 * rate / 10,
        _ => 0,
    }
}

fn spend_stamina(
    who: Handle,
    mover: &mut Mover,
    fighter: &mut Fighter,
    rules: &MovementRules,
    rate: u64,
    running: bool,
    events: &mut Vec<MovementEvent>,
) {
    let per_step =
        |per_second: Fixed32| Fixed32::from_raw((per_second.raw().max(0) as u64 / rate) as i32);
    if running {
        fighter.stamina =
            (fighter.stamina - per_step(rules.run_drain_per_second)).max(Fixed32::ZERO);
        if fighter.stamina == Fixed32::ZERO {
            mover.exhausted = true;
            events.push(MovementEvent::Exhausted { who });
        }
    } else {
        fighter.stamina =
            (fighter.stamina + per_step(rules.regenerate_per_second)).min(fighter.maximum_stamina);
        if mover.exhausted && fighter.stamina >= rules.recover_at {
            mover.exhausted = false;
            events.push(MovementEvent::Recovered { who });
        }
    }
}

/// Finds a path from the body's cell to its target, treating other bodies as walls when asked.
fn plan<T: Topology>(
    who: Handle,
    mover: &mut Mover,
    world: &mut MovementWorld<'_, T>,
    bodies_as_walls: bool,
) -> bool {
    let (Some(target), Some(from)) = (mover.target, world.occupancy.cell_of(who)) else {
        return false;
    };
    let mut path = Vec::new();
    let options = PathOptions {
        treat_occupants_as_walls: bodies_as_walls,
        ..PathOptions::default()
    };
    let found = world
        .pathfinder
        .find(world.map, world.occupancy, from, target, options, &mut path);
    path.reverse();
    mover.path = path;
    mover.blocked = 0;
    matches!(found, PathResult::Found { .. })
}

fn walk<T: Topology>(
    who: Handle,
    mover: &mut Mover,
    gait: Gait,
    world: &mut MovementWorld<'_, T>,
    rate: u64,
    events: &mut Vec<MovementEvent>,
) {
    let Some(target) = mover.target else {
        return;
    };
    let speed = match gait {
        Gait::Walk => world.rules.walk_cells_per_second,
        Gait::Run => world.rules.run_cells_per_second,
        Gait::Sneak => world.rules.sneak_cells_per_second,
    };
    mover.progress = mover.progress.saturating_add(speed.raw().max(0) as u64);
    loop {
        let Some(from) = world.occupancy.cell_of(who) else {
            return;
        };
        if from == target || mover.path.is_empty() {
            mover.target = None;
            mover.path.clear();
            mover.progress = 0;
            events.push(if from == target {
                MovementEvent::Arrived { who, at: from }
            } else {
                MovementEvent::Unreachable { who }
            });
            return;
        }
        let next = *mover.path.last().expect("checked above");
        // A straight cell costs 65,536 units times the rate; a diagonal 1.4 times that.
        let cost = world.map.distance(from, next) as u64 * 65_536 * rate / 10;
        if mover.progress < cost {
            return;
        }
        let step_to = if world.occupancy.at(next).is_some_and(|body| body != who) {
            slide(who, from, next, world)
        } else {
            Some(next)
        };
        let Some(to) = step_to else {
            mover.blocked += 1;
            if mover.blocked as u64 * 65_536
                >= world.rules.blocked_repath_seconds.raw().max(0) as u64 * rate
            {
                plan(who, mover, world, true);
                events.push(MovementEvent::Repathed { who });
            }
            // Hold the progress for the step that frees the way.
            mover.progress = mover.progress.min(cost);
            return;
        };
        world
            .occupancy
            .move_to(who, to)
            .expect("the cell was checked free");
        mover.progress -= cost.min(mover.progress);
        mover.blocked = 0;
        mover.facing = quantise_facing(direction(world.map, from, to));
        if to == next {
            mover.path.pop();
        } else {
            plan(who, mover, world, false);
        }
        let factor = match gait {
            Gait::Walk => Fixed32::ONE,
            Gait::Run => Fixed32::from_int(2),
            Gait::Sneak => Fixed32::HALF,
        };
        events.push(MovementEvent::Moved {
            who,
            from,
            to,
            gait,
            noise_metres: (world.walking_noise)(to) * factor,
        });
    }
}

/// When the next cell is taken, the two neighbours on either side of the way, nearest the
/// intended direction first, that the body can step into and that are free.
fn slide<T: Topology>(
    who: Handle,
    from: Cell,
    blocked: Cell,
    world: &MovementWorld<'_, T>,
) -> Option<Cell> {
    let intended = direction(world.map, from, blocked);
    let mut neighbours = Vec::new();
    world.map.neighbours(from, &mut neighbours);
    let mut candidates: Vec<(Turn, Cell)> = neighbours
        .into_iter()
        .filter(|cell| *cell != blocked)
        .map(|cell| {
            (
                angle_between(direction(world.map, from, cell), intended),
                cell,
            )
        })
        .collect();
    candidates.sort_unstable();
    candidates
        .into_iter()
        .take(2)
        .find(|(_, cell)| {
            world.map.can_step(from, *cell)
                && world.occupancy.at(*cell).is_none_or(|body| body == who)
        })
        .map(|(_, cell)| cell)
}
