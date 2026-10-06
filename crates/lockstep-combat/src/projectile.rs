// SPDX-License-Identifier: Apache-2.0
//! Projectiles: grid-stepped bodies that travel a line, hit the first body in their way, and stop
//! at walls. They are not in occupancy, so they never block a body.

use crate::damage::DamagePacket;
use crate::fighter::{hit_target, CombatEvent, Fighter, Moveset};
use crate::frame::Frame;
use crate::shape::direction;
use lockstep_core::math::Turn;
use lockstep_core::{Column, Handle, Message, StableVector, Streams};
use lockstep_spatial::{Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// Why a projectile stopped without hitting a body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stopped {
    /// A wall, a wall corner or rising ground at least its altitude, or the map edge.
    Wall,
    /// It descended to the ground, or the ground rose to meet it.
    Landed,
    /// It travelled its whole range.
    Spent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Projectile {
    pub owner: Handle,
    pub at: Cell,
    /// The cells still to travel, nearest first.
    path: Vec<Cell>,
    pub cells_per_step: u8,
    /// `None` for a projectile that only makes noise, such as a scream.
    pub damage: Option<DamagePacket>,
    /// How far each cell it passes is heard, in steps; 0 is silent.
    pub loudness: u8,
    /// Absolute altitude: the launch cell's elevation plus the launch height. A cell whose
    /// elevation plus wall height reaches it stops the projectile, and it lands where the ground
    /// rises to meet it.
    pub altitude: u16,
    /// It drops one level every this many cells, standing in for gravity; 0 never drops.
    pub descent_every: u8,
    travelled: u32,
    range: u32,
    /// The last body it reached, so a body wider than one cell is met once.
    last_body: Option<Handle>,
}

/// What a projectile is launched with, apart from where and which way.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Launch {
    pub range_cells: u8,
    pub cells_per_step: u8,
    pub damage: Option<DamagePacket>,
    pub loudness: u8,
    /// Height above the launch cell's ground.
    pub height: u8,
    pub descent_every: u8,
}

impl Projectile {
    /// A projectile leaving `from` toward the cell `range_cells` steps along `heading`, chosen as
    /// if the map had no edge.
    pub fn launch<T: Topology>(
        map: &GridMap<T>,
        owner: Handle,
        from: Cell,
        heading: Turn,
        launch: &Launch,
    ) -> Self {
        let frame = Frame::around(map, launch.range_cells as u32);
        let mut path = frame.path_ahead::<T>(from, heading, launch.range_cells as u32);
        path.reverse();
        Projectile {
            owner,
            at: from,
            path,
            cells_per_step: launch.cells_per_step.max(1),
            damage: launch.damage.clone(),
            loudness: launch.loudness,
            altitude: map.elevation(from) as u16 + launch.height as u16,
            descent_every: launch.descent_every,
            travelled: 0,
            range: launch.range_cells as u32,
            last_body: None,
        }
    }

    /// The cells it still has to travel, nearest first.
    pub fn remaining(&self) -> impl Iterator<Item = Cell> + '_ {
        self.path.iter().rev().copied()
    }
}

/// How high a cell reaches: its ground plus its wall. A full wall reaches any altitude.
fn top<T: Topology>(map: &GridMap<T>, cell: Cell) -> u16 {
    if !map.is_passable(cell) && map.wall_height(cell) == u8::MAX {
        u16::MAX
    } else {
        map.elevation(cell) as u16 + map.wall_height(cell) as u16
    }
}

/// Whether a projectile at `altitude` can enter `to` from `from`: the cell must sit below it and,
/// on a diagonal, so must both cells it passes between.
fn clears<T: Topology>(map: &GridMap<T>, from: Cell, to: Cell, altitude: u16) -> bool {
    if top(map, to) >= altitude {
        return false;
    }
    if map.distance(from, to) <= 10 {
        return true;
    }
    let ((from_x, from_y), (to_x, to_y)) = (map.coordinates(from), map.coordinates(to));
    top(map, map.index(from_x, to_y)) < altitude && top(map, map.index(to_x, from_y)) < altitude
}

/// Moves every projectile up to its cells per step, in handle order, and removes those that
/// stopped. At each cell: a cell (or, on a diagonal, a corner) that reaches its altitude stops it
/// (`Wall`), whether a wall or rising ground; otherwise it moves in and is heard there when it has
/// a loudness; the first cell of a body other than its owner takes its damage through the same
/// rules as a strike (a dodge lets it fly on), and a noise-only projectile passes through bodies;
/// then it may descend, landing where its altitude meets the ground. A projectile out of cells
/// stops on that step: `Spent` at its full range, `Wall` when the line left the map.
#[allow(clippy::too_many_arguments)]
pub fn step_projectiles<T: Topology>(
    projectiles: &mut StableVector<Projectile>,
    fighters: &mut Column<Fighter>,
    movesets: &[Moveset],
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    steps_per_second: u32,
    streams: &mut Streams,
    events: &mut Vec<CombatEvent>,
) {
    let rate = steps_per_second.max(1);
    for handle in projectiles.handles() {
        let projectile = projectiles.get_mut(handle).expect("listed");
        let mut finished = None;
        for _ in 0..projectile.cells_per_step {
            let Some(next) = projectile.path.last().copied() else {
                break;
            };
            if !clears(map, projectile.at, next, projectile.altitude) {
                finished = Some(Stopped::Wall);
                break;
            }
            let heading = direction(map, projectile.at, next);
            projectile.path.pop();
            projectile.at = next;
            projectile.travelled += 1;
            if projectile.loudness > 0 {
                events.push(CombatEvent::Sounded {
                    by: projectile.owner,
                    at: next,
                    loudness: projectile.loudness,
                });
            }
            let body = occupancy.at(next).filter(|body| *body != projectile.owner);
            let new_body = body.filter(|body| projectile.last_body != Some(*body));
            projectile.last_body = body;
            if let (Some(packet), Some(target)) = (&projectile.damage, new_body) {
                let hit = hit_target(
                    projectile.owner,
                    target,
                    heading,
                    packet,
                    fighters,
                    movesets,
                    map,
                    occupancy,
                    rate,
                    streams,
                    events,
                );
                if let Some(hit) = hit {
                    events.push(CombatEvent::ProjectileHit {
                        by: projectile.owner,
                        projectile: handle,
                        hit,
                    });
                    projectiles.remove(handle);
                    break;
                }
            }
            if projectile.descent_every > 0
                && projectile
                    .travelled
                    .is_multiple_of(projectile.descent_every as u32)
            {
                projectile.altitude = projectile.altitude.saturating_sub(1);
            }
            if projectile.altitude <= map.elevation(next) as u16 {
                finished = Some(Stopped::Landed);
                break;
            }
        }
        let Some(projectile) = projectiles.get(handle) else {
            continue;
        };
        // Out of cells: stop on this step, not the next.
        if finished.is_none() && projectile.path.is_empty() {
            finished = Some(if projectile.travelled >= projectile.range {
                Stopped::Spent
            } else {
                Stopped::Wall
            });
        }
        if let Some(reason) = finished {
            events.push(CombatEvent::ProjectileStopped {
                projectile: handle,
                at: projectile.at,
                reason,
            });
            projectiles.remove(handle);
        }
    }
}
