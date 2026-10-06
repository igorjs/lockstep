// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod catalogue;
mod knowledge;

pub use catalogue::{
    Catalogue, CatalogueError, Fact, FactId, Predicate, Question, QuestionId, Rule, RuleId,
    RuleKind,
};
pub use knowledge::{
    evaluate, holds, receive, Fragment, History, Knowledge, KnowledgeEvent, NoHistory,
};
