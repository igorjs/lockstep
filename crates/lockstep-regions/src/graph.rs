// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::{Fixed32, Vector2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct RegionId(pub u16);

/// One place, anchored to a point in metres.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub anchor: Vector2,
    /// How readily a condition takes hold here, 0 to 100 percent.
    pub receptive_percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionsError {
    Json(String),
    TooMany,
    DuplicateName(String),
    /// A link names a region that does not exist.
    UnknownRegion(String),
    /// A region linked to itself.
    SelfLink(String),
    /// A receptiveness above 100 percent.
    Receptive(String),
    /// An anchor more than 16,000 metres from the origin, so any two anchors differ by less than
    /// the 16.16 range.
    Anchor(String),
}

impl std::fmt::Display for RegionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RegionsError {}

/// Every region and the links between them, which run both ways. Build it with
/// `Regions::from_json`; it is saved but not loaded back, so its checks cannot be skipped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Regions {
    regions: Vec<Region>,
    /// Sorted neighbours of each region.
    neighbours: Vec<Vec<RegionId>>,
}

impl Regions {
    /// Reads the data form documented in `docs/lockstep-regions.md`.
    pub fn from_json(text: &str) -> Result<Self, RegionsError> {
        let file: file::RegionsFile =
            serde_json::from_str(text).map_err(|error| RegionsError::Json(error.to_string()))?;
        if file.regions.len() > u16::MAX as usize {
            return Err(RegionsError::TooMany);
        }
        let mut names = BTreeSet::new();
        let mut regions = Vec::new();
        for entry in file.regions {
            if !names.insert(entry.name.clone()) {
                return Err(RegionsError::DuplicateName(entry.name));
            }
            let receptive_percent = entry.receptive_percent.unwrap_or(100);
            if receptive_percent > 100 {
                return Err(RegionsError::Receptive(entry.name));
            }
            let metres = |value: i32| {
                (-16_000..=16_000)
                    .contains(&value)
                    .then(|| Fixed32::from_int(value))
                    .ok_or_else(|| RegionsError::Anchor(entry.name.clone()))
            };
            regions.push(Region {
                anchor: Vector2::new(metres(entry.x)?, metres(entry.y)?),
                name: entry.name.clone(),
                receptive_percent,
            });
        }
        let mut regions = Regions {
            neighbours: vec![Vec::new(); regions.len()],
            regions,
        };
        for (from, to) in file.links {
            let find = |name: &str| {
                regions
                    .region_id(name)
                    .ok_or_else(|| RegionsError::UnknownRegion(name.to_string()))
            };
            let (from_id, to_id) = (find(&from)?, find(&to)?);
            if from_id == to_id {
                return Err(RegionsError::SelfLink(from));
            }
            regions.neighbours[from_id.0 as usize].push(to_id);
            regions.neighbours[to_id.0 as usize].push(from_id);
        }
        for list in &mut regions.neighbours {
            list.sort();
            list.dedup();
        }
        Ok(regions)
    }

    pub fn len(&self) -> usize {
        self.regions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn region(&self, id: RegionId) -> Option<&Region> {
        self.regions.get(id.0 as usize)
    }

    pub fn region_id(&self, name: &str) -> Option<RegionId> {
        self.regions
            .iter()
            .position(|region| region.name == name)
            .map(|index| RegionId(index as u16))
    }

    /// Every region, in order.
    pub fn ids(&self) -> impl Iterator<Item = RegionId> {
        (0..self.regions.len()).map(|index| RegionId(index as u16))
    }

    /// The linked regions, lowest first. Nothing for an unknown region.
    pub fn neighbours(&self, id: RegionId) -> &[RegionId] {
        self.neighbours
            .get(id.0 as usize)
            .map_or(&[], Vec::as_slice)
    }

    /// The fewest links between two regions, or nothing when no chain of links joins them.
    pub fn hops(&self, from: RegionId, to: RegionId) -> Option<u32> {
        self.region(from)?;
        self.region(to)?;
        let mut distance = vec![u32::MAX; self.regions.len()];
        distance[from.0 as usize] = 0;
        let mut queue = VecDeque::from([from]);
        while let Some(current) = queue.pop_front() {
            for &next in self.neighbours(current) {
                if distance[next.0 as usize] == u32::MAX {
                    distance[next.0 as usize] = distance[current.0 as usize] + 1;
                    queue.push_back(next);
                }
            }
        }
        Some(distance[to.0 as usize]).filter(|&hops| hops != u32::MAX)
    }

    /// The region whose anchor is nearest a point; the lowest wins a tie. Nothing when there are
    /// no regions.
    pub fn nearest(&self, point: Vector2) -> Option<RegionId> {
        self.ids()
            .map(|id| (point.distance(self.regions[id.0 as usize].anchor), id))
            .min()
            .map(|(_, id)| id)
    }
}

mod file {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RegionsFile {
        pub regions: Vec<RegionFile>,
        #[serde(default)]
        pub links: Vec<(String, String)>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RegionFile {
        pub name: String,
        pub x: i32,
        pub y: i32,
        pub receptive_percent: Option<u32>,
    }
}
