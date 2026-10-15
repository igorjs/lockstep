// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One attribute a template sets: its bounds and what a point above the archetype's base costs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeRule {
    pub name: String,
    pub minimum: i32,
    pub maximum: i32,
    pub cost_per_point: u32,
}

/// A starting shape: a base value for every attribute.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Archetype {
    pub name: String,
    /// In the order of the attribute rules.
    pub base: Vec<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemplateError {
    Json(String),
    DuplicateName(String),
    /// A minimum above the maximum, or a cost of nothing.
    Rule(String),
    /// An archetype names an attribute with no rule, omits one, or has a base outside the bounds.
    Archetype(String),
    UnknownArchetype(String),
    UnknownAttribute(String),
    /// A purchase past an attribute's maximum.
    AboveMaximum(String),
    /// Purchases that cost more than the budget.
    OverBudget {
        cost: u64,
        budget: u32,
    },
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TemplateError {}

/// Archetypes and a point-buy budget. Build it with `Templates::from_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Templates {
    pub budget: u32,
    pub attributes: Vec<AttributeRule>,
    pub archetypes: Vec<Archetype>,
}

impl Templates {
    /// Reads the data form documented in `docs/lockstep-meta.md`.
    pub fn from_json(text: &str) -> Result<Self, TemplateError> {
        let file: file::TemplatesFile =
            serde_json::from_str(text).map_err(|error| TemplateError::Json(error.to_string()))?;
        let templates = file.into_templates()?;
        templates.check()?;
        Ok(templates)
    }

    pub fn archetype(&self, name: &str) -> Option<&Archetype> {
        self.archetypes
            .iter()
            .find(|archetype| archetype.name == name)
    }

    /// The attributes of a hire: the archetype's base, plus each purchase of whole points above
    /// it. The cost is the points bought times the attribute's cost a point, summed, and may not
    /// exceed the budget; no attribute may pass its maximum. A name bought twice adds up.
    pub fn build(
        &self,
        archetype: &str,
        purchases: &[(&str, u32)],
    ) -> Result<Vec<(String, i32)>, TemplateError> {
        let archetype = self
            .archetype(archetype)
            .ok_or_else(|| TemplateError::UnknownArchetype(archetype.to_string()))?;
        let mut values = archetype.base.clone();
        let mut cost: u64 = 0;
        for (name, points) in purchases {
            let index = self
                .attributes
                .iter()
                .position(|rule| rule.name == *name)
                .ok_or_else(|| TemplateError::UnknownAttribute(name.to_string()))?;
            let rule = &self.attributes[index];
            let raised = values[index] as i64 + *points as i64;
            if raised > rule.maximum as i64 {
                return Err(TemplateError::AboveMaximum(rule.name.clone()));
            }
            values[index] = raised as i32;
            cost += *points as u64 * rule.cost_per_point as u64;
        }
        if cost > self.budget as u64 {
            return Err(TemplateError::OverBudget {
                cost,
                budget: self.budget,
            });
        }
        Ok(self
            .attributes
            .iter()
            .zip(values)
            .map(|(rule, value)| (rule.name.clone(), value))
            .collect())
    }

    fn check(&self) -> Result<(), TemplateError> {
        let mut seen = BTreeSet::new();
        for rule in &self.attributes {
            if !seen.insert(rule.name.as_str()) {
                return Err(TemplateError::DuplicateName(rule.name.clone()));
            }
            if rule.minimum > rule.maximum || rule.cost_per_point == 0 {
                return Err(TemplateError::Rule(rule.name.clone()));
            }
        }
        let mut seen = BTreeSet::new();
        for archetype in &self.archetypes {
            if !seen.insert(archetype.name.as_str()) {
                return Err(TemplateError::DuplicateName(archetype.name.clone()));
            }
            let inside = archetype
                .base
                .iter()
                .zip(&self.attributes)
                .all(|(value, rule)| rule.minimum <= *value && *value <= rule.maximum);
            if archetype.base.len() != self.attributes.len() || !inside {
                return Err(TemplateError::Archetype(archetype.name.clone()));
            }
        }
        Ok(())
    }
}

mod file {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct TemplatesFile {
        budget: u32,
        attributes: Vec<AttributeFile>,
        archetypes: Vec<ArchetypeFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AttributeFile {
        name: String,
        minimum: i32,
        maximum: i32,
        cost_per_point: u32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ArchetypeFile {
        name: String,
        base: BTreeMap<String, i32>,
    }

    impl TemplatesFile {
        pub(super) fn into_templates(self) -> Result<Templates, TemplateError> {
            let attributes: Vec<AttributeRule> = self
                .attributes
                .into_iter()
                .map(|file| AttributeRule {
                    name: file.name,
                    minimum: file.minimum,
                    maximum: file.maximum,
                    cost_per_point: file.cost_per_point,
                })
                .collect();
            let mut archetypes = Vec::new();
            for file in self.archetypes {
                if let Some(unknown) = file
                    .base
                    .keys()
                    .find(|name| !attributes.iter().any(|rule| &rule.name == *name))
                {
                    return Err(TemplateError::Archetype(format!(
                        "{} names {unknown}",
                        file.name
                    )));
                }
                // An attribute left out leaves the base short, which the length check refuses.
                let base: Vec<i32> = attributes
                    .iter()
                    .filter_map(|rule| file.base.get(&rule.name).copied())
                    .collect();
                archetypes.push(Archetype {
                    name: file.name,
                    base,
                });
            }
            Ok(Templates {
                budget: self.budget,
                attributes,
                archetypes,
            })
        }
    }
}
