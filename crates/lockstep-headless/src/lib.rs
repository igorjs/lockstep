// SPDX-License-Identifier: Apache-2.0
//! The headless runner: runs a named fixture without a host and prints its state hash.
//!
//! Commands:
//! - `fixture <name> [--seed <number>] [--steps <number>]` runs a fixture and prints its hash.
//! - `verify <name> --expect <file> [--seed <number>] [--steps <number>]` compares the hash with
//!   a committed hash file and fails when they differ.
//! - `record <name> --out <file> [--seed] [--steps] [--every]` writes a recording of the fixture.
//! - `replay <file> [--until <step>] [--save <file>]` runs a recording again and fails when it
//!   diverges; `--save` writes a save where the replay stopped.
//! - `resume <save> --recording <file>` continues a recording from a save, converting an older
//!   save, and fails when a later checkpoint differs.
//! - `bisect <file>` names the checkpoints around the first divergence and what differs.
//! - `stats <file>` prints the session's length, intents per minute and multiplier use.

pub mod recordings;

use recordings::replayables;
use std::fs;

pub struct Fixture {
    pub name: &'static str,
    pub default_seed: u64,
    pub default_steps: u64,
    pub run: fn(seed: u64, steps: u64) -> u64,
}

pub fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "capsule",
            default_seed: capsule::default_seed(),
            default_steps: capsule::default_steps(),
            run: capsule::fixture_hash,
        },
        Fixture {
            name: "mars-rovers",
            default_seed: mars_rovers::DEFAULT_SEED,
            default_steps: mars_rovers::DEFAULT_STEPS,
            run: mars_rovers::fixture_hash,
        },
        Fixture {
            name: "crowd",
            default_seed: crowd::DEFAULT_SEED,
            default_steps: crowd::DEFAULT_STEPS,
            run: crowd::fixture_hash,
        },
        Fixture {
            name: "drone-fleet",
            default_seed: drone_fleet::DEFAULT_SEED,
            default_steps: drone_fleet::DEFAULT_STEPS,
            run: drone_fleet::fixture_hash,
        },
        Fixture {
            name: "cargo-bay",
            default_seed: cargo_bay::DEFAULT_SEED,
            default_steps: cargo_bay::DEFAULT_STEPS,
            run: cargo_bay::fixture_hash,
        },
        Fixture {
            name: "crater-survey",
            default_seed: crater_survey::DEFAULT_SEED,
            default_steps: crater_survey::DEFAULT_STEPS,
            run: crater_survey::fixture_hash,
        },
        Fixture {
            name: "audit",
            default_seed: audit::DEFAULT_SEED,
            default_steps: audit::DEFAULT_STEPS,
            run: audit::fixture_hash,
        },
        Fixture {
            name: "training",
            default_seed: training::DEFAULT_SEED,
            default_steps: training::DEFAULT_STEPS,
            run: training::fixture_hash,
        },
        Fixture {
            name: "dispatch",
            default_seed: dispatch::DEFAULT_SEED,
            default_steps: dispatch::DEFAULT_STEPS,
            run: dispatch::fixture_hash,
        },
        Fixture {
            name: "bakery",
            default_seed: bakery::DEFAULT_SEED,
            default_steps: bakery::DEFAULT_STEPS,
            run: bakery::fixture_hash,
        },
        Fixture {
            name: "estate",
            default_seed: estate::DEFAULT_SEED,
            default_steps: estate::DEFAULT_STEPS,
            run: estate::fixture_hash,
        },
        Fixture {
            name: "checkout",
            default_seed: checkout::DEFAULT_SEED,
            default_steps: checkout::DEFAULT_STEPS,
            run: checkout::fixture_hash,
        },
        Fixture {
            name: "roster",
            default_seed: roster::DEFAULT_SEED,
            default_steps: roster::DEFAULT_STEPS,
            run: roster::fixture_hash,
        },
        Fixture {
            name: "orchard",
            default_seed: orchard::DEFAULT_SEED,
            default_steps: orchard::DEFAULT_STEPS,
            run: orchard::fixture_hash,
        },
        Fixture {
            name: "sparring",
            default_seed: sparring::DEFAULT_SEED,
            default_steps: sparring::DEFAULT_STEPS,
            run: sparring::fixture_hash,
        },
        Fixture {
            name: "rates",
            default_seed: rates::DEFAULT_SEED,
            default_steps: rates::DEFAULT_STEPS,
            run: rates::fixture_hash,
        },
        Fixture {
            name: "tariffs",
            default_seed: tariffs::DEFAULT_SEED,
            default_steps: tariffs::DEFAULT_STEPS,
            run: tariffs::fixture_hash,
        },
        Fixture {
            name: "ledger",
            default_seed: ledger::DEFAULT_SEED,
            default_steps: ledger::DEFAULT_STEPS,
            run: ledger::fixture_hash,
        },
        Fixture {
            name: "ledger-bills",
            default_seed: ledger::DEFAULT_SEED,
            default_steps: ledger::bills::DEFAULT_ROUNDS,
            run: ledger::bills::bill_fixture_hash,
        },
    ]
}

pub fn format_hash(hash: u64) -> String {
    format!("{hash:016x}")
}

#[derive(Debug, PartialEq)]
pub enum CommandError {
    Usage(String),
    UnknownFixture(String),
    Unreadable(String),
    Mismatch {
        name: String,
        expected: String,
        actual: String,
    },
    /// A recording that cannot be read or run.
    Recording(String),
    /// A file that cannot be written.
    Unwritable(String),
    /// A replay that no longer matches its recording.
    Diverged(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Usage(message) => write!(formatter, "{message}"),
            CommandError::UnknownFixture(name) => {
                let known: Vec<&str> = fixtures().iter().map(|fixture| fixture.name).collect();
                write!(
                    formatter,
                    "unknown fixture '{name}', known fixtures: {}",
                    known.join(", ")
                )
            }
            CommandError::Unreadable(message) => write!(formatter, "{message}"),
            CommandError::Mismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "fixture '{name}' diverged: expected {expected}, got {actual}"
            ),
            CommandError::Recording(message)
            | CommandError::Unwritable(message)
            | CommandError::Diverged(message) => {
                write!(formatter, "{message}")
            }
        }
    }
}

const USAGE: &str = "usage: lockstep-headless fixture <name> [--seed <number>] [--steps <number>]
       lockstep-headless verify <name> --expect <file> [--seed <number>] [--steps <number>]
       lockstep-headless record <name> --out <file> [--seed <number>] [--steps <number>] [--every <number>]
       lockstep-headless replay <recording> [--until <step>] [--save <file>]
       lockstep-headless resume <save> --recording <recording>
       lockstep-headless bisect <recording>
       lockstep-headless stats <recording>";

/// Checkpoint spacing for `record` when `--every` is not given.
const DEFAULT_CHECKPOINT_EVERY: u64 = 100;

struct Options {
    seed: Option<u64>,
    steps: Option<u64>,
    expect: Option<String>,
    out: Option<String>,
    every: Option<u64>,
    until: Option<u64>,
    save: Option<String>,
    recording: Option<String>,
}

fn parse_options(arguments: &[String]) -> Result<Options, CommandError> {
    let mut options = Options {
        seed: None,
        steps: None,
        expect: None,
        out: None,
        every: None,
        until: None,
        save: None,
        recording: None,
    };
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| CommandError::Usage(format!("{flag} needs a value\n{USAGE}")))?;
        match flag {
            "--seed" => options.seed = Some(parse_number(flag, value)?),
            "--steps" => options.steps = Some(parse_number(flag, value)?),
            "--expect" => options.expect = Some(value.clone()),
            "--out" => options.out = Some(value.clone()),
            "--every" => options.every = Some(parse_number(flag, value)?.max(1)),
            "--until" => options.until = Some(parse_number(flag, value)?),
            "--save" => options.save = Some(value.clone()),
            "--recording" => options.recording = Some(value.clone()),
            other => {
                return Err(CommandError::Usage(format!(
                    "unknown option {other}\n{USAGE}"
                )))
            }
        }
        index += 2;
    }
    Ok(options)
}

/// Each command takes only its own options, so a misplaced one is an error, not ignored.
fn refuse_unused_options(command: &str, options: &Options) -> Result<(), CommandError> {
    let given = [
        ("--seed", options.seed.is_some()),
        ("--steps", options.steps.is_some()),
        ("--expect", options.expect.is_some()),
        ("--out", options.out.is_some()),
        ("--every", options.every.is_some()),
        ("--until", options.until.is_some()),
        ("--save", options.save.is_some()),
        ("--recording", options.recording.is_some()),
    ];
    let allowed: &[&str] = match command {
        "fixture" => &["--seed", "--steps"],
        "verify" => &["--seed", "--steps", "--expect"],
        "record" => &["--seed", "--steps", "--out", "--every"],
        "replay" => &["--until", "--save"],
        "resume" => &["--recording"],
        _ => &[],
    };
    match given
        .iter()
        .find(|(flag, present)| *present && !allowed.contains(flag))
    {
        Some((flag, _)) => Err(CommandError::Usage(format!(
            "{command} does not take {flag}\n{USAGE}"
        ))),
        None => Ok(()),
    }
}

fn parse_number(flag: &str, value: &str) -> Result<u64, CommandError> {
    value
        .parse()
        .map_err(|_| CommandError::Usage(format!("{flag} needs a whole number, got '{value}'")))
}

fn run_named(name: &str, options: &Options) -> Result<u64, CommandError> {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture.name == name)
        .ok_or_else(|| CommandError::UnknownFixture(name.to_string()))?;
    let seed = options.seed.unwrap_or(fixture.default_seed);
    let steps = options.steps.unwrap_or(fixture.default_steps);
    Ok((fixture.run)(seed, steps))
}

/// Runs one command and returns the line to print.
pub fn run(arguments: &[String]) -> Result<String, CommandError> {
    let (command, rest) = arguments
        .split_first()
        .ok_or_else(|| CommandError::Usage(USAGE.to_string()))?;
    let (name, rest) = rest
        .split_first()
        .ok_or_else(|| CommandError::Usage(USAGE.to_string()))?;
    let options = parse_options(rest)?;
    refuse_unused_options(command, &options)?;
    match command.as_str() {
        "fixture" => Ok(format_hash(run_named(name, &options)?)),
        "verify" => {
            let path = options.expect.clone().ok_or_else(|| {
                CommandError::Usage(format!("verify needs --expect <file>\n{USAGE}"))
            })?;
            let expected = fs::read_to_string(&path)
                .map_err(|error| CommandError::Unreadable(format!("cannot read {path}: {error}")))?
                .trim()
                .to_string();
            let actual = format_hash(run_named(name, &options)?);
            if actual == expected {
                Ok(format!("ok {name} {actual}"))
            } else {
                Err(CommandError::Mismatch {
                    name: name.clone(),
                    expected,
                    actual,
                })
            }
        }
        "record" => {
            let fixture = fixtures()
                .into_iter()
                .find(|fixture| fixture.name == name)
                .ok_or_else(|| CommandError::UnknownFixture(name.to_string()))?;
            let replayable = replayables()
                .into_iter()
                .find(|replayable| replayable.id == name)
                .ok_or_else(|| {
                    CommandError::Recording(format!(
                        "fixture '{name}' cannot be recorded, see decision 0043"
                    ))
                })?;
            let out = options.out.clone().ok_or_else(|| {
                CommandError::Usage(format!("record needs --out <file>\n{USAGE}"))
            })?;
            let bytes = (replayable.record)(
                options.seed.unwrap_or(fixture.default_seed),
                options.steps.unwrap_or(fixture.default_steps),
                options.every.unwrap_or(DEFAULT_CHECKPOINT_EVERY),
            )
            .map_err(CommandError::Recording)?;
            fs::write(&out, &bytes).map_err(|error| {
                CommandError::Unwritable(format!("cannot write {out}: {error}"))
            })?;
            Ok(format!("recorded {name} to {out} ({} bytes)", bytes.len()))
        }
        "resume" => {
            let recording_path = options.recording.clone().ok_or_else(|| {
                CommandError::Usage(format!("resume needs --recording <file>\n{USAGE}"))
            })?;
            let save_bytes = fs::read(name).map_err(|error| {
                CommandError::Unreadable(format!("cannot read {name}: {error}"))
            })?;
            let bytes = fs::read(&recording_path).map_err(|error| {
                CommandError::Unreadable(format!("cannot read {recording_path}: {error}"))
            })?;
            let saved = recordings::saved_id(&save_bytes).map_err(CommandError::Recording)?;
            let recorded = recordings::simulation_id(&bytes).map_err(CommandError::Recording)?;
            if saved != recorded {
                return Err(CommandError::Recording(format!(
                    "the save is for '{saved}' but the recording is for '{recorded}'"
                )));
            }
            let replayable = replayables()
                .into_iter()
                .find(|replayable| replayable.id == saved)
                .ok_or_else(|| {
                    CommandError::Recording(format!(
                        "no simulation called '{saved}' can be resumed"
                    ))
                })?;
            let resumed =
                (replayable.resume)(&save_bytes, &bytes).map_err(CommandError::Recording)?;
            if resumed.identical {
                Ok(resumed.report)
            } else {
                Err(CommandError::Diverged(resumed.report))
            }
        }
        "replay" | "bisect" | "stats" => {
            let bytes = fs::read(name).map_err(|error| {
                CommandError::Unreadable(format!("cannot read {name}: {error}"))
            })?;
            let id = recordings::simulation_id(&bytes).map_err(CommandError::Recording)?;
            let replayable = replayables()
                .into_iter()
                .find(|replayable| replayable.id == id)
                .ok_or_else(|| {
                    CommandError::Recording(format!("no simulation called '{id}' can be replayed"))
                })?;
            match command.as_str() {
                "replay" => {
                    let replayed =
                        (replayable.replay)(&bytes, options.until, options.save.is_some())
                            .map_err(CommandError::Recording)?;
                    if replayed.identical {
                        if let (Some(path), Some(save)) = (&options.save, &replayed.save) {
                            fs::write(path, save).map_err(|error| {
                                CommandError::Unwritable(format!("cannot write {path}: {error}"))
                            })?;
                            return Ok(format!(
                                "{}\nsaved to {path} ({} bytes)",
                                replayed.report,
                                save.len()
                            ));
                        }
                        Ok(replayed.report)
                    } else {
                        Err(CommandError::Diverged(replayed.report))
                    }
                }
                "bisect" => {
                    let found = (replayable.bisect)(&bytes).map_err(CommandError::Recording)?;
                    if found.identical {
                        Ok(found.report)
                    } else {
                        Err(CommandError::Diverged(found.report))
                    }
                }
                _ => (replayable.stats)(&bytes).map_err(CommandError::Recording),
            }
        }
        other => Err(CommandError::Usage(format!(
            "unknown command {other}\n{USAGE}"
        ))),
    }
}
