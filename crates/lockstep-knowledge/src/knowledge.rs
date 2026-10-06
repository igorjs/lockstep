// SPDX-License-Identifier: Apache-2.0
use crate::catalogue::{Catalogue, FactId, Predicate, QuestionId, RuleId, RuleKind};
use lockstep_core::{Column, Handle, Indexable, Message, Timeline};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One piece of evidence for a fact, from a source: a document, a witness, an instrument. The
/// same source twice counts once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fragment {
    pub fact: FactId,
    pub source: u32,
}

/// What one knower has gathered, and what has already fired for it. Everything here is saved,
/// so nothing fires twice after a load.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knowledge {
    fragments: BTreeSet<Fragment>,
    known: BTreeSet<FactId>,
    opened: BTreeSet<QuestionId>,
    fired: BTreeSet<RuleId>,
}

impl Knowledge {
    pub fn knows(&self, fact: FactId) -> bool {
        self.known.contains(&fact)
    }

    pub fn has_opened(&self, question: QuestionId) -> bool {
        self.opened.contains(&question)
    }

    pub fn has_fired(&self, rule: RuleId) -> bool {
        self.fired.contains(&rule)
    }

    /// Fragments of a fact from distinct sources.
    pub fn fragments_of(&self, fact: FactId) -> u16 {
        let from = Fragment { fact, source: 0 };
        self.fragments
            .range(from..)
            .take_while(|fragment| fragment.fact == fact)
            .count()
            .min(u16::MAX as usize) as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum KnowledgeEvent {
    /// A fact reached its fragments and is known.
    Learned { knower: Handle, fact: FactId },
    /// A question reached its fragments.
    Opened {
        knower: Handle,
        question: QuestionId,
    },
    /// A rule's predicate held for the first time.
    Fired {
        knower: Handle,
        rule: RuleId,
        kind: RuleKind,
    },
}

/// What happened before, for `Predicate::Happened`: how many events of a kind mention a knower.
pub trait History {
    fn happened(&self, kind: u16, who: Handle) -> u32;
}

/// A history with nothing in it.
pub struct NoHistory;

impl History for NoHistory {
    fn happened(&self, _: u16, _: Handle) -> u32 {
        0
    }
}

impl<E: Message + Indexable> History for Timeline<E> {
    fn happened(&self, kind: u16, who: Handle) -> u32 {
        self.for_entity(who)
            .filter(|entry| entry.event.kind() == kind)
            .count()
            .min(u32::MAX as usize) as u32
    }
}

/// Gives a knower a fragment. A new fragment can make its fact known and open questions, each
/// once; a fragment from a source already counted for that fact changes nothing. A knower with
/// no knowledge yet starts with an empty one. A fact the catalogue lacks is ignored.
pub fn receive(
    knowledge: &mut Column<Knowledge>,
    knower: Handle,
    fragment: Fragment,
    catalogue: &Catalogue,
    events: &mut Vec<KnowledgeEvent>,
) {
    let Some(fact) = catalogue.facts.get(fragment.fact.0 as usize) else {
        return;
    };
    if !knowledge.has(knower) {
        knowledge.set(knower, Knowledge::default());
    }
    let mind = knowledge.get_mut(knower).expect("just set");
    if !mind.fragments.insert(fragment) {
        return;
    }
    if !mind.known.contains(&fragment.fact) && mind.fragments_of(fragment.fact) >= fact.fragments {
        mind.known.insert(fragment.fact);
        events.push(KnowledgeEvent::Learned {
            knower,
            fact: fragment.fact,
        });
    }
    for (index, question) in catalogue.questions.iter().enumerate() {
        let id = QuestionId(index as u16);
        if mind.opened.contains(&id) || !question.facts.contains(&fragment.fact) {
            continue;
        }
        let gathered: u32 = question
            .facts
            .iter()
            .map(|fact| mind.fragments_of(*fact) as u32)
            .sum();
        if gathered >= question.fragments as u32 {
            mind.opened.insert(id);
            events.push(KnowledgeEvent::Opened {
                knower,
                question: id,
            });
        }
    }
}

/// Whether a predicate holds for a knower now.
pub fn holds(
    predicate: &Predicate,
    knower: Handle,
    mind: &Knowledge,
    history: &dyn History,
) -> bool {
    match predicate {
        Predicate::Known(fact) => mind.knows(*fact),
        Predicate::Fragments { fact, at_least } => mind.fragments_of(*fact) >= *at_least,
        Predicate::Opened(question) => mind.has_opened(*question),
        Predicate::Fired(rule) => mind.has_fired(*rule),
        Predicate::Happened { kind, at_least } => history.happened(*kind, knower) >= *at_least,
        Predicate::All(parts) => parts.iter().all(|part| holds(part, knower, mind, history)),
        Predicate::Any(parts) => parts.iter().any(|part| holds(part, knower, mind, history)),
        Predicate::Not(part) => !holds(part, knower, mind, history),
    }
}

/// Fires every rule whose predicate holds for each knower, each at most once per knower, in
/// handle order and then rule order. A rule that names an earlier rule sees it fired in the same
/// pass.
pub fn evaluate(
    knowledge: &mut Column<Knowledge>,
    catalogue: &Catalogue,
    history: &dyn History,
    events: &mut Vec<KnowledgeEvent>,
) {
    for (knower, mind) in knowledge.iter_mut() {
        for (index, rule) in catalogue.rules.iter().enumerate() {
            let id = RuleId(index as u16);
            if mind.fired.contains(&id) || !holds(&rule.when, knower, mind, history) {
                continue;
            }
            mind.fired.insert(id);
            events.push(KnowledgeEvent::Fired {
                knower,
                rule: id,
                kind: rule.kind,
            });
        }
    }
}
