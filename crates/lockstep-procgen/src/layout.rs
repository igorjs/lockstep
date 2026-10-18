// SPDX-License-Identifier: Apache-2.0
use crate::templates::{Pool, PoolId, TemplateId, Templates, Tile, EAST, NORTH, SOUTH, WEST};
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
    /// `apply` met a layout that names a template these templates do not hold.
    UnknownTemplate,
    /// More than `MAXIMUM_CHUNKS` chunks.
    TooLarge,
    /// `roll_matched` met templates without edge sockets.
    NoSockets,
    /// The search proved that no layout joins up on every edge with these reservations.
    NoLayout,
    /// The search spent its whole budget (see `search_budget`) without a layout or a proof.
    SearchLimit,
}

/// The most chunks one layout may hold.
pub const MAXIMUM_CHUNKS: u32 = 1 << 20;

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
        if (width as u64) * (height as u64) > MAXIMUM_CHUNKS as u64 {
            return Err(RollError::TooLarge);
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

/// The most template tries a matched roll of this many chunks may spend: a fixed number per
/// chunk, so the bound is the same on every platform.
pub fn search_budget(chunks: u32) -> u64 {
    chunks as u64 * 32 + 1024
}

impl Templates {
    /// Rolls a layout whose neighbouring chunks agree on the socket of the edge they share
    /// (the east socket of a chunk equals the west socket of the chunk to its east, and the
    /// south socket equals the north socket of the one below). The edges of the layout are free.
    ///
    /// Chunks are placed in reading order. Each chunk tries the pool's templates in an order
    /// drawn by weight, without repeats, from its own stream seeded by `chunk_seed`, and takes
    /// the first that fits its placed neighbours and its reserved ones. When none fits, the search
    /// steps back to the previous free chunk and tries its next template. With nothing in the
    /// way the first try is the draw `roll` makes. The search spends at most `search_budget`
    /// tries; it returns `NoLayout` when it has shown that none exists and `SearchLimit` when
    /// the budget ran out first. The result is a function of the seed, the size, the pool and the
    /// reservations alone.
    pub fn roll_matched(
        &self,
        world_seed: u64,
        width: u32,
        height: u32,
        pool: PoolId,
        reserved: &[Reservation],
    ) -> Result<Layout, RollError> {
        if !self.has_sockets() {
            return Err(RollError::NoSockets);
        }
        let pool = self.pool(pool).ok_or(RollError::UnknownPool)?;
        if width == 0 || height == 0 {
            return Err(RollError::Empty);
        }
        if (width as u64) * (height as u64) > MAXIMUM_CHUNKS as u64 {
            return Err(RollError::TooLarge);
        }
        let count = (width * height) as usize;
        let mut fixed: Vec<Option<TemplateId>> = vec![None; count];
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
        let mut picks: Vec<Option<TemplateId>> = fixed.clone();
        // Reservations that cannot sit together are known before any search.
        for (index, template) in fixed.iter().enumerate() {
            let place = (index as u32 % width, index as u32 / width);
            if let Some(template) = template {
                if !self.fits(pool, &mut picks, width, height, place, *template) {
                    return Err(RollError::NoLayout);
                }
            }
        }
        // The templates still to try at each chunk on the path, and how many were tried.
        let mut order: Vec<Option<(Vec<TemplateId>, usize)>> = vec![None; count];
        let mut remaining = search_budget(width * height);
        let mut at = 0usize;
        while at < count {
            let (x, y) = (at as u32 % width, at as u32 / width);
            let tries = order[at].get_or_insert_with(|| {
                let list = match fixed[at] {
                    Some(template) => vec![template],
                    None => self.draw_order(pool, chunk_seed(world_seed, x, y)),
                };
                (list, 0)
            });
            let mut found = None;
            while tries.1 < tries.0.len() {
                let candidate = tries.0[tries.1];
                tries.1 += 1;
                if remaining == 0 {
                    return Err(RollError::SearchLimit);
                }
                remaining -= 1;
                if self.fits(pool, &mut picks, width, height, (x, y), candidate) {
                    found = Some(candidate);
                    break;
                }
            }
            match found {
                Some(candidate) => {
                    picks[at] = Some(candidate);
                    at += 1;
                }
                None => {
                    order[at] = None;
                    // Step back to the nearest chunk that is free to change.
                    loop {
                        if at == 0 {
                            return Err(RollError::NoLayout);
                        }
                        at -= 1;
                        if fixed[at].is_none() {
                            picks[at] = None;
                            break;
                        }
                        order[at] = None;
                    }
                }
            }
        }
        Ok(Layout {
            width,
            height,
            picks: picks.into_iter().flatten().collect(),
        })
    }

    /// The pool's templates in the order one chunk tries them: repeated weighted draws, each
    /// from what is left, keeping a template at its first draw.
    fn draw_order(&self, pool: &Pool, seed: u64) -> Vec<TemplateId> {
        let mut streams = Streams::new(seed);
        let mut left: Vec<(TemplateId, u32)> = pool.entries.clone();
        let mut order: Vec<TemplateId> = Vec::new();
        while !left.is_empty() {
            let total: u32 = left.iter().map(|(_, weight)| weight).sum();
            if total == 0 {
                break;
            }
            let draw = streams.range(STREAM, 0, total as i32) as u32;
            let mut running = 0;
            let mut chosen = 0;
            for (index, (_, weight)) in left.iter().enumerate() {
                running += weight;
                if draw < running {
                    chosen = index;
                    break;
                }
            }
            let template = left.remove(chosen).0;
            if !order.contains(&template) {
                order.push(template);
            }
        }
        order
    }

    /// Whether a template at a place agrees with the neighbours already holding a template, and
    /// leaves every free neighbour at least one template of the pool that still agrees.
    fn fits(
        &self,
        pool: &Pool,
        picks: &mut [Option<TemplateId>],
        width: u32,
        height: u32,
        at: (u32, u32),
        candidate: TemplateId,
    ) -> bool {
        if !self.agrees(picks, width, height, at, candidate) {
            return false;
        }
        let index = (at.1 * width + at.0) as usize;
        let before = picks[index];
        picks[index] = Some(candidate);
        let mut fits = true;
        for (_, _, neighbour_x, neighbour_y) in Self::neighbours(width, height, at) {
            if picks[(neighbour_y * width + neighbour_x) as usize].is_some() {
                continue;
            }
            if !pool.entries.iter().any(|(option, weight)| {
                *weight > 0
                    && self.agrees(picks, width, height, (neighbour_x, neighbour_y), *option)
            }) {
                fits = false;
                break;
            }
        }
        picks[index] = before;
        fits
    }

    /// The places next to a place on the layout, with the side of each that faces the place.
    fn neighbours(width: u32, height: u32, (x, y): (u32, u32)) -> Vec<(usize, usize, u32, u32)> {
        let mut found = Vec::with_capacity(4);
        if y > 0 {
            found.push((NORTH, SOUTH, x, y - 1));
        }
        if x + 1 < width {
            found.push((EAST, WEST, x + 1, y));
        }
        if y + 1 < height {
            found.push((SOUTH, NORTH, x, y + 1));
        }
        if x > 0 {
            found.push((WEST, EAST, x - 1, y));
        }
        found
    }

    /// Whether a template at a place agrees with every neighbour already holding a template.
    fn agrees(
        &self,
        picks: &[Option<TemplateId>],
        width: u32,
        height: u32,
        at: (u32, u32),
        candidate: TemplateId,
    ) -> bool {
        let Some(template) = self.template(candidate) else {
            return false;
        };
        Self::neighbours(width, height, at)
            .iter()
            .all(|(own, theirs, neighbour_x, neighbour_y)| {
                match picks[(neighbour_y * width + neighbour_x) as usize]
                    .and_then(|id| self.template(id))
                {
                    Some(other) => template.socket(*own) == other.socket(*theirs),
                    None => true,
                }
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
        if self
            .picks
            .iter()
            .any(|id| templates.template(*id).is_none())
        {
            return Err(RollError::UnknownTemplate);
        }
        for chunk_y in 0..self.height {
            for chunk_x in 0..self.width {
                let id = self.picks[(chunk_y * self.width + chunk_x) as usize];
                let template = templates.template(id).ok_or(RollError::UnknownTemplate)?;
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
