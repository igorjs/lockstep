// SPDX-License-Identifier: Apache-2.0
//! Projectiles: grid-stepped bodies that travel a line, hit the first body in their way, and stop
//! at walls. They are not in occupancy, so they never block a body.

use crate::damage::DamagePacket;
use crate::fighter::{hit_target, CombatEvent, Fighter, Moveset};
use crate::frame::{cuts_corner, Frame};
use crate::shape::direction;
use lockstep_core::math::Turn;
use lockstep_core::{Column, Handle, Message, StableVector, Streams};
use lockstep_spatial::{Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// Why a projectile stopped without hitting a body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stopped {
    /// A wall, a wall corner, a low wall at least its height, or the map edge.
    Wall,
    /// It descended to the ground.
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
    /// Height above the ground; a low wall at least this high stops it.
    pub height: u8,
    /// It drops one height every this many cells, standing in for gravity; 0 never drops.
    pub descent_every: u8,
    travelled: u32,
    range: u32,
}

/// What a projectile is launched with, apart from where and which way.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Launch {
    pub range_cells: u8,
    pub cells_per_step: u8,
    pub damage: Option<DamagePacket>,
    pub loudness: u8,
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
            height: launch.height,
            descent_every: launch.descent_every,
            travelled: 0,
            range: launch.range_cells as u32,
        }
    }

    /// The cells it still has to travel, nearest first.
    pub fn remaining(&self) -> impl Iterator<Item = Cell> + '_ {
        self.path.iter().rev().copied()
    }
}

/// Moves every projectile up to its cells per step, in handle order, and removes those that
/// stopped. At each cell: a wall, a cut wall corner or a low wall at least its height stops it
/// (`Wall`); otherwise it moves in and is heard there when it has a loudness; a body other than
/// its owner takes its damage through the same rules as a strike (a dodge lets it pass on), and a
/// noise-only projectile passes through bodies; then it may descend and land. A projectile that
/// runs out of cells is `Spent`.
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
                // Out of cells before its range means the line left the map.
                let reason = if projectile.travelled >= projectile.range {
                    Stopped::Spent
                } else {
                    Stopped::Wall
                };
                finished = Some(CombatEvent::ProjectileStopped {
                    projectile: handle,
                    at: projectile.at,
                    reason,
                });
                break;
            };
            // A full wall is as high as a wall gets, so height alone decides: a projectile flies
            // over a low wall below it and stops at anything at least as high.
            let wall_height = map.wall_height(next);
            if cuts_corner(map, projectile.at, next)
                || (wall_height > 0 && wall_height >= projectile.height)
            {
                finished = Some(CombatEvent::ProjectileStopped {
                    projectile: handle,
                    at: projectile.at,
                    reason: Stopped::Wall,
                });
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
            if let (Some(packet), Some(target)) = (&projectile.damage, occupancy.at(next)) {
                if target != projectile.owner {
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
                        finished = Some(CombatEvent::ProjectileHit {
                            by: projectile.owner,
                            projectile: handle,
                            hit,
                        });
                        break;
                    }
                }
            }
            if projectile.descent_every > 0
                && projectile
                    .travelled
                    .is_multiple_of(projectile.descent_every as u32)
            {
                projectile.height = projectile.height.saturating_sub(1);
                if projectile.height == 0 {
                    finished = Some(CombatEvent::ProjectileStopped {
                        projectile: handle,
                        at: projectile.at,
                        reason: Stopped::Landed,
                    });
                    break;
                }
            }
        }
        if let Some(event) = finished {
            events.push(event);
            projectiles.remove(handle);
        }
    }
}
