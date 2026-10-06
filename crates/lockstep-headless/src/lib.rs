// SPDX-License-Identifier: Apache-2.0
//! The headless runner: runs a named fixture without a host and prints its state hash.
//!
//! Commands:
//! - `fixture <name> [--seed <number>] [--steps <number>]` runs a fixture and prints its hash.
//! - `verify <name> --expect <file> [--seed <number>] [--steps <number>]` compares the hash with
//!   a committed hash file and fails when they differ.
//! - `record <name> --out <file> [--seed] [--steps] [--every]` writes a recording of the fixture.
//! - `replay <file> [--until <step>]` runs a recording again and fails when it diverges.
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
            name: "ledger",
            default_seed: ledger::DEFAULT_SEED,
            default_steps: ledger::DEFAULT_STEPS,
            run: ledger::fixture_hash,
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
       lockstep-headless replay <recording> [--until <step>]
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
}

fn parse_options(arguments: &[String]) -> Result<Options, CommandError> {
    let mut options = Options {
        seed: None,
        steps: None,
        expect: None,
        out: None,
        every: None,
        until: None,
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
    ];
    let allowed: &[&str] = match command {
        "fixture" => &["--seed", "--steps"],
        "verify" => &["--seed", "--steps", "--expect"],
        "record" => &["--seed", "--steps", "--out", "--every"],
        "replay" => &["--until"],
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
                    CommandError::Recording(format!("fixture '{name}' cannot be recorded yet"))
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
                    let replayed = (replayable.replay)(&bytes, options.until)
                        .map_err(CommandError::Recording)?;
                    if replayed.identical {
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
