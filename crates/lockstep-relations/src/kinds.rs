// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::Fixed32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct RelationId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct GroupId(pub u16);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Threshold {
    pub at: Fixed32,
    pub name: String,
}

/// One kind of standing, such as trust or reputation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub name: String,
    pub minimum: Fixed32,
    pub maximum: Fixed32,
    /// Where a standing starts, before anything changes it.
    pub starting: Fixed32,
    /// Where a standing drifts back to.
    pub rest: Fixed32,
    /// Points a game day a standing drifts toward rest.
    pub decay_per_day: Fixed32,
    /// Sorted by `at`.
    pub thresholds: Vec<Threshold>,
}

/// Every relation and group, in a stable order. Build it with `Kinds::from_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kinds {
    pub relations: Vec<Relation>,
    pub groups: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KindsError {
    Json(String),
    TooMany,
    DuplicateName(String),
    /// The minimum is above the maximum, or the starting or rest value is outside them.
    Bounds(String),
    /// A threshold outside the bounds.
    ThresholdOutside {
        relation: String,
        threshold: String,
    },
    NegativeDecay(String),
}

impl std::fmt::Display for KindsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for KindsError {}

impl Kinds {
    /// Reads the data form documented in `docs/lockstep-relations.md`.
    pub fn from_json(text: &str) -> Result<Self, KindsError> {
        let file: file::KindsFile =
            serde_json::from_str(text).map_err(|error| KindsError::Json(error.to_string()))?;
        let kinds = file.into_kinds();
        kinds.check()?;
        Ok(kinds)
    }

    pub fn relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(id.0 as usize)
    }

    pub fn relation_id(&self, name: &str) -> Option<RelationId> {
        self.relations
            .iter()
            .position(|relation| relation.name == name)
            .map(|index| RelationId(index as u16))
    }

    pub fn group_id(&self, name: &str) -> Option<GroupId> {
        self.groups
            .iter()
            .position(|group| group == name)
            .map(|index| GroupId(index as u16))
    }

    fn check(&self) -> Result<(), KindsError> {
        if self.relations.len() > u16::MAX as usize || self.groups.len() > u16::MAX as usize {
            return Err(KindsError::TooMany);
        }
        for names in [
            self.relations
                .iter()
                .map(|relation| relation.name.as_str())
                .collect::<Vec<_>>(),
            self.groups.iter().map(String::as_str).collect(),
        ] {
            let mut seen = BTreeSet::new();
            if let Some(repeat) = names.into_iter().find(|name| !seen.insert(*name)) {
                return Err(KindsError::DuplicateName(repeat.to_string()));
            }
        }
        for relation in &self.relations {
            let inside = |value: Fixed32| relation.minimum <= value && value <= relation.maximum;
            if relation.minimum > relation.maximum
                || !inside(relation.starting)
                || !inside(relation.rest)
            {
                return Err(KindsError::Bounds(relation.name.clone()));
            }
            if relation.decay_per_day < Fixed32::ZERO {
                return Err(KindsError::NegativeDecay(relation.name.clone()));
            }
            if let Some(outside) = relation
                .thresholds
                .iter()
                .find(|threshold| !inside(threshold.at))
            {
                return Err(KindsError::ThresholdOutside {
                    relation: relation.name.clone(),
                    threshold: outside.name.clone(),
                });
            }
        }
        Ok(())
    }
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct KindsFile {
        relations: Vec<RelationFile>,
        #[serde(default)]
        groups: Vec<String>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RelationFile {
        name: String,
        minimum: Decimal,
        maximum: Decimal,
        starting: Decimal,
        /// The starting value when absent.
        #[serde(default)]
        rest: Option<Decimal>,
        #[serde(default)]
        decay_per_day: Option<Decimal>,
        #[serde(default)]
        thresholds: Vec<ThresholdFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ThresholdFile {
        at: Decimal,
        name: String,
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

    impl KindsFile {
        pub(super) fn into_kinds(self) -> Kinds {
            let relations = self
                .relations
                .into_iter()
                .map(|file| {
                    let mut thresholds: Vec<Threshold> = file
                        .thresholds
                        .into_iter()
                        .map(|threshold| Threshold {
                            at: threshold.at.0,
                            name: threshold.name,
                        })
                        .collect();
                    thresholds.sort_by_key(|threshold| threshold.at);
                    Relation {
                        name: file.name,
                        minimum: file.minimum.0,
                        maximum: file.maximum.0,
                        starting: file.starting.0,
                        rest: file.rest.map_or(file.starting.0, |rest| rest.0),
                        decay_per_day: file.decay_per_day.map_or(Fixed32::ZERO, |decay| decay.0),
                        thresholds,
                    }
                })
                .collect();
            Kinds {
                relations,
                groups: self.groups,
            }
        }
    }
}
