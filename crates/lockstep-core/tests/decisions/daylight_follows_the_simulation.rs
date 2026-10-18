// SPDX-License-Identifier: Apache-2.0
//! Decision 0060: a simulation whose daylight follows a calendar says so through
//! `Simulation::daylight`, and the runner sets it before every step. A replay therefore sets the
//! same daylight the first run did, with nothing recorded for it.
//! Alternatives rejected: the host setting daylight on the clock between steps (a recording does
//! not hold it, so a replay of such a session diverges), and a daylight field on every recorded
//! step (it grows every recording to repeat what the simulation already knows).
//! Would change if: a recording of a simulation that sets daylight replays to another hash, or a
//! simulation that does not set daylight gets its clock changed.

use lockstep_core::{
    record_session, replay, ClockConfiguration, Context, Message, ReplayOutcome, Runner, Session,
    Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Nothing;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Seen {
    sunrise_total: u64,
}

struct Dawn<const FOLLOWS: bool> {
    seen: Seen,
}

impl<const FOLLOWS: bool> Simulation for Dawn<FOLLOWS> {
    type Intent = Nothing;
    type Event = Nothing;
    type Snapshot = Seen;
    type Configuration = Nothing;

    fn create(_configuration: Nothing, _randomness: &mut Streams) -> Self {
        Dawn {
            seen: Seen { sunrise_total: 0 },
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, _intents: &[Nothing]) {
        self.seen.sunrise_total += context.clock.configuration().sunrise_minute as u64;
    }

    fn daylight(&self, day: u32) -> Option<(u32, u32)> {
        FOLLOWS.then_some((300 + day, 1_080))
    }

    fn snapshot(&self) -> Seen {
        self.seen.clone()
    }

    fn restore(snapshot: Seen) -> Self {
        Dawn { seen: snapshot }
    }
}

fn clock() -> ClockConfiguration {
    ClockConfiguration {
        // 180 steps a day at the default step.
        day_length_real_minutes: 0.1,
        sunrise_minute: 360,
        sunset_minute: 1_080,
        starting_minute: 0,
        starting_day: 0,
    }
}

fn runner<const FOLLOWS: bool>() -> Runner<Dawn<FOLLOWS>> {
    Runner::new(Nothing, 1, StepConfiguration::default(), clock())
}

fn drive(session: &mut impl Session<Dawn<true>>) {
    for _ in 0..500 {
        session.step_once(&[]);
    }
}

#[test]
fn the_runner_sets_the_daylight_of_each_day_before_its_steps() {
    let mut following = runner::<true>();
    let mut leaving = runner::<false>();
    for _ in 0..500 {
        following.step_once(&[]);
        leaving.step_once(&[]);
    }
    // Days 0, 1 and 2 hold 180, 180 and 140 of the 500 steps.
    let expected = 180 * 300 + 180 * 301 + 140 * 302;
    assert_eq!(following.simulation().snapshot().sunrise_total, expected);
    assert_eq!(leaving.simulation().snapshot().sunrise_total, 500 * 360);
}

#[test]
fn a_recording_of_such_a_simulation_replays_to_the_same_hash() {
    let recording = record_session("dawn", &Nothing, 1, runner::<true>(), 100, false, drive);
    let (replayed, outcome) = replay::<Dawn<true>>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    let mut direct = runner::<true>();
    drive(&mut direct);
    assert_eq!(replayed.hash(), direct.hash());
}
