// SPDX-License-Identifier: Apache-2.0
use lockstep_core::Handle;
use serde::{Deserialize, Serialize};

/// One reason to prefer something, scored as a plain integer a designer can read. Any
/// `Fn(Handle, &W) -> i32` is one, so a closure over the world works.
pub trait Consideration<W: ?Sized> {
    fn score(&self, agent: Handle, world: &W) -> i32;
}

impl<W: ?Sized, F: Fn(Handle, &W) -> i32> Consideration<W> for F {
    fn score(&self, agent: Handle, world: &W) -> i32 {
        self(agent, world)
    }
}

/// Something an agent can do, scored by the sum of its considerations.
pub struct Choice<W: ?Sized> {
    pub id: u16,
    pub considerations: Vec<Box<dyn Consideration<W>>>,
}

impl<W: ?Sized> Choice<W> {
    /// Each consideration's score, in order, for showing why.
    pub fn scores(&self, agent: Handle, world: &W) -> Vec<i32> {
        self.considerations
            .iter()
            .map(|consideration| consideration.score(agent, world))
            .collect()
    }

    /// The sum of the scores, in 64 bits, so it never wraps.
    pub fn total(&self, agent: Handle, world: &W) -> i64 {
        self.scores(agent, world).into_iter().map(i64::from).sum()
    }
}

/// The choice with the highest total; a tie goes to the lowest id. `None` with no choices.
pub fn choose<W: ?Sized>(agent: Handle, choices: &[Choice<W>], world: &W) -> Option<u16> {
    choices
        .iter()
        .map(|choice| (choice.total(agent, world), choice.id))
        .max_by(|(a_total, a_id), (b_total, b_id)| a_total.cmp(b_total).then(b_id.cmp(a_id)))
        .map(|(_, id)| id)
}

/// Why a task was refused: the designer's number for a consideration, which a host shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Reason(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskResponse {
    Accept,
    Delay { minutes: u16 },
    Refuse { reason: Reason },
}

/// A task one agent asks of another: considerations with the reason each stands for, and the
/// totals that accept it or delay it.
pub struct Task<W: ?Sized> {
    pub id: u16,
    pub considerations: Vec<(Reason, Box<dyn Consideration<W>>)>,
    /// Accepted at this total or above.
    pub accept_at: i64,
    /// Delayed at this total or above, below `accept_at`; refused below it.
    pub delay_at: i64,
    pub delay_minutes: u16,
}

/// Accepts, delays or refuses a task by its total score. A refusal names the consideration that
/// scored lowest, the first listed on a tie, so the host can show why; with no considerations
/// the reason is `Reason(0)`.
pub fn evaluate_task<W: ?Sized>(agent: Handle, task: &Task<W>, world: &W) -> TaskResponse {
    let scores: Vec<(Reason, i32)> = task
        .considerations
        .iter()
        .map(|(reason, consideration)| (*reason, consideration.score(agent, world)))
        .collect();
    let total: i64 = scores.iter().map(|(_, score)| i64::from(*score)).sum();
    if total >= task.accept_at {
        TaskResponse::Accept
    } else if total >= task.delay_at {
        TaskResponse::Delay {
            minutes: task.delay_minutes,
        }
    } else {
        let reason = scores
            .iter()
            .enumerate()
            .min_by_key(|(index, (_, score))| (*score, *index))
            .map_or(Reason(0), |(_, (reason, _))| *reason);
        TaskResponse::Refuse { reason }
    }
}
