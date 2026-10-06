// SPDX-License-Identifier: Apache-2.0
//! Commands on recorded sessions: `record`, `replay`, `bisect` and `stats`. A recording names its
//! simulation, and the table below finds the code that can run it.

use lockstep_core::{bisect, replay, Recording, ReplayOutcome, Simulation};

/// A simulation the headless runner can record and replay.
pub struct Replayable {
    pub id: &'static str,
    /// Records the fixture session: seed, steps, checkpoint every.
    pub record: fn(u64, u64, u64) -> Result<Vec<u8>, String>,
    pub replay: fn(&[u8], Option<u64>) -> Result<Replayed, String>,
    pub bisect: fn(&[u8]) -> Result<String, String>,
    pub stats: fn(&[u8]) -> Result<String, String>,
}

/// What a replay found.
#[derive(Debug, PartialEq)]
pub struct Replayed {
    pub identical: bool,
    pub report: String,
}

pub fn replayables() -> Vec<Replayable> {
    vec![Replayable {
        id: ledger::SIMULATION_ID,
        record: |seed, steps, every| {
            ledger::record_fixture(seed, steps, every)
                .to_bytes()
                .map_err(|error| error.to_string())
        },
        replay: replay_report::<ledger::Ledger>,
        bisect: bisect_report::<ledger::Ledger>,
        stats: stats_report::<ledger::Ledger>,
    }]
}

/// The simulation id at the start of a recording's bytes: a little-endian 64-bit length, then
/// that many bytes of text, as the fixed-endian encoding writes the first field.
pub fn simulation_id(bytes: &[u8]) -> Result<String, String> {
    let length = bytes
        .get(..8)
        .map(|prefix| u64::from_le_bytes(prefix.try_into().expect("eight bytes")))
        .ok_or("not a recording: too short")?;
    let text = usize::try_from(length)
        .ok()
        .and_then(|length| bytes.get(8..8usize.checked_add(length)?))
        .ok_or("not a recording: the simulation id runs past the end")?;
    String::from_utf8(text.to_vec())
        .map_err(|_| "not a recording: the simulation id is not text".to_string())
}

fn read<S: Simulation>(bytes: &[u8]) -> Result<Recording<S::Intent>, String> {
    Recording::from_bytes(bytes).map_err(|error| error.to_string())
}

/// Keeps the first `until` steps and the checkpoints within them.
fn truncate<S: Simulation>(recording: &mut Recording<S::Intent>, until: u64) {
    recording
        .steps
        .truncate(until.min(recording.steps.len() as u64) as usize);
    recording.checkpoints.retain(|(step, _)| *step <= until);
    recording.snapshots.retain(|(step, _)| *step <= until);
}

fn replay_report<S: Simulation>(bytes: &[u8], until: Option<u64>) -> Result<Replayed, String> {
    let mut recording = read::<S>(bytes)?;
    if let Some(until) = until {
        truncate::<S>(&mut recording, until);
    }
    let (runner, outcome) = replay::<S>(&recording).map_err(|error| error.to_string())?;
    Ok(match outcome {
        ReplayOutcome::Identical => Replayed {
            identical: true,
            report: format!(
                "identical {} after {} steps, hash {:016x}",
                recording.simulation_id,
                runner.step_number(),
                runner.hash()
            ),
        },
        ReplayOutcome::Diverged {
            at_step,
            expected,
            actual,
        } => Replayed {
            identical: false,
            report: format!(
                "diverged {} at step {at_step}: recorded {expected:016x}, replayed {actual:016x}",
                recording.simulation_id
            ),
        },
    })
}

fn bisect_report<S: Simulation>(bytes: &[u8]) -> Result<String, String> {
    let recording = read::<S>(bytes)?;
    let found = bisect::<S>(&recording).map_err(|error| error.to_string())?;
    let Some(found) = found else {
        return Ok(format!(
            "no divergence in {} steps of {}",
            recording.steps.len(),
            recording.simulation_id
        ));
    };
    let mut lines = vec![
        match found.last_good_step {
            Some(step) => format!("last good checkpoint: step {step}"),
            None => "last good checkpoint: none, the session differs from its start".to_string(),
        },
        format!("first bad checkpoint: step {}", found.first_bad_step),
    ];
    if found.differences.is_empty() {
        lines.push("the recording kept no snapshot here, so no difference to show".to_string());
    } else {
        lines.extend(found.differences);
    }
    Ok(lines.join("\n"))
}

fn stats_report<S: Simulation>(bytes: &[u8]) -> Result<String, String> {
    let recording = read::<S>(bytes)?;
    let steps = recording.steps.len() as u64;
    let real_seconds = steps as f64 * recording.step_configuration.step_seconds as f64;
    let intents: u64 = recording
        .steps
        .iter()
        .map(|step| step.intents.len() as u64)
        .sum();
    let per_minute = if real_seconds > 0.0 {
        intents as f64 * 60.0 / real_seconds
    } else {
        0.0
    };
    // Steps at each multiplier, in the order each multiplier was first used.
    let mut multipliers: Vec<(f32, u64)> = Vec::new();
    for step in &recording.steps {
        match multipliers
            .iter_mut()
            .find(|(value, _)| value.to_bits() == step.clock_multiplier.to_bits())
        {
            Some((_, count)) => *count += 1,
            None => multipliers.push((step.clock_multiplier, 1)),
        }
    }
    let multipliers = multipliers
        .iter()
        .map(|(value, count)| format!("{value} for {count} steps"))
        .collect::<Vec<_>>()
        .join(", ");
    Ok([
        format!("simulation {}", recording.simulation_id),
        format!("seed {}", recording.seed),
        format!("steps {steps} ({real_seconds:.1} real seconds)"),
        format!("intents {intents} ({per_minute:.1} per real minute)"),
        format!(
            "clock multipliers: {}",
            if multipliers.is_empty() {
                "none".to_string()
            } else {
                multipliers
            }
        ),
        format!(
            "checkpoints {}, snapshots {}",
            recording.checkpoints.len(),
            recording.snapshots.len()
        ),
    ]
    .join("\n"))
}
