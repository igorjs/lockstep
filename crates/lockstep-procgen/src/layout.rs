// SPDX-License-Identifier: Apache-2.0
use crate::templates::{PoolId, TemplateId, Templates, Tile};
use lockstep_core::Streams;
use lockstep_spatial::{GridMap, Topology};
use serde::{Deserialize, Serialize};

/// The stream each chunk draws its template from. Every chunk has a stream of its own.
pub const STREAM: &str = "procgen";

/// The cost of walking a marsh cell.
const MARSH_COST: u8 = 3;

/// A chunk fixed to a template: the draw is skipped there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reservation {
    pub x: u32,
    pub y: u32,
    pub template: TemplateId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollError {
    UnknownPool,
    /// No chunks to roll.
    Empty,
    /// The reservation at this index is outside the layout, or names no template.
    BadReservation(usize),
    /// Two reservations name the same chunk; this is the later index.
    Reserved(usize),
    /// `apply` met a map too small for the layout.
    MapTooSmall,
}

impl std::fmt::Display for RollError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RollError {}

/// The seed of one chunk: a function of the world seed and the chunk's place alone, so a chunk
/// rolls the same whatever size the layout is or whichever chunk is rolled first. For a fixed
/// world seed the map from places to seeds is one to one, because each step of the mix is a
/// bijection of 64 bits; across world seeds a collision is as likely as any 64-bit collision.
pub fn chunk_seed(world_seed: u64, x: u32, y: u32) -> u64 {
    mix(mix(world_seed) ^ (((x as u64) << 32) | y as u64))
}

/// The splitmix64 finalizer: a bijection of 64 bits with good diffusion.
fn mix(value: u64) -> u64 {
    let mut value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// A grid of chunks, each a template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    width: u32,
    height: u32,
    picks: Vec<TemplateId>,
}

impl Templates {
    /// Rolls a layout of `width` by `height` chunks. Each chunk not reserved draws its template
    /// from the pool by weight, from its own stream seeded by `chunk_seed`.
    pub fn roll(
        &self,
        world_seed: u64,
        width: u32,
        height: u32,
        pool: PoolId,
        reserved: &[Reservation],
    ) -> Result<Layout, RollError> {
        let pool = self.pool(pool).ok_or(RollError::UnknownPool)?;
        if width == 0 || height == 0 {
            return Err(RollError::Empty);
        }
        let mut fixed: Vec<Option<TemplateId>> = vec![None; (width * height) as usize];
        for (index, reservation) in reserved.iter().enumerate() {
            if reservation.x >= width
                || reservation.y >= height
                || self.template(reservation.template).is_none()
            {
                return Err(RollError::BadReservation(index));
            }
            let slot = &mut fixed[(reservation.y * width + reservation.x) as usize];
            if slot.is_some() {
                return Err(RollError::Reserved(index));
            }
            *slot = Some(reservation.template);
        }
        let total: u32 = pool.entries.iter().map(|(_, weight)| weight).sum();
        let mut picks = Vec::with_capacity(fixed.len());
        for y in 0..height {
            for x in 0..width {
                if let Some(template) = fixed[(y * width + x) as usize] {
                    picks.push(template);
                    continue;
                }
                let mut streams = Streams::new(chunk_seed(world_seed, x, y));
                let draw = streams.range(STREAM, 0, total as i32) as u32;
                let mut running = 0;
                for (template, weight) in &pool.entries {
                    running += weight;
                    if draw < running {
                        picks.push(*template);
                        break;
                    }
                }
            }
        }
        Ok(Layout {
            width,
            height,
            picks,
        })
    }
}

impl Layout {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The template of the chunk at a column and row.
    pub fn template_at(&self, x: u32, y: u32) -> Option<TemplateId> {
        (x < self.width && y < self.height).then(|| self.picks[(y * self.width + x) as usize])
    }

    /// Writes the layout onto a map, chunk by chunk from its top left. Every cell is written
    /// through the map's setters, so the chunks it changes are marked dirty. A map smaller than
    /// the layout is refused and left alone.
    pub fn apply<T: Topology>(
        &self,
        templates: &Templates,
        map: &mut GridMap<T>,
    ) -> Result<(), RollError> {
        let side = templates.side();
        if map.width() < self.width * side || map.height() < self.height * side {
            return Err(RollError::MapTooSmall);
        }
        for chunk_y in 0..self.height {
            for chunk_x in 0..self.width {
                let id = self.picks[(chunk_y * self.width + chunk_x) as usize];
                let template = templates.template(id).ok_or(RollError::UnknownPool)?;
                for y in 0..side {
                    for x in 0..side {
                        let cell = map.index(chunk_x * side + x, chunk_y * side + y);
                        let tile = template.tile(side, x, y);
                        map.set_passable(cell, tile != Tile::Wall);
                        map.set_cost(cell, if tile == Tile::Marsh { MARSH_COST } else { 1 });
                    }
                }
            }
        }
        Ok(())
    }
}
