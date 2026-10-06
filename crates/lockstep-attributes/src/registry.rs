// SPDX-License-Identifier: Apache-2.0
use crate::curve::Curve;
use lockstep_core::math::Fixed32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// An attribute's index in its registry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AttributeId(pub u16);

/// What happens to the current value when modifiers change the maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaximumPolicy {
    /// Hunger: the current value stays, clipped to the new maximum.
    Clamp,
    /// Health: the current value keeps its fraction, so 50 of 100 becomes 75 of 150.
    ScaleCurrent,
    /// Corruption: clipped like `Clamp`, and each threshold the current value rises through becomes
    /// the new minimum. The minimum never falls.
    Ratchet,
}

/// A named value the current value can cross, such as a status icon: Hunger at 60, 40, 20, 5.
#[derive(Clone, Debug, PartialEq)]
pub struct Threshold {
    pub at: Fixed32,
    pub name: String,
}

/// A value computed from other attributes, recomputed in registry order after every change.
#[derive(Clone, Debug, PartialEq)]
pub struct Derived {
    pub inputs: Vec<AttributeId>,
    pub curve: Curve,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub minimum: Fixed32,
    /// The maximum before modifiers.
    pub maximum: Fixed32,
    pub starting: Fixed32,
    pub thresholds: Vec<Threshold>,
    pub derived: Option<Derived>,
    pub on_maximum_change: MaximumPolicy,
}

/// Every attribute an entity can have, in a stable order. Build it with `Registry::new` or
/// `Registry::from_json`, which both check it.
#[derive(Clone, Debug, PartialEq)]
pub struct Registry {
    names: Vec<String>,
    definitions: Vec<Definition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    Json(String),
    TooMany,
    EmptyName,
    DuplicateName(String),
    /// The minimum is above the maximum, or the starting value is outside them.
    Bounds(String),
    /// A threshold is outside the minimum and maximum.
    ThresholdOutside {
        attribute: String,
        threshold: String,
    },
    UnknownInput {
        attribute: String,
        input: String,
    },
    /// A derived attribute reads itself.
    ReadsItself(String),
    /// Derived attributes read each other in a loop. Names the first one in registry order.
    Cycle(String),
    /// Linear, piecewise and threshold curves read exactly one input; the others one or more.
    InputCount(String),
    /// Knots and steps must be non-empty and strictly increasing in their input.
    Unsorted(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RegistryError {}

impl Registry {
    pub fn new(attributes: Vec<(String, Definition)>) -> Result<Self, RegistryError> {
        let (names, definitions) = attributes.into_iter().unzip();
        let registry = Registry { names, definitions };
        registry.check()?;
        Ok(registry)
    }

    /// Reads the data form documented in `docs/lockstep-attributes.md`.
    pub fn from_json(text: &str) -> Result<Self, RegistryError> {
        let file: file::RegistryFile =
            serde_json::from_str(text).map_err(|error| RegistryError::Json(error.to_string()))?;
        file.into_registry()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn ids(&self) -> impl Iterator<Item = AttributeId> {
        (0..self.definitions.len() as u16).map(AttributeId)
    }

    pub fn id(&self, name: &str) -> Option<AttributeId> {
        self.names
            .iter()
            .position(|known| known == name)
            .map(|index| AttributeId(index as u16))
    }

    pub fn name(&self, id: AttributeId) -> &str {
        &self.names[id.0 as usize]
    }

    pub fn definition(&self, id: AttributeId) -> &Definition {
        &self.definitions[id.0 as usize]
    }

    fn check(&self) -> Result<(), RegistryError> {
        if self.definitions.len() > u16::MAX as usize {
            return Err(RegistryError::TooMany);
        }
        let mut seen = BTreeSet::new();
        for (index, (name, definition)) in self.names.iter().zip(&self.definitions).enumerate() {
            if name.is_empty() {
                return Err(RegistryError::EmptyName);
            }
            if !seen.insert(name.as_str()) {
                return Err(RegistryError::DuplicateName(name.clone()));
            }
            let Definition {
                minimum,
                maximum,
                starting,
                ..
            } = *definition;
            if minimum > maximum || starting < minimum || starting > maximum {
                return Err(RegistryError::Bounds(name.clone()));
            }
            for threshold in &definition.thresholds {
                if threshold.at < minimum || threshold.at > maximum {
                    return Err(RegistryError::ThresholdOutside {
                        attribute: name.clone(),
                        threshold: threshold.name.clone(),
                    });
                }
            }
            let Some(derived) = &definition.derived else {
                continue;
            };
            for input in &derived.inputs {
                if input.0 as usize >= self.definitions.len() {
                    return Err(RegistryError::UnknownInput {
                        attribute: name.clone(),
                        input: format!("#{}", input.0),
                    });
                }
                if input.0 as usize == index {
                    return Err(RegistryError::ReadsItself(name.clone()));
                }
            }
            let count = derived.inputs.len();
            let one = derived.curve.takes_one_input();
            if (one && count != 1) || (!one && count == 0) {
                return Err(RegistryError::InputCount(name.clone()));
            }
            let points = match &derived.curve {
                Curve::Piecewise { knots } => Some(knots),
                Curve::Threshold { steps } => Some(steps),
                _ => None,
            };
            if let Some(points) = points {
                if points.is_empty() || points.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    return Err(RegistryError::Unsorted(name.clone()));
                }
            }
        }
        self.check_cycles()
    }

    /// A derived attribute may read one declared after it (it lags one change), but no chain of
    /// derived inputs may lead back to where it started.
    fn check_cycles(&self) -> Result<(), RegistryError> {
        // 0: not visited, 1: on the current path, 2: finished.
        let mut state = vec![0u8; self.definitions.len()];
        for start in 0..self.definitions.len() {
            if state[start] != 0 {
                continue;
            }
            // Depth first without recursion: (attribute, next input to look at).
            let mut stack = vec![(start, 0usize)];
            state[start] = 1;
            while let Some((index, next)) = stack.last_mut() {
                let inputs = self.definitions[*index]
                    .derived
                    .as_ref()
                    .map_or(&[][..], |derived| &derived.inputs[..]);
                let Some(input) = inputs.get(*next) else {
                    state[*index] = 2;
                    stack.pop();
                    continue;
                };
                *next += 1;
                let input = input.0 as usize;
                match state[input] {
                    0 => {
                        state[input] = 1;
                        stack.push((input, 0));
                    }
                    1 => return Err(RegistryError::Cycle(self.names[start].clone())),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// The JSON form. Numbers are decimal strings (`"1.5"`) or whole numbers (`100`), never JSON
/// fractions, so a value reads the same on every platform.
mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct RegistryFile {
        attributes: Vec<AttributeFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AttributeFile {
        name: String,
        minimum: Decimal,
        maximum: Decimal,
        starting: Decimal,
        #[serde(default)]
        thresholds: Vec<ThresholdFile>,
        #[serde(default)]
        derived: Option<DerivedFile>,
        #[serde(default = "clamp")]
        on_maximum_change: MaximumPolicy,
    }

    fn clamp() -> MaximumPolicy {
        MaximumPolicy::Clamp
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ThresholdFile {
        at: Decimal,
        name: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DerivedFile {
        inputs: Vec<String>,
        curve: CurveFile,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum CurveFile {
        Linear { per_point: Decimal, offset: Decimal },
        Piecewise { knots: Vec<(Decimal, Decimal)> },
        Threshold { steps: Vec<(Decimal, Decimal)> },
        Product,
        Sum,
        Difference,
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

    impl RegistryFile {
        pub(super) fn into_registry(self) -> Result<Registry, RegistryError> {
            let names: Vec<String> = self
                .attributes
                .iter()
                .map(|entry| entry.name.clone())
                .collect();
            let lookup = |attribute: &str, input: &str| {
                names
                    .iter()
                    .position(|name| name == input)
                    .map(|index| AttributeId(index as u16))
                    .ok_or_else(|| RegistryError::UnknownInput {
                        attribute: attribute.to_string(),
                        input: input.to_string(),
                    })
            };
            let pairs = |points: Vec<(Decimal, Decimal)>| {
                points
                    .into_iter()
                    .map(|(x, y)| (x.0, y.0))
                    .collect::<Vec<_>>()
            };
            let mut attributes = Vec::with_capacity(self.attributes.len());
            for entry in self.attributes {
                let derived = match entry.derived {
                    None => None,
                    Some(derived) => Some(Derived {
                        inputs: derived
                            .inputs
                            .iter()
                            .map(|input| lookup(&entry.name, input))
                            .collect::<Result<_, _>>()?,
                        curve: match derived.curve {
                            CurveFile::Linear { per_point, offset } => Curve::Linear {
                                per_point: per_point.0,
                                offset: offset.0,
                            },
                            CurveFile::Piecewise { knots } => Curve::Piecewise {
                                knots: pairs(knots),
                            },
                            CurveFile::Threshold { steps } => Curve::Threshold {
                                steps: pairs(steps),
                            },
                            CurveFile::Product => Curve::Product,
                            CurveFile::Sum => Curve::Sum,
                            CurveFile::Difference => Curve::Difference,
                        },
                    }),
                };
                let definition = Definition {
                    minimum: entry.minimum.0,
                    maximum: entry.maximum.0,
                    starting: entry.starting.0,
                    thresholds: entry
                        .thresholds
                        .into_iter()
                        .map(|threshold| Threshold {
                            at: threshold.at.0,
                            name: threshold.name,
                        })
                        .collect(),
                    derived,
                    on_maximum_change: entry.on_maximum_change,
                };
                attributes.push((entry.name, definition));
            }
            Registry::new(attributes)
        }
    }
}
