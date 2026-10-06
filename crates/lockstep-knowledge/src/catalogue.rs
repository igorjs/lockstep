// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct FactId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct QuestionId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct RuleId(pub u16);

/// Something a knower can come to know, once enough fragments of it arrive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub name: String,
    /// Fragments from distinct sources it takes to know the fact.
    pub fragments: u16,
}

/// A question that opens once enough fragments of its facts have arrived, whether or not any of
/// them is known yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub name: String,
    pub facts: Vec<FactId>,
    pub fragments: u16,
}

/// What kind of thing a rule reveals; the simulation decides what each means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Revelation,
    Secret,
    Achievement,
}

/// A condition as data, so it can be saved, compared and hashed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    Known(FactId),
    /// At least this many fragments of a fact, known or not.
    Fragments {
        fact: FactId,
        at_least: u16,
    },
    Opened(QuestionId),
    Fired(RuleId),
    /// At least this many events of a kind in the history mentioning the knower.
    Happened {
        kind: u16,
        at_least: u32,
    },
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
    Not(Box<Predicate>),
}

/// Fires once per knower, the first time its predicate holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub kind: RuleKind,
    pub when: Predicate,
}

/// Every fact, question and rule, in a stable order. Build it with `Catalogue::from_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    pub facts: Vec<Fact>,
    pub questions: Vec<Question>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogueError {
    Json(String),
    TooMany,
    DuplicateName(String),
    UnknownFact {
        owner: String,
        fact: String,
    },
    UnknownQuestion {
        owner: String,
        question: String,
    },
    UnknownRule {
        owner: String,
        rule: String,
    },
    UnknownEvent {
        owner: String,
        event: String,
    },
    /// A fact or question that needs no fragments would be known from the start.
    NoFragments(String),
    /// Rules may only name rules listed before them, so firing has one order.
    RuleOrder {
        owner: String,
        rule: String,
    },
}

impl std::fmt::Display for CatalogueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CatalogueError {}

impl Catalogue {
    /// Reads the data form documented in `docs/lockstep-knowledge.md`. `event_kinds` names the
    /// simulation's event kinds for `happened`.
    pub fn from_json(text: &str, event_kinds: &[(&str, u16)]) -> Result<Self, CatalogueError> {
        let file: file::CatalogueFile =
            serde_json::from_str(text).map_err(|error| CatalogueError::Json(error.to_string()))?;
        file.into_catalogue(event_kinds)
    }

    pub fn fact_id(&self, name: &str) -> Option<FactId> {
        position(self.facts.iter().map(|fact| fact.name.as_str()), name).map(FactId)
    }

    pub fn question_id(&self, name: &str) -> Option<QuestionId> {
        position(
            self.questions.iter().map(|question| question.name.as_str()),
            name,
        )
        .map(QuestionId)
    }

    pub fn rule_id(&self, name: &str) -> Option<RuleId> {
        position(self.rules.iter().map(|rule| rule.name.as_str()), name).map(RuleId)
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
        facts: Vec<Fact>,
        #[serde(default)]
        questions: Vec<QuestionFile>,
        #[serde(default)]
        rules: Vec<RuleFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct QuestionFile {
        name: String,
        facts: Vec<String>,
        fragments: u16,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RuleFile {
        name: String,
        kind: RuleKind,
        when: PredicateFile,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum PredicateFile {
        Known(String),
        Fragments { fact: String, at_least: u16 },
        Opened(String),
        Fired(String),
        Happened { event: String, at_least: u32 },
        All(Vec<PredicateFile>),
        Any(Vec<PredicateFile>),
        Not(Box<PredicateFile>),
    }

    struct Names<'a> {
        facts: &'a [Fact],
        questions: &'a [Question],
        /// Only the rules before the one being read.
        rules: &'a [String],
        /// Every rule, to tell a later one from an unknown one.
        all_rules: &'a [String],
        events: &'a [(&'a str, u16)],
        owner: &'a str,
    }

    impl PredicateFile {
        fn resolve(self, names: &Names<'_>) -> Result<Predicate, CatalogueError> {
            let owner = || names.owner.to_string();
            let fact = |name: String| {
                position(names.facts.iter().map(|fact| fact.name.as_str()), &name)
                    .map(FactId)
                    .ok_or(CatalogueError::UnknownFact {
                        owner: owner(),
                        fact: name,
                    })
            };
            Ok(match self {
                PredicateFile::Known(name) => Predicate::Known(fact(name)?),
                PredicateFile::Fragments {
                    fact: name,
                    at_least,
                } => Predicate::Fragments {
                    fact: fact(name)?,
                    at_least,
                },
                PredicateFile::Opened(name) => Predicate::Opened(
                    position(
                        names
                            .questions
                            .iter()
                            .map(|question| question.name.as_str()),
                        &name,
                    )
                    .map(QuestionId)
                    .ok_or(CatalogueError::UnknownQuestion {
                        owner: owner(),
                        question: name,
                    })?,
                ),
                PredicateFile::Fired(name) => {
                    match position(names.rules.iter().map(String::as_str), &name) {
                        Some(index) => Predicate::Fired(RuleId(index)),
                        None if names.all_rules.contains(&name) => {
                            return Err(CatalogueError::RuleOrder {
                                owner: owner(),
                                rule: name,
                            })
                        }
                        None => {
                            return Err(CatalogueError::UnknownRule {
                                owner: owner(),
                                rule: name,
                            })
                        }
                    }
                }
                PredicateFile::Happened { event, at_least } => Predicate::Happened {
                    kind: names
                        .events
                        .iter()
                        .find(|(known, _)| *known == event)
                        .map(|(_, kind)| *kind)
                        .ok_or(CatalogueError::UnknownEvent {
                            owner: owner(),
                            event,
                        })?,
                    at_least,
                },
                PredicateFile::All(all) => Predicate::All(
                    all.into_iter()
                        .map(|part| part.resolve(names))
                        .collect::<Result<_, _>>()?,
                ),
                PredicateFile::Any(any) => Predicate::Any(
                    any.into_iter()
                        .map(|part| part.resolve(names))
                        .collect::<Result<_, _>>()?,
                ),
                PredicateFile::Not(part) => Predicate::Not(Box::new(part.resolve(names)?)),
            })
        }
    }

    fn unique<'a>(names: impl Iterator<Item = &'a str>) -> Result<(), CatalogueError> {
        let mut seen = BTreeSet::new();
        for name in names {
            if !seen.insert(name) {
                return Err(CatalogueError::DuplicateName(name.to_string()));
            }
        }
        Ok(())
    }

    impl CatalogueFile {
        pub(super) fn into_catalogue(
            self,
            events: &[(&str, u16)],
        ) -> Result<Catalogue, CatalogueError> {
            if [self.facts.len(), self.questions.len(), self.rules.len()]
                .iter()
                .any(|count| *count > u16::MAX as usize)
            {
                return Err(CatalogueError::TooMany);
            }
            unique(self.facts.iter().map(|fact| fact.name.as_str()))?;
            unique(self.questions.iter().map(|question| question.name.as_str()))?;
            unique(self.rules.iter().map(|rule| rule.name.as_str()))?;
            if let Some(fact) = self.facts.iter().find(|fact| fact.fragments == 0) {
                return Err(CatalogueError::NoFragments(fact.name.clone()));
            }
            let facts = self.facts;
            let questions = self
                .questions
                .into_iter()
                .map(|question| {
                    if question.fragments == 0 {
                        return Err(CatalogueError::NoFragments(question.name));
                    }
                    let ids = question
                        .facts
                        .into_iter()
                        .map(|name| {
                            position(facts.iter().map(|fact| fact.name.as_str()), &name)
                                .map(FactId)
                                .ok_or(CatalogueError::UnknownFact {
                                    owner: question.name.clone(),
                                    fact: name,
                                })
                        })
                        .collect::<Result<_, _>>()?;
                    Ok(Question {
                        name: question.name,
                        facts: ids,
                        fragments: question.fragments,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let all_rules: Vec<String> = self.rules.iter().map(|rule| rule.name.clone()).collect();
            let mut rule_names: Vec<String> = Vec::new();
            let mut rules = Vec::new();
            for rule in self.rules {
                let when = rule.when.resolve(&Names {
                    facts: &facts,
                    questions: &questions,
                    rules: &rule_names,
                    all_rules: &all_rules,
                    events,
                    owner: &rule.name,
                })?;
                rule_names.push(rule.name.clone());
                rules.push(Rule {
                    name: rule.name,
                    kind: rule.kind,
                    when,
                });
            }
            Ok(Catalogue {
                facts,
                questions,
                rules,
            })
        }
    }
}
