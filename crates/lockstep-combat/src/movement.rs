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
    /// Steps spent blocked by a body since the last step taken.
    blocked: u32,
    /// Once blocked long enough, paths treat other bodies as walls until it arrives or stops.
    avoid_bodies: bool,
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
            avoid_bodies: false,
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
    /// Stopped beside a target another body holds.
    Halted {
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

/// Runs one step of movement: orders first, then each mover in handle order moves as far as its
/// speed allows (sliding around a body in the way, or finding a new path around bodies once blocked
/// long enough), then spends or restores stamina (from its `Fighter`, when it has one): running
/// while advancing spends it, and anything else, including waiting behind a body, restores it.
/// Movement moves single-cell bodies.
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
            MoveOrder::Stop => stop(mover),
            MoveOrder::MoveTo { target, gait } => {
                mover.gait = *gait;
                // Held at the destination, or already heading there: nothing to plan.
                if mover.target == Some(*target) || world.occupancy.cell_of(*who) == Some(*target) {
                    continue;
                }
                let was_idle = mover.target.is_none();
                mover.target = Some(*target);
                if !plan(*who, mover, world) {
                    events.push(MovementEvent::Unreachable { who: *who });
                    stop(mover);
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
        let gait = if mover.gait == Gait::Run && mover.exhausted {
            Gait::Walk
        } else {
            mover.gait
        };
        let advancing = walk(who, mover, gait, world, rate, events);
        if let Some(fighter) = fighters.get_mut(who) {
            let ran = advancing && gait == Gait::Run;
            spend_stamina(who, mover, fighter, world.rules, rate, ran, events);
        }
    }
}

fn stop(mover: &mut Mover) {
    mover.target = None;
    mover.path.clear();
    mover.progress = 0;
    mover.blocked = 0;
    mover.avoid_bodies = false;
}

/// What the first cell of a fresh path costs, in progress units.
fn first_cost<T: Topology>(
    who: Handle,
    mover: &Mover,
    world: &MovementWorld<'_, T>,
    rate: u64,
) -> u64 {
    match (world.occupancy.cell_of(who), mover.path.last()) {
        (Some(from), Some(next)) => cell_cost(world.map, from, *next, rate),
        _ => 0,
    }
}

/// A straight cell costs 65,536 units times the rate; a diagonal 1.4 times that.
fn cell_cost<T: Topology>(map: &GridMap<T>, from: Cell, to: Cell, rate: u64) -> u64 {
    map.distance(from, to) as u64 * 65_536 * rate / 10
}

fn spend_stamina(
    who: Handle,
    mover: &mut Mover,
    fighter: &mut Fighter,
    rules: &MovementRules,
    rate: u64,
    ran: bool,
    events: &mut Vec<MovementEvent>,
) {
    let per_step =
        |per_second: Fixed32| Fixed32::from_raw((per_second.raw().max(0) as u64 / rate) as i32);
    if ran {
        fighter.stamina =
            (fighter.stamina - per_step(rules.run_drain_per_second)).max(Fixed32::ZERO);
        if fighter.stamina == Fixed32::ZERO && !mover.exhausted {
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

/// Finds a path from the body's cell to its target, around other bodies too once the mover has
/// been blocked long enough.
fn plan<T: Topology>(who: Handle, mover: &mut Mover, world: &mut MovementWorld<'_, T>) -> bool {
    let (Some(target), Some(from)) = (mover.target, world.occupancy.cell_of(who)) else {
        return false;
    };
    let mut path = Vec::new();
    let options = PathOptions {
        treat_occupants_as_walls: mover.avoid_bodies,
        ..PathOptions::default()
    };
    let found = world
        .pathfinder
        .find(world.map, world.occupancy, from, target, options, &mut path);
    path.reverse();
    mover.path = path;
    matches!(found, PathResult::Found { .. })
}

/// Moves as far as the progress allows. Returns whether the body was advancing this step: moving
/// toward its target and not held back by a body, whether or not it entered a new cell.
fn walk<T: Topology>(
    who: Handle,
    mover: &mut Mover,
    gait: Gait,
    world: &mut MovementWorld<'_, T>,
    rate: u64,
    events: &mut Vec<MovementEvent>,
) -> bool {
    let Some(target) = mover.target else {
        return false;
    };
    let speed = match gait {
        Gait::Walk => world.rules.walk_cells_per_second,
        Gait::Run => world.rules.run_cells_per_second,
        Gait::Sneak => world.rules.sneak_cells_per_second,
    };
    mover.progress = mover.progress.saturating_add(speed.raw().max(0) as u64);
    loop {
        let Some(from) = world.occupancy.cell_of(who) else {
            return false;
        };
        if from == target {
            stop(mover);
            events.push(MovementEvent::Arrived { who, at: from });
            return true;
        }
        let Some(next) = mover.path.last().copied() else {
            stop(mover);
            events.push(MovementEvent::Unreachable { who });
            return false;
        };
        // The map may have changed since the path was planned: a closed door is planned around.
        if !world.map.can_step(from, next) {
            if !plan(who, mover, world) {
                stop(mover);
                events.push(MovementEvent::Unreachable { who });
                return false;
            }
            continue;
        }
        let taken = world.occupancy.at(next).is_some_and(|body| body != who);
        if taken && next == target {
            // The target itself is held: stop beside it.
            stop(mover);
            events.push(MovementEvent::Halted { who, at: from });
            return false;
        }
        let to = if taken {
            slide(who, from, next, world)
        } else {
            Some(next)
        };
        let Some(to) = to else {
            if mover.progress >= cell_cost(world.map, from, next, rate) {
                mover.blocked += 1;
                if !mover.avoid_bodies
                    && mover.blocked as u64 * 65_536
                        >= world.rules.blocked_repath_seconds.raw().max(0) as u64 * rate
                {
                    mover.avoid_bodies = true;
                    plan(who, mover, world);
                    events.push(MovementEvent::Repathed { who });
                }
            }
            // Hold at most one cell of progress for the step that frees the way.
            mover.progress = mover.progress.min(cell_cost(world.map, from, next, rate));
            return false;
        };
        // The cell actually entered sets the price: a sidestep on a diagonal costs a diagonal.
        let cost = cell_cost(world.map, from, to, rate);
        if mover.progress < cost {
            return true;
        }
        world
            .occupancy
            .move_to(who, to)
            .expect("the cell was checked free");
        mover.progress -= cost;
        mover.blocked = 0;
        mover.facing = quantise_facing(direction(world.map, from, to));
        if to == next {
            mover.path.pop();
        } else if !plan(who, mover, world) {
            stop(mover);
            events.push(MovementEvent::Unreachable { who });
            return false;
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
