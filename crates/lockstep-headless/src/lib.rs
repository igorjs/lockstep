// SPDX-License-Identifier: Apache-2.0
//! The headless runner: runs a named fixture without a host and prints its state hash.
//!
//! Commands:
//! - `fixture <name> [--seed <number>] [--steps <number>]` runs a fixture and prints its hash.
//! - `verify <name> --expect <file> [--seed <number>] [--steps <number>]` compares the hash with
//!   a committed hash file and fails when they differ.

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
        }
    }
}

const USAGE: &str = "usage: lockstep-headless fixture <name> [--seed <number>] [--steps <number>]\n       lockstep-headless verify <name> --expect <file> [--seed <number>] [--steps <number>]";

struct Options {
    seed: Option<u64>,
    steps: Option<u64>,
    expect: Option<String>,
}

fn parse_options(arguments: &[String]) -> Result<Options, CommandError> {
    let mut options = Options {
        seed: None,
        steps: None,
        expect: None,
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
        other => Err(CommandError::Usage(format!(
            "unknown command {other}\n{USAGE}"
        ))),
    }
}
