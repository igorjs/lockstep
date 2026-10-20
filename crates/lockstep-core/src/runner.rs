// SPDX-License-Identifier: Apache-2.0
use crate::clock::{Clock, ClockConfiguration, ClockEvent};
use crate::hashing::{encode, hash_of};
use crate::message::Message;
use crate::save::{Migrations, Save, SaveError, SAVE_FORMAT};
use crate::simulation::{Context, Simulation};
use crate::streams::Streams;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepConfiguration {
    pub step_seconds: f32,
    pub maximum_steps_per_advance: u32,
}

impl Default for StepConfiguration {
    fn default() -> Self {
        Self {
            step_seconds: 1.0 / 30.0,
            maximum_steps_per_advance: 8,
        }
    }
}

/// Owns time: a fixed step, an accumulator, the simulated clock, and the named random streams.
pub struct Runner<S: Simulation> {
    simulation: S,
    clock: Clock,
    randomness: Streams,
    step_configuration: StepConfiguration,
    clock_configuration: ClockConfiguration,
    accumulator_seconds: f32,
    step_number: u64,
    /// Queued by the host, consumed by the next step that runs.
    pending: Vec<S::Intent>,
    clock_events: Vec<ClockEvent>,
}

pub struct Advanced<S: Simulation> {
    pub events: Vec<S::Event>,
    pub steps_run: u32,
    pub clock_events: Vec<ClockEvent>,
}

impl<S: Simulation> Runner<S> {
    pub fn new(
        configuration: S::Configuration,
        seed: u64,
        step: StepConfiguration,
        clock: ClockConfiguration,
    ) -> Self {
        let mut randomness = Streams::new(seed);
        let simulation = S::create(configuration, &mut randomness);
        Self {
            simulation,
            clock: Clock::new(clock, step.step_seconds),
            randomness,
            step_configuration: step,
            clock_configuration: clock,
            accumulator_seconds: 0.0,
            step_number: 0,
            pending: Vec::new(),
            clock_events: Vec::new(),
        }
    }

    /// The whole state at this step boundary, under the simulation's id. Unspent real time and
    /// queued intents are not part of it (see `Save`).
    pub fn save(&self, simulation_id: &str) -> Save {
        Save {
            format: SAVE_FORMAT,
            simulation_id: simulation_id.to_string(),
            snapshot_version: <S::Snapshot as Message>::VERSION,
            step_configuration: self.step_configuration,
            clock_configuration: self.clock_configuration,
            step_number: self.step_number,
            clock: self.clock.clone(),
            randomness: self.randomness.clone(),
            snapshot: encode(&self.simulation.snapshot()),
        }
    }

    /// Continues from a save, converting an older snapshot shape through `migrations`. The runner
    /// starts with no unspent real time and no queued intents.
    pub fn from_save(save: &Save, migrations: &Migrations) -> Result<Self, SaveError> {
        save.validate()?;
        let snapshot: S::Snapshot = migrations.migrate(save.snapshot_version, &save.snapshot)?;
        Ok(Self {
            simulation: S::restore(snapshot),
            clock: save.clock.clone(),
            randomness: save.randomness.clone(),
            step_configuration: save.step_configuration,
            clock_configuration: save.clock_configuration,
            accumulator_seconds: 0.0,
            step_number: save.step_number,
            pending: Vec::new(),
            clock_events: Vec::new(),
        })
    }

    /// Queue intents from the host. They apply to the next step that runs,
    /// however many frames pass first, so input arriving between steps is never lost.
    pub fn queue(&mut self, intents: impl IntoIterator<Item = S::Intent>) {
        self.pending.extend(intents);
    }

    /// Feed real time; runs zero or more fixed steps. Queued intents apply to the first step run.
    pub fn advance(&mut self, real_seconds: f32) -> Advanced<S> {
        self.advance_observed(real_seconds, &mut |_, _| {})
    }

    /// `advance`, calling `observer` after every step it runs with the runner and the intents that
    /// step used. The recorder uses it to write each step down.
    pub(crate) fn advance_observed(
        &mut self,
        real_seconds: f32,
        observer: &mut dyn FnMut(&Self, &[S::Intent]),
    ) -> Advanced<S> {
        let mut events = Vec::new();
        let mut steps_run = 0;
        self.accumulator_seconds += real_seconds.max(0.0);
        while self.accumulator_seconds >= self.step_configuration.step_seconds
            && steps_run < self.step_configuration.maximum_steps_per_advance
        {
            let intents = std::mem::take(&mut self.pending);
            self.run_one_step(&intents, &mut events);
            self.accumulator_seconds -= self.step_configuration.step_seconds;
            steps_run += 1;
            observer(self, &intents);
        }
        // Drop excess time after a hitch rather than spiral.
        if steps_run == self.step_configuration.maximum_steps_per_advance {
            self.accumulator_seconds = 0.0;
        }
        Advanced {
            events,
            steps_run,
            clock_events: std::mem::take(&mut self.clock_events),
        }
    }

    /// Exactly one step with the given intents, bypassing the accumulator and the queue.
    /// Used by replay and turn-based callers.
    pub fn step_once(&mut self, intents: &[S::Intent]) -> Advanced<S> {
        let mut events = Vec::new();
        self.run_one_step(intents, &mut events);
        Advanced {
            events,
            steps_run: 1,
            clock_events: std::mem::take(&mut self.clock_events),
        }
    }

    fn run_one_step(&mut self, intents: &[S::Intent], events: &mut Vec<S::Event>) {
        if let Some((sunrise, sunset)) = self.simulation.daylight(self.clock.day()) {
            self.clock.set_daylight(sunrise, sunset);
        }
        let (elapsed_simulated_minutes, elapsed_minutes) =
            self.clock.advance_exactly(&mut self.clock_events);
        let mut context = Context {
            clock: &self.clock,
            elapsed_simulated_minutes,
            elapsed_minutes,
            randomness: &mut self.randomness,
            events,
            step_number: self.step_number,
            step_seconds: self.step_configuration.step_seconds,
        };
        self.simulation.step(&mut context, intents);
        self.step_number += 1;
    }

    pub fn step_configuration(&self) -> StepConfiguration {
        self.step_configuration
    }

    pub fn set_clock_multiplier(&mut self, multiplier: f32) {
        self.clock.set_multiplier(multiplier);
    }

    /// Fraction of a step accumulated, for host interpolation between two snapshots.
    pub fn accumulator_fraction(&self) -> f32 {
        self.accumulator_seconds / self.step_configuration.step_seconds
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    pub fn simulation(&self) -> &S {
        &self.simulation
    }

    pub fn step_number(&self) -> u64 {
        self.step_number
    }

    pub fn snapshot(&self) -> S::Snapshot {
        self.simulation.snapshot()
    }

    pub fn hash(&self) -> u64 {
        hash_of(&(
            self.step_number,
            self.clock.position_units(),
            self.clock.day(),
            &self.randomness,
            &self.simulation.snapshot(),
        ))
    }
}
