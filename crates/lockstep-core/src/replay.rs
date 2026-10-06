// SPDX-License-Identifier: Apache-2.0
//! Record a session's inputs, replay them to the same hash, and find where a replay diverges.

use crate::clock::ClockConfiguration;
use crate::hashing::{decode, encode, DECODE_LIMIT};
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
    /// The recording's bytes. Refuses a recording too large for `from_bytes` to read back.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ReplayError> {
        let bytes = encode(self);
        if bytes.len() as u64 > DECODE_LIMIT {
            return Err(ReplayError::TooLarge(bytes.len() as u64));
        }
        Ok(bytes)
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
    /// A checkpoint is out of order or past the last step, so it could never be compared.
    Checkpoints {
        step: u64,
    },
    /// A kept snapshot does not decode as this simulation's snapshot, as when the shape changed.
    Snapshot(String),
    /// The recording's bytes would pass `DECODE_LIMIT`.
    TooLarge(u64),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::NotARecording(reason) => write!(formatter, "not a recording: {reason}"),
            ReplayError::Configuration(reason) => {
                write!(formatter, "the configuration does not decode: {reason}")
            }
            ReplayError::Checkpoints { step } => {
                write!(formatter, "the checkpoint at step {step} can never be compared")
            }
            ReplayError::Snapshot(reason) => {
                write!(formatter, "a kept snapshot does not decode: {reason}")
            }
            ReplayError::TooLarge(bytes) => write!(
                formatter,
                "the recording is {bytes} bytes, over the {DECODE_LIMIT} bytes it could be read back with"
            ),
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
    replay_with::<S>(recording, &mut |_, _| {})
}

/// Sees the runner and the step's events after every replayed step.
pub(crate) type StepObserver<'a, S> = dyn FnMut(&Runner<S>, &[<S as Simulation>::Event]) + 'a;

/// `replay`, calling `observer` after every step with the runner and the events that step
/// produced. The timeline is rebuilt this way.
pub(crate) fn replay_with<S: Simulation>(
    recording: &Recording<S::Intent>,
    observer: &mut StepObserver<'_, S>,
) -> Result<(Runner<S>, ReplayOutcome), ReplayError> {
    let configuration: S::Configuration =
        decode(&recording.configuration).map_err(ReplayError::Configuration)?;
    let mut runner = Runner::new(
        configuration,
        recording.seed,
        recording.step_configuration,
        recording.clock_configuration,
    );
    // Checkpoints must rise strictly; one at or below the last compared, or past the last step,
    // would be skipped without a comparison, so it is an error rather than a silent pass.
    let mut checkpoints = recording.checkpoints.iter().peekable();
    let mut compared: Option<u64> = None;
    let mut check = |runner: &Runner<S>| -> Result<Option<ReplayOutcome>, ReplayError> {
        while let Some((at_step, expected)) = checkpoints.peek().copied() {
            if *at_step > runner.step_number() {
                break;
            }
            checkpoints.next();
            if *at_step != runner.step_number() || compared.is_some_and(|last| last >= *at_step) {
                return Err(ReplayError::Checkpoints { step: *at_step });
            }
            compared = Some(*at_step);
            let actual = runner.hash();
            if actual != *expected {
                return Ok(Some(ReplayOutcome::Diverged {
                    at_step: *at_step,
                    expected: *expected,
                    actual,
                }));
            }
        }
        Ok(None)
    };
    if let Some(outcome) = check(&runner)? {
        return Ok((runner, outcome));
    }
    for step in &recording.steps {
        runner.set_clock_multiplier(step.clock_multiplier);
        let advanced = runner.step_once(&step.intents);
        observer(&runner, &advanced.events);
        if let Some(outcome) = check(&runner)? {
            return Ok((runner, outcome));
        }
    }
    if let Some((step, _)) = checkpoints.next() {
        return Err(ReplayError::Checkpoints { step: *step });
    }
    Ok((runner, ReplayOutcome::Identical))
}

/// Where a replay stopped matching, and what differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bisection {
    /// The last checkpoint whose hash still matched; `None` when the first one already differs,
    /// as when `create` changed.
    pub last_good_step: Option<u64>,
    /// The first checkpoint whose hash differs: the change is in the steps after `last_good_step`
    /// up to and including this one, or in `create` when nothing matched.
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
        .rfind(|step| *step < at_step);
    let differences = match recording
        .snapshots
        .iter()
        .find(|(step, _)| *step == at_step)
    {
        Some((_, bytes)) => {
            let recorded: S::Snapshot = decode(bytes).map_err(ReplayError::Snapshot)?;
            line_differences(&recorded, &runner.snapshot())
        }
        None => Vec::new(),
    };
    Ok(Some(Bisection {
        last_good_step,
        first_bad_step: at_step,
        differences,
    }))
}

/// Past this many pairs of lines, the diff compares line by line instead of searching for the
/// longest common run, to keep `bisect` fast on large snapshots.
const DIFF_CELLS: usize = 4_000_000;

/// The lines of two values' pretty debug output that differ, as `- recorded` and `+ replayed`,
/// with their indentation kept so each line shows whose field it is. Lines both share are left out.
fn line_differences<T: std::fmt::Debug>(recorded: &T, replayed: &T) -> Vec<String> {
    let recorded = format!("{recorded:#?}");
    let replayed = format!("{replayed:#?}");
    let left: Vec<&str> = recorded.lines().collect();
    let right: Vec<&str> = replayed.lines().collect();
    // Shared lines at the start and end need no search.
    let prefix = left.iter().zip(&right).take_while(|(a, b)| a == b).count();
    let suffix = left[prefix..]
        .iter()
        .rev()
        .zip(right[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let left = &left[prefix..left.len() - suffix];
    let right = &right[prefix..right.len() - suffix];
    let mut differences = Vec::new();
    if left.len().saturating_mul(right.len()) > DIFF_CELLS {
        differences.extend(left.iter().map(|line| format!("- {line}")));
        differences.extend(right.iter().map(|line| format!("+ {line}")));
        return differences;
    }
    // Longest common subsequence, then walk it to list what each side has alone.
    let (rows, columns) = (left.len(), right.len());
    let mut lengths = vec![0u32; (rows + 1) * (columns + 1)];
    let at = |row: usize, column: usize| row * (columns + 1) + column;
    for row in (0..rows).rev() {
        for column in (0..columns).rev() {
            lengths[at(row, column)] = if left[row] == right[column] {
                lengths[at(row + 1, column + 1)] + 1
            } else {
                lengths[at(row + 1, column)].max(lengths[at(row, column + 1)])
            };
        }
    }
    let (mut row, mut column) = (0, 0);
    while row < rows || column < columns {
        if row < rows && column < columns && left[row] == right[column] {
            row += 1;
            column += 1;
        } else if column == columns
            || (row < rows && lengths[at(row + 1, column)] >= lengths[at(row, column + 1)])
        {
            differences.push(format!("- {}", left[row]));
            row += 1;
        } else {
            differences.push(format!("+ {}", right[column]));
            column += 1;
        }
    }
    differences
}

#[cfg(test)]
mod tests {
    use super::line_differences;

    #[test]
    fn one_inserted_entry_is_one_line_of_difference_with_its_indentation() {
        let recorded = vec![1, 2, 3, 4, 5, 6];
        let replayed = vec![1, 2, 99, 3, 4, 5, 6];
        assert_eq!(line_differences(&recorded, &replayed), ["+     99,"]);
        assert_eq!(line_differences(&replayed, &recorded), ["-     99,"]);
        assert!(line_differences(&recorded, &recorded).is_empty());
    }
}
