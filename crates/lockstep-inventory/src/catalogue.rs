// SPDX-License-Identifier: Apache-2.0
use lockstep_attributes::{AttributeId, Modifier, Registry};
use lockstep_core::math::Fixed32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A kind's index in its catalogue.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct KindId(pub u16);

/// An equipment slot's index in its catalogue.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct SlotId(pub u16);

/// An affix's index in its catalogue.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AffixId(pub u16);

/// What every item of one kind shares.
#[derive(Clone, Debug, PartialEq)]
pub struct Kind {
    pub name: String,
    /// Sorted and without repeats.
    pub tags: Vec<String>,
    /// The weight of one unit.
    pub weight: Fixed32,
    /// The most units one item holds. One means the kind never stacks.
    pub stack: u16,
    /// Whole game minutes to spoil at 100 percent; `None` never spoils.
    pub spoils_after_minutes: Option<u32>,
    /// The slot it is worn in, when it can be worn.
    pub slot: Option<SlotId>,
    /// How many affixes one item can carry.
    pub affix_slots: u8,
    /// Applied to the wearer's attributes while worn.
    pub modifiers: Vec<(AttributeId, Modifier)>,
}

impl Kind {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags
            .binary_search_by(|held| held.as_str().cmp(tag))
            .is_ok()
    }
}

/// A property added to one item, such as a faulty seal.
#[derive(Clone, Debug, PartialEq)]
pub struct Affix {
    pub name: String,
    /// A bound item cannot be taken off while it carries this affix.
    pub binds: bool,
    /// Applied to the wearer's attributes while the item is worn.
    pub modifiers: Vec<(AttributeId, Modifier)>,
}

/// How fast food spoils at a temperature: the first band whose `below` is above the temperature,
/// or the last band, which has no `below`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band {
    pub below: Option<i32>,
    pub percent: u32,
}

/// Every kind, slot and affix, in a stable order, with the spoilage bands. Build it with
/// `Catalogue::from_json`, which checks it against the attribute registry.
#[derive(Clone, Debug, PartialEq)]
pub struct Catalogue {
    kinds: Vec<Kind>,
    slots: Vec<String>,
    affixes: Vec<Affix>,
    bands: Vec<Band>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogueError {
    Json(String),
    TooMany,
    DuplicateName(String),
    UnknownSlot {
        kind: String,
        slot: String,
    },
    UnknownAttribute {
        owner: String,
        attribute: String,
    },
    /// A stack of zero units.
    EmptyStack(String),
    NegativeWeight(String),
    /// The bands must rise strictly by `below`, and only the last has no `below`.
    Bands,
}

impl std::fmt::Display for CatalogueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CatalogueError {}

impl Catalogue {
    /// Reads the data form documented in `docs/lockstep-inventory.md`. Modifiers name attributes
    /// in `attributes`.
    pub fn from_json(text: &str, attributes: &Registry) -> Result<Self, CatalogueError> {
        let file: file::CatalogueFile =
            serde_json::from_str(text).map_err(|error| CatalogueError::Json(error.to_string()))?;
        file.into_catalogue(attributes)
    }

    pub fn kind(&self, id: KindId) -> &Kind {
        &self.kinds[id.0 as usize]
    }

    pub fn kind_id(&self, name: &str) -> Option<KindId> {
        position(self.kinds.iter().map(|kind| kind.name.as_str()), name).map(KindId)
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn slot_id(&self, name: &str) -> Option<SlotId> {
        position(self.slots.iter().map(String::as_str), name).map(SlotId)
    }

    pub fn slot_name(&self, id: SlotId) -> &str {
        &self.slots[id.0 as usize]
    }

    pub fn affix(&self, id: AffixId) -> &Affix {
        &self.affixes[id.0 as usize]
    }

    pub fn affix_id(&self, name: &str) -> Option<AffixId> {
        position(self.affixes.iter().map(|affix| affix.name.as_str()), name).map(AffixId)
    }

    /// The spoilage rate at a temperature, in percent of the rate at which `spoils_after_minutes`
    /// is counted.
    pub fn spoilage_percent(&self, temperature: i32) -> u32 {
        self.bands
            .iter()
            .find(|band| band.below.is_none_or(|below| temperature < below))
            .map_or(100, |band| band.percent)
    }

    fn check(&self) -> Result<(), CatalogueError> {
        if [self.kinds.len(), self.slots.len(), self.affixes.len()]
            .iter()
            .any(|count| *count > u16::MAX as usize)
        {
            return Err(CatalogueError::TooMany);
        }
        for names in [
            self.kinds.iter().map(|kind| &kind.name).collect::<Vec<_>>(),
            self.slots.iter().collect(),
            self.affixes.iter().map(|affix| &affix.name).collect(),
        ] {
            let mut seen = BTreeSet::new();
            if let Some(repeat) = names.into_iter().find(|name| !seen.insert(*name)) {
                return Err(CatalogueError::DuplicateName(repeat.clone()));
            }
        }
        for kind in &self.kinds {
            if kind.stack == 0 {
                return Err(CatalogueError::EmptyStack(kind.name.clone()));
            }
            if kind.weight < Fixed32::ZERO {
                return Err(CatalogueError::NegativeWeight(kind.name.clone()));
            }
        }
        let open_ended = self
            .bands
            .iter()
            .position(|band| band.below.is_none())
            .ok_or(CatalogueError::Bands)?;
        let rising = self.bands.windows(2).all(|pair| {
            matches!((pair[0].below, pair[1].below), (Some(a), Some(b)) if a < b)
                || pair[1].below.is_none()
        });
        if open_ended != self.bands.len() - 1 || !rising {
            return Err(CatalogueError::Bands);
        }
        Ok(())
    }
}

fn position<'a>(names: impl Iterator<Item = &'a str>, name: &str) -> Option<u16> {
    names
        .enumerate()
        .find(|(_, known)| *known == name)
        .map(|(index, _)| index as u16)
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct CatalogueFile {
        #[serde(default)]
        slots: Vec<String>,
        spoilage: Vec<BandFile>,
        #[serde(default)]
        affixes: Vec<AffixFile>,
        kinds: Vec<KindFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct BandFile {
        #[serde(default)]
        below: Option<i32>,
        percent: u32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AffixFile {
        name: String,
        #[serde(default)]
        binds: bool,
        #[serde(default)]
        modifiers: Vec<ModifierFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct KindFile {
        name: String,
        #[serde(default)]
        tags: Vec<String>,
        weight: Decimal,
        #[serde(default = "one")]
        stack: u16,
        #[serde(default)]
        spoils_after_minutes: Option<u32>,
        #[serde(default)]
        slot: Option<String>,
        #[serde(default)]
        affix_slots: u8,
        #[serde(default)]
        modifiers: Vec<ModifierFile>,
    }

    fn one() -> u16 {
        1
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ModifierFile {
        attribute: String,
        modifier: ModifierKindFile,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum ModifierKindFile {
        Add(Decimal),
        Multiply(Decimal),
        Override(Decimal),
    }

    #[derive(Deserialize)]
    #[serde(try_from = "DecimalFile")]
    struct Decimal(Fixed32);

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DecimalFile {
        Text(String),
        Whole(i64),
    }

    impl TryFrom<DecimalFile> for Decimal {
        type Error = String;
        fn try_from(value: DecimalFile) -> Result<Self, String> {
            match value {
                DecimalFile::Text(text) => Fixed32::parse_decimal(&text)
                    .map(Decimal)
                    .map_err(|error| format!("{text:?}: {error}")),
                DecimalFile::Whole(whole) => i32::try_from(whole)
                    .ok()
                    .filter(|whole| (-32_768..=32_767).contains(whole))
                    .map(|whole| Decimal(Fixed32::from_int(whole)))
                    .ok_or_else(|| format!("{whole} is outside the 16.16 range")),
            }
        }
    }

    fn modifiers(
        owner: &str,
        files: Vec<ModifierFile>,
        attributes: &Registry,
    ) -> Result<Vec<(AttributeId, Modifier)>, CatalogueError> {
        files
            .into_iter()
            .map(|file| {
                let id = attributes.id(&file.attribute).ok_or_else(|| {
                    CatalogueError::UnknownAttribute {
                        owner: owner.to_string(),
                        attribute: file.attribute.clone(),
                    }
                })?;
                let modifier = match file.modifier {
                    ModifierKindFile::Add(value) => Modifier::Add(value.0),
                    ModifierKindFile::Multiply(value) => Modifier::Multiply(value.0),
                    ModifierKindFile::Override(value) => Modifier::Override(value.0),
                };
                Ok((id, modifier))
            })
            .collect()
    }

    impl CatalogueFile {
        pub(super) fn into_catalogue(
            self,
            attributes: &Registry,
        ) -> Result<Catalogue, CatalogueError> {
            let slots = self.slots;
            let affixes = self
                .affixes
                .into_iter()
                .map(|file| {
                    Ok(Affix {
                        modifiers: modifiers(&file.name, file.modifiers, attributes)?,
                        name: file.name,
                        binds: file.binds,
                    })
                })
                .collect::<Result<Vec<_>, CatalogueError>>()?;
            let kinds = self
                .kinds
                .into_iter()
                .map(|file| {
                    let slot = match file.slot {
                        None => None,
                        Some(slot) => Some(
                            position(slots.iter().map(String::as_str), &slot)
                                .map(SlotId)
                                .ok_or_else(|| CatalogueError::UnknownSlot {
                                    kind: file.name.clone(),
                                    slot,
                                })?,
                        ),
                    };
                    let mut tags = file.tags;
                    tags.sort();
                    tags.dedup();
                    Ok(Kind {
                        modifiers: modifiers(&file.name, file.modifiers, attributes)?,
                        name: file.name,
                        tags,
                        weight: file.weight.0,
                        stack: file.stack,
                        spoils_after_minutes: file.spoils_after_minutes,
                        slot,
                        affix_slots: file.affix_slots,
                    })
                })
                .collect::<Result<Vec<_>, CatalogueError>>()?;
            let bands = self
                .spoilage
                .into_iter()
                .map(|band| Band {
                    below: band.below,
                    percent: band.percent,
                })
                .collect();
            let catalogue = Catalogue {
                kinds,
                slots,
                affixes,
                bands,
            };
            catalogue.check()?;
            Ok(catalogue)
        }
    }
}
