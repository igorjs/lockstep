// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TemplateId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct PoolId(pub u16);

/// What one cell of a template is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tile {
    /// Written as `.`: open ground.
    Open,
    /// Written as `#`: a wall.
    Wall,
    /// Written as `~`: passable, and slow.
    Marsh,
}

/// A square block of tiles, one chunk of a generated map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Template {
    pub name: String,
    tiles: Vec<Tile>,
}

impl Template {
    /// The tile at a column and row, counted from the template's top left.
    pub fn tile(&self, side: u32, x: u32, y: u32) -> Tile {
        self.tiles[(y * side + x) as usize]
    }
}

/// Weighted entries of templates to draw a chunk from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Pool {
    pub name: String,
    pub entries: Vec<(TemplateId, u32)>,
}

/// Every template and pool, in a stable order. The only way to build one is
/// `Templates::from_json`, which checks it, so a roll never meets a pool it cannot draw from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Templates {
    side: u32,
    templates: Vec<Template>,
    pools: Vec<Pool>,
}

/// The widest template, in cells.
pub const MAXIMUM_SIDE: u32 = 64;

/// The largest total weight of a pool: small enough that the draw's skew stays negligible.
pub const MAXIMUM_TOTAL_WEIGHT: u32 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplatesError {
    Json(String),
    /// A side of zero or past `MAXIMUM_SIDE`.
    BadSide(u32),
    TooMany,
    DuplicateName(String),
    /// A template whose rows are not `side` rows of `side` glyphs.
    BadShape(String),
    UnknownGlyph {
        template: String,
        glyph: char,
    },
    UnknownTemplate {
        pool: String,
        template: String,
    },
    /// No entries, or weights that add to nothing or past `MAXIMUM_TOTAL_WEIGHT`.
    Weights(String),
}

impl std::fmt::Display for TemplatesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TemplatesError {}

impl Templates {
    /// Reads the data form documented in `docs/lockstep-procgen.md`.
    pub fn from_json(text: &str) -> Result<Self, TemplatesError> {
        let file: file::TemplatesFile =
            serde_json::from_str(text).map_err(|error| TemplatesError::Json(error.to_string()))?;
        file.into_templates()
    }

    /// Cells along one side of every template.
    pub fn side(&self) -> u32 {
        self.side
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn template(&self, id: TemplateId) -> Option<&Template> {
        self.templates.get(id.0 as usize)
    }

    pub fn template_id(&self, name: &str) -> Option<TemplateId> {
        self.templates
            .iter()
            .position(|template| template.name == name)
            .map(|index| TemplateId(index as u16))
    }

    pub fn pool_id(&self, name: &str) -> Option<PoolId> {
        self.pools
            .iter()
            .position(|pool| pool.name == name)
            .map(|index| PoolId(index as u16))
    }

    pub(crate) fn pool(&self, id: PoolId) -> Option<&Pool> {
        self.pools.get(id.0 as usize)
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct TemplatesFile {
        side: u32,
        templates: Vec<TemplateFile>,
        pools: Vec<PoolFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct TemplateFile {
        name: String,
        rows: Vec<String>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PoolFile {
        name: String,
        entries: Vec<EntryFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EntryFile {
        template: String,
        weight: u32,
    }

    impl TemplatesFile {
        pub fn into_templates(self) -> Result<Templates, TemplatesError> {
            if self.side == 0 || self.side > MAXIMUM_SIDE {
                return Err(TemplatesError::BadSide(self.side));
            }
            if self.templates.len() > u16::MAX as usize || self.pools.len() > u16::MAX as usize {
                return Err(TemplatesError::TooMany);
            }
            let mut names = BTreeSet::new();
            let mut templates = Vec::new();
            for file in &self.templates {
                if !names.insert(file.name.clone()) {
                    return Err(TemplatesError::DuplicateName(file.name.clone()));
                }
                let side = self.side as usize;
                if file.rows.len() != side
                    || file.rows.iter().any(|row| row.chars().count() != side)
                {
                    return Err(TemplatesError::BadShape(file.name.clone()));
                }
                let mut tiles = Vec::with_capacity(side * side);
                for glyph in file.rows.iter().flat_map(|row| row.chars()) {
                    tiles.push(match glyph {
                        '.' => Tile::Open,
                        '#' => Tile::Wall,
                        '~' => Tile::Marsh,
                        glyph => {
                            return Err(TemplatesError::UnknownGlyph {
                                template: file.name.clone(),
                                glyph,
                            })
                        }
                    });
                }
                templates.push(Template {
                    name: file.name.clone(),
                    tiles,
                });
            }
            let mut pool_names = BTreeSet::new();
            let mut pools = Vec::new();
            for file in self.pools {
                if !pool_names.insert(file.name.clone()) {
                    return Err(TemplatesError::DuplicateName(file.name));
                }
                let mut entries = Vec::new();
                let mut total = 0u64;
                for entry in &file.entries {
                    let index = templates
                        .iter()
                        .position(|template| template.name == entry.template)
                        .ok_or_else(|| TemplatesError::UnknownTemplate {
                            pool: file.name.clone(),
                            template: entry.template.clone(),
                        })?;
                    total += entry.weight as u64;
                    entries.push((TemplateId(index as u16), entry.weight));
                }
                if total == 0 || total > MAXIMUM_TOTAL_WEIGHT as u64 {
                    return Err(TemplatesError::Weights(file.name));
                }
                pools.push(Pool {
                    name: file.name,
                    entries,
                });
            }
            Ok(Templates {
                side: self.side,
                templates,
                pools,
            })
        }
    }
}
