// SPDX-License-Identifier: Apache-2.0
//! Commands on recorded sessions: `record`, `replay`, `bisect` and `stats`. A recording names its
//! simulation, and the table below finds the code that can run it.

use lockstep_core::{
    bisect, hash_of, recorded_simulation_id, replay, replay_from, saved_simulation_id, Message,
    Migrations, Recording, RecordingSet, ReplayOutcome, Save, Simulation,
};
use mars_rovers::{Hex, MarsRovers, Square4, Square8};

/// A simulation the headless runner can record and replay.
pub struct Replayable {
    pub id: &'static str,
    /// Records the fixture session: seed, steps, checkpoint every.
    pub record: fn(u64, u64, u64) -> Result<Vec<u8>, String>,
    /// Replays, stopping after `until` steps when given; the flag asks for a save at the stop.
    pub replay: fn(&[u8], Option<u64>, bool) -> ReplayResult,
    /// Continues a recording from a save: save bytes, recording bytes.
    pub resume: fn(&[u8], &[u8]) -> Result<Replayed, String>,
    pub bisect: fn(&[u8]) -> Result<Replayed, String>,
    pub stats: fn(&[u8]) -> Result<String, String>,
    /// The conversions that load this simulation's older saves.
    pub migrations: fn() -> Migrations,
    /// The schema of each of its four message types, by role.
    pub schemas: fn() -> Schemas,
}

/// (role, schema text) for a simulation's intent, event, snapshot and configuration.
pub type Schemas = Vec<(&'static str, &'static str)>;

/// The schemas of a simulation's four message types (decision 0062).
pub fn schemas_of<S: Simulation>() -> Schemas {
    vec![
        ("intent", <S::Intent as Message>::SCHEMA),
        ("event", <S::Event as Message>::SCHEMA),
        ("snapshot", <S::Snapshot as Message>::SCHEMA),
        ("configuration", <S::Configuration as Message>::SCHEMA),
    ]
}

/// The schemas of a fixture's simulation, or `None` for a name that is not a fixture.
pub fn schemas_for(name: &str) -> Option<Schemas> {
    let found = replayables()
        .into_iter()
        .find(|replayable| replayable.id == name)
        .map(|replayable| (replayable.schemas)());
    found.or_else(|| match name {
        "ledger-bills" => Some(schemas_of::<ledger::Ledger>()),
        _ => None,
    })
}

type ReplayResult = Result<Replayed, String>;

/// What a replay or a bisect found: `identical` is false when the session diverged.
#[derive(Debug, PartialEq)]
pub struct Replayed {
    pub identical: bool,
    pub report: String,
    /// The save bytes taken where the replay stopped, when one was asked for and it matched.
    pub save: Option<Vec<u8>>,
}

/// One table entry for a crate that exports `SIMULATION_ID` and `record_fixture`. The ledger's
/// snapshots are small and so are the others', so every recording keeps one per checkpoint for
/// bisect.
macro_rules! replayable {
    ($krate:ident, $simulation:ty) => {
        replayable!($krate, $simulation, Migrations::new)
    };
    ($krate:ident, $simulation:ty, $migrations:expr) => {
        Replayable {
            id: $krate::SIMULATION_ID,
            record: |seed, steps, every| {
                $krate::record_fixture(seed, steps, every, true)
                    .to_bytes()
                    .map_err(|error| error.to_string())
            },
            replay: replay_report::<$simulation>,
            resume: resume_report::<$simulation>,
            bisect: bisect_report::<$simulation>,
            stats: stats_report::<$simulation>,
            migrations: $migrations,
            schemas: schemas_of::<$simulation>,
        }
    };
}

/// Every example with a fixture. `mars-rovers` and `roster` record as a set of sessions (decisions
/// 0061 and 0067; the second project of `roster` carries the folded chronicle in its recorded
/// configuration) and `orchard` sets its own daylight (decision 0060).
pub fn replayables() -> Vec<Replayable> {
    vec![
        replayable!(ledger, ledger::Ledger),
        replayable!(tariffs, tariffs::Tariffs, tariffs::migrations),
        replayable!(capsule, capsule::Capsule),
        replayable!(crowd, crowd::Crowd),
        replayable!(drone_fleet, drone_fleet::Fleet),
        replayable!(sparring, sparring::Sparring),
        replayable!(cargo_bay, cargo_bay::CargoBay),
        replayable!(crater_survey, crater_survey::Survey),
        replayable!(audit, audit::Audit),
        replayable!(training, training::Training),
        replayable!(dispatch, dispatch::Dispatch),
        replayable!(bakery, bakery::Bakery),
        replayable!(estate, estate::Estate),
        replayable!(checkout, checkout::Checkout),
        replayable!(rates, rates::Rates),
        replayable!(orchard, orchard::Orchard),
        Replayable {
            id: mars_rovers::SIMULATION_ID,
            record: |seed, steps, every| {
                mars_rovers::record_fixture(seed, steps, every, true)
                    .to_bytes()
                    .map_err(|error| error.to_string())
            },
            replay: replay_set_report,
            resume: |_, _| Err(NO_SET_SAVE.to_string()),
            bisect: bisect_set_report,
            stats: stats_set_report::<mars_rovers::Intent>,
            migrations: Migrations::new,
            schemas: schemas_of::<MarsRovers<Square4>>,
        },
        Replayable {
            id: roster::SIMULATION_ID,
            record: |seed, steps, every| {
                roster::record_fixture(seed, steps, every, true)
                    .to_bytes()
                    .map_err(|error| error.to_string())
            },
            replay: |bytes, until, save| {
                replay_set::<roster::Intent>(bytes, until, save, &|_, recording| {
                    replay_session::<roster::Roster>(recording)
                })
            },
            resume: |_, _| Err(NO_SET_SAVE.to_string()),
            bisect: |bytes| {
                bisect_set::<roster::Intent>(bytes, &|_, recording| {
                    bisect_recording::<roster::Roster>(recording)
                })
            },
            stats: stats_set_report::<roster::Intent>,
            migrations: Migrations::new,
            schemas: schemas_of::<roster::Roster>,
        },
    ]
}

/// The simulation id at the front of a save's bytes.
pub fn saved_id(bytes: &[u8]) -> Result<String, String> {
    saved_simulation_id(bytes).map_err(|error| error.to_string())
}

/// The simulation id at the front of a recording's bytes.
pub fn simulation_id(bytes: &[u8]) -> Result<String, String> {
    recorded_simulation_id(bytes).map_err(|error| error.to_string())
}

fn read<S: Simulation>(bytes: &[u8]) -> Result<Recording<S::Intent>, String> {
    Recording::from_bytes(bytes).map_err(|error| error.to_string())
}

/// Keeps the first `until` steps and the checkpoints within them.
fn truncate<I: Message>(recording: &mut Recording<I>, until: u64) {
    recording
        .steps
        .truncate(until.min(recording.steps.len() as u64) as usize);
    recording.checkpoints.retain(|(step, _)| *step <= until);
    recording.snapshots.retain(|(step, _)| *step <= until);
}

fn replay_report<S: Simulation>(
    bytes: &[u8],
    until: Option<u64>,
    save: bool,
) -> Result<Replayed, String> {
    let mut recording = read::<S>(bytes)?;
    if let Some(until) = until {
        truncate(&mut recording, until);
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
            save: if save {
                Some(
                    runner
                        .save(&recording.simulation_id)
                        .to_bytes()
                        .map_err(|error| error.to_string())?,
                )
            } else {
                None
            },
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
            save: None,
        },
    })
}

/// Loads the save, converting an older shape, and runs the recording from the save's step on,
/// comparing every checkpoint from there.
fn resume_report<S: Simulation>(save_bytes: &[u8], bytes: &[u8]) -> Result<Replayed, String> {
    let recording = read::<S>(bytes)?;
    let save = Save::from_bytes(save_bytes).map_err(|error| error.to_string())?;
    let migrations = replayables()
        .into_iter()
        .find(|replayable| replayable.id == save.simulation_id)
        .map(|replayable| (replayable.migrations)())
        .unwrap_or_default();
    let (runner, outcome) =
        replay_from::<S>(&recording, &save, &migrations).map_err(|error| error.to_string())?;
    Ok(match outcome {
        ReplayOutcome::Identical => Replayed {
            identical: true,
            report: format!(
                "identical {} from step {} to step {}, hash {:016x}",
                recording.simulation_id,
                save.step_number,
                runner.step_number(),
                runner.hash()
            ),
            save: None,
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
            save: None,
        },
    })
}

fn bisect_report<S: Simulation>(bytes: &[u8]) -> Result<Replayed, String> {
    bisect_recording::<S>(&read::<S>(bytes)?)
}

fn bisect_recording<S: Simulation>(recording: &Recording<S::Intent>) -> Result<Replayed, String> {
    let found = bisect::<S>(recording).map_err(|error| error.to_string())?;
    let Some(found) = found else {
        return Ok(Replayed {
            identical: true,
            report: format!(
                "no divergence in {} steps of {}",
                recording.steps.len(),
                recording.simulation_id
            ),
            save: None,
        });
    };
    let mut lines = vec![
        match found.last_good_step {
            Some(step) => format!("last good checkpoint: step {step}"),
            None => "last good checkpoint: none, the session differs from its start".to_string(),
        },
        format!("first bad checkpoint: step {}", found.first_bad_step),
    ];
    if !found.compared_snapshots {
        lines.push("the recording kept no snapshot here, so no difference to show".to_string());
    } else if found.differences.is_empty() {
        lines.push("the kept snapshot matches, so only the recorded hash differs".to_string());
    } else {
        lines.extend(found.differences);
    }
    Ok(Replayed {
        identical: false,
        report: lines.join("\n"),
        save: None,
    })
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

const NO_SET_SAVE: &str =
    "a recording set holds many sessions, so there is no one state to save or resume";

fn read_set<I: Message>(bytes: &[u8]) -> Result<RecordingSet<I>, String> {
    RecordingSet::from_bytes(bytes).map_err(|error| error.to_string())
}

/// Runs one session of a set on the topology its label names.
fn on_topology<R>(
    label: &str,
    square4: impl FnOnce() -> Result<R, String>,
    square8: impl FnOnce() -> Result<R, String>,
    hex: impl FnOnce() -> Result<R, String>,
) -> Result<R, String> {
    match label {
        label if label == mars_rovers::TOPOLOGIES[0] => square4(),
        label if label == mars_rovers::TOPOLOGIES[1] => square8(),
        label if label == mars_rovers::TOPOLOGIES[2] => hex(),
        other => Err(format!("a session on an unknown topology: {other}")),
    }
}

/// Replays one session, stopping at the first checkpoint that differs.
fn replay_session<S: Simulation>(
    recording: &Recording<S::Intent>,
) -> Result<(u64, u64, ReplayOutcome), String> {
    let (runner, outcome) = replay::<S>(recording).map_err(|error| error.to_string())?;
    Ok((runner.step_number(), runner.hash(), outcome))
}

/// What one session of a set replays to: its steps, its final hash and the outcome.
type SessionReplay<I> = dyn Fn(&str, &Recording<I>) -> Result<(u64, u64, ReplayOutcome), String>;

/// Bisects one session of a set.
type SessionBisect<I> = dyn Fn(&str, &Recording<I>) -> Result<Replayed, String>;

fn replay_set_report(bytes: &[u8], until: Option<u64>, save: bool) -> Result<Replayed, String> {
    replay_set::<mars_rovers::Intent>(bytes, until, save, &|label, recording| {
        on_topology(
            label,
            || replay_session::<MarsRovers<Square4>>(recording),
            || replay_session::<MarsRovers<Square8>>(recording),
            || replay_session::<MarsRovers<Hex>>(recording),
        )
    })
}

fn replay_set<I: Message>(
    bytes: &[u8],
    until: Option<u64>,
    save: bool,
    session: &SessionReplay<I>,
) -> Result<Replayed, String> {
    if save {
        return Err(NO_SET_SAVE.to_string());
    }
    let set = read_set::<I>(bytes)?;
    let mut hashes = Vec::new();
    let mut steps = 0;
    for (index, (label, recording)) in set.sessions.iter().enumerate() {
        let mut recording = recording.clone();
        if let Some(until) = until {
            truncate(&mut recording, until);
        }
        let (session_steps, hash, outcome) = session(label, &recording)?;
        if let ReplayOutcome::Diverged {
            at_step,
            expected,
            actual,
        } = outcome
        {
            return Ok(Replayed {
                identical: false,
                report: format!(
                    "diverged {} in session {index} ({label}) at step {at_step}: recorded {expected:016x}, replayed {actual:016x}",
                    set.simulation_id
                ),
                save: None,
            });
        }
        steps = session_steps;
        hashes.push(hash);
    }
    Ok(Replayed {
        identical: true,
        report: format!(
            "identical {} after {} sessions of {steps} steps, hash {:016x}",
            set.simulation_id,
            hashes.len(),
            hash_of(&hashes)
        ),
        save: None,
    })
}

fn bisect_set_report(bytes: &[u8]) -> Result<Replayed, String> {
    bisect_set::<mars_rovers::Intent>(bytes, &|label, recording| {
        on_topology(
            label,
            || bisect_recording::<MarsRovers<Square4>>(recording),
            || bisect_recording::<MarsRovers<Square8>>(recording),
            || bisect_recording::<MarsRovers<Hex>>(recording),
        )
    })
}

fn bisect_set<I: Message>(bytes: &[u8], session: &SessionBisect<I>) -> Result<Replayed, String> {
    let set = read_set::<I>(bytes)?;
    for (index, (label, recording)) in set.sessions.iter().enumerate() {
        let report = session(label, recording)?;
        if !report.identical {
            return Ok(Replayed {
                identical: false,
                report: format!("session {index} ({label})\n{}", report.report),
                save: None,
            });
        }
    }
    let steps: usize = set
        .sessions
        .iter()
        .map(|(_, session)| session.steps.len())
        .sum();
    Ok(Replayed {
        identical: true,
        report: format!(
            "no divergence in {steps} steps of {} sessions of {}",
            set.sessions.len(),
            set.simulation_id
        ),
        save: None,
    })
}

fn stats_set_report<I: Message>(bytes: &[u8]) -> Result<String, String> {
    let set = read_set::<I>(bytes)?;
    let sessions = set.sessions.iter().map(|(_, session)| session);
    let steps: usize = sessions.clone().map(|session| session.steps.len()).sum();
    let intents: usize = sessions
        .clone()
        .flat_map(|session| session.steps.iter())
        .map(|step| step.intents.len())
        .sum();
    let checkpoints: usize = sessions
        .clone()
        .map(|session| session.checkpoints.len())
        .sum();
    let snapshots: usize = sessions.map(|session| session.snapshots.len()).sum();
    Ok([
        format!("simulation {}", set.simulation_id),
        format!("sessions {}", set.sessions.len()),
        format!("steps {steps}"),
        format!("intents {intents}"),
        format!("checkpoints {checkpoints}, snapshots {snapshots}"),
    ]
    .join("\n"))
}
