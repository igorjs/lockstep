// SPDX-License-Identifier: Apache-2.0
//! Record a session's inputs, replay them to the same hash, and find where a replay diverges.

use crate::clock::ClockConfiguration;
use crate::hashing::{decode, encode};
use crate::message::Message;
use crate::runner::{Advanced, Runner, StepConfiguration};
use crate::simulation::Simulation;
use serde::{Deserialize, Serialize};

/// One step's inputs: the intents it ran with and the clock multiplier the host had set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordedStep<I> {
    pub intents: Vec<I>,
    /// Exactly the value the host passed to `set_clock_multiplier`, so a replay sets the same
    /// fixed point rate.
    pub clock_multiplier: f32,
}

/// Everything needed to run a session again: the starting point and every step's inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Recording<I: Message> {
    pub simulation_id: String,
    /// The configuration, in the fixed-endian bytes `hash_of` uses.
    pub configuration: Vec<u8>,
    pub seed: u64,
    pub step_configuration: StepConfiguration,
    pub clock_configuration: ClockConfiguration,
    pub steps: Vec<RecordedStep<I>>,
    /// (step number, runner hash), at step 0 and every `checkpoint_every` steps after.
    pub checkpoints: Vec<(u64, u64)>,
    /// (step number, encoded snapshot) at each checkpoint, when the recorder keeps them; `bisect`
    /// uses them to show what differs.
    pub snapshots: Vec<(u64, Vec<u8>)>,
}

impl<I: Message> Recording<I> {
    pub fn to_bytes(&self) -> Vec<u8> {
        encode(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReplayError> {
        decode(bytes).map_err(ReplayError::NotARecording)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    NotARecording(String),
    /// The configuration bytes do not decode as this simulation's configuration.
    Configuration(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::NotARecording(reason) => write!(formatter, "not a recording: {reason}"),
            ReplayError::Configuration(reason) => {
                write!(formatter, "the configuration does not decode: {reason}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

/// Wraps a runner and writes down every step's inputs. A host uses it exactly as it would use the
/// runner.
pub struct Recorder<S: Simulation> {
    runner: Runner<S>,
    recording: Recording<S::Intent>,
    checkpoint_every: u64,
    keep_snapshots: bool,
    clock_multiplier: f32,
}

impl<S: Simulation> Recorder<S> {
    /// Starts a session and records it. `checkpoint_every` is at least 1.
    pub fn new(
        simulation_id: &str,
        configuration: S::Configuration,
        seed: u64,
        step_configuration: StepConfiguration,
        clock_configuration: ClockConfiguration,
        checkpoint_every: u64,
    ) -> Self {
        let encoded = encode(&configuration);
        let runner = Runner::new(configuration, seed, step_configuration, clock_configuration);
        let recording = Recording {
            simulation_id: simulation_id.to_string(),
            configuration: encoded,
            seed,
            step_configuration,
            clock_configuration,
            steps: Vec::new(),
            checkpoints: Vec::new(),
            snapshots: Vec::new(),
        };
        let mut recorder = Recorder {
            runner,
            recording,
            checkpoint_every: checkpoint_every.max(1),
            keep_snapshots: false,
            clock_multiplier: 1.0,
        };
        recorder.checkpoint();
        recorder
    }

    /// Also keeps the snapshot at every checkpoint, so `bisect` can show what differs. Costs a
    /// snapshot per checkpoint in the recording. Call it before the first step.
    pub fn keep_snapshots(mut self) -> Self {
        self.keep_snapshots = true;
        self.recording.snapshots.clear();
        self.checkpoint_snapshot();
        self
    }

    pub fn queue(&mut self, intents: impl IntoIterator<Item = S::Intent>) {
        self.runner.queue(intents);
    }

    pub fn set_clock_multiplier(&mut self, multiplier: f32) {
        self.clock_multiplier = multiplier;
        self.runner.set_clock_multiplier(multiplier);
    }

    pub fn advance(&mut self, real_seconds: f32) -> Advanced<S> {
        let (every, keep, multiplier) = (
            self.checkpoint_every,
            self.keep_snapshots,
            self.clock_multiplier,
        );
        let recording = &mut self.recording;
        // One call can run several steps; the observer sees the runner after each, which is
        // where a checkpoint falling between them is taken.
        self.runner
            .advance_observed(real_seconds, &mut |runner: &Runner<S>, intents| {
                recording.steps.push(RecordedStep {
                    intents: intents.to_vec(),
                    clock_multiplier: multiplier,
                });
                let step = runner.step_number();
                if step.is_multiple_of(every) {
                    recording.checkpoints.push((step, runner.hash()));
                    if keep {
                        recording.snapshots.push((step, encode(&runner.snapshot())));
                    }
                }
            })
    }

    pub fn step_once(&mut self, intents: &[S::Intent]) -> Advanced<S> {
        let advanced = self.runner.step_once(intents);
        self.recording.steps.push(RecordedStep {
            intents: intents.to_vec(),
            clock_multiplier: self.clock_multiplier,
        });
        if self
            .runner
            .step_number()
            .is_multiple_of(self.checkpoint_every)
        {
            self.checkpoint();
        }
        advanced
    }

    pub fn runner(&self) -> &Runner<S> {
        &self.runner
    }

    pub fn recording(&self) -> &Recording<S::Intent> {
        &self.recording
    }

    pub fn into_recording(self) -> Recording<S::Intent> {
        self.recording
    }

    fn checkpoint(&mut self) {
        let step = self.runner.step_number();
        if self.recording.checkpoints.last().map(|(at, _)| *at) == Some(step) {
            return;
        }
        self.recording.checkpoints.push((step, self.runner.hash()));
        self.checkpoint_snapshot();
    }

    fn checkpoint_snapshot(&mut self) {
        if self.keep_snapshots {
            let step = self.runner.step_number();
            self.recording
                .snapshots
                .push((step, encode(&self.runner.snapshot())));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayOutcome {
    Identical,
    /// The first checkpoint whose hash differs. The world was still the same at the checkpoint
    /// before it.
    Diverged {
        at_step: u64,
        expected: u64,
        actual: u64,
    },
}

/// Runs a recording again from the start. Stops at the first checkpoint that differs and returns
/// the runner there; otherwise runs every step.
pub fn replay<S: Simulation>(
    recording: &Recording<S::Intent>,
) -> Result<(Runner<S>, ReplayOutcome), ReplayError> {
    let configuration: S::Configuration =
        decode(&recording.configuration).map_err(ReplayError::Configuration)?;
    let mut runner = Runner::new(
        configuration,
        recording.seed,
        recording.step_configuration,
        recording.clock_configuration,
    );
    let mut checkpoints = recording.checkpoints.iter().peekable();
    let mut check = |runner: &Runner<S>| -> Option<ReplayOutcome> {
        while let Some((at_step, expected)) = checkpoints.peek().copied() {
            if *at_step > runner.step_number() {
                break;
            }
            checkpoints.next();
            if *at_step == runner.step_number() {
                let actual = runner.hash();
                if actual != *expected {
                    return Some(ReplayOutcome::Diverged {
                        at_step: *at_step,
                        expected: *expected,
                        actual,
                    });
                }
            }
        }
        None
    };
    if let Some(outcome) = check(&runner) {
        return Ok((runner, outcome));
    }
    for step in &recording.steps {
        runner.set_clock_multiplier(step.clock_multiplier);
        runner.step_once(&step.intents);
        if let Some(outcome) = check(&runner) {
            return Ok((runner, outcome));
        }
    }
    Ok((runner, ReplayOutcome::Identical))
}

/// Where a replay stopped matching, and what differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bisection {
    /// The last checkpoint whose hash still matched.
    pub last_good_step: u64,
    /// The first checkpoint whose hash differs: the change is in the steps after `last_good_step`
    /// up to and including this one.
    pub first_bad_step: u64,
    /// Lines of the recorded and replayed snapshots that differ at `first_bad_step`, as
    /// `- recorded` and `+ replayed`, when the recording kept snapshots.
    pub differences: Vec<String>,
}

/// Replays a recording and names the checkpoints around the first divergence. `None` when the
/// replay matches. Record with `checkpoint_every` 1 to narrow it to the exact step.
pub fn bisect<S: Simulation>(
    recording: &Recording<S::Intent>,
) -> Result<Option<Bisection>, ReplayError> {
    let (runner, outcome) = replay::<S>(recording)?;
    let ReplayOutcome::Diverged { at_step, .. } = outcome else {
        return Ok(None);
    };
    let last_good_step = recording
        .checkpoints
        .iter()
        .map(|(step, _)| *step)
        .rfind(|step| *step < at_step)
        .unwrap_or(0);
    let differences = recording
        .snapshots
        .iter()
        .find(|(step, _)| *step == at_step)
        .and_then(|(_, bytes)| decode::<S::Snapshot>(bytes).ok())
        .map(|recorded| line_differences(&recorded, &runner.snapshot()))
        .unwrap_or_default();
    Ok(Some(Bisection {
        last_good_step,
        first_bad_step: at_step,
        differences,
    }))
}

/// The lines of two values' pretty debug output that differ, paired by position: a structural
/// diff that is enough when two snapshots have the same shape and different values.
fn line_differences<T: std::fmt::Debug>(recorded: &T, replayed: &T) -> Vec<String> {
    let recorded = format!("{recorded:#?}");
    let replayed = format!("{replayed:#?}");
    let (recorded, replayed): (Vec<&str>, Vec<&str>) =
        (recorded.lines().collect(), replayed.lines().collect());
    let mut differences = Vec::new();
    for index in 0..recorded.len().max(replayed.len()) {
        let (left, right) = (recorded.get(index), replayed.get(index));
        if left != right {
            if let Some(line) = left {
                differences.push(format!("- {}", line.trim()));
            }
            if let Some(line) = right {
                differences.push(format!("+ {}", line.trim()));
            }
        }
    }
    differences
}
