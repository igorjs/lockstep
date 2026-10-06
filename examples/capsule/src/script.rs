// SPDX-License-Identifier: Apache-2.0
//! A tiny text format for scripted sessions, run by the headless runner.
//!
//! ```text
//! seed 20260925
//! steps 9000
//! 10 move survivor 15 15 walk
//! 200 wound survivor
//! 800 multiplier 20
//! ```
//!
//! Commands at one step are applied before that step runs, in file order: a multiplier takes
//! effect before the step, and intents queue for it. A file must give `seed` and `steps` exactly
//! once, and a line may not carry extra words. A `ParseError` with line 0 refers to the whole file.

use crate::{runner, Capsule, Cell, Intent};
use lockstep_core::Runner;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Move { cell: Cell, run: bool },
    Stop,
    Wound,
    Bandage,
    Pray,
    Multiplier(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    pub seed: u64,
    pub steps: u64,
    /// Commands in file order, each with the step it applies to.
    pub commands: Vec<(u64, Command)>,
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

fn number<T: std::str::FromStr>(
    word: Option<&str>,
    line: usize,
    what: &str,
) -> Result<T, ParseError> {
    word.and_then(|text| text.parse().ok()).ok_or(ParseError {
        line,
        message: format!("expected {what}"),
    })
}

pub fn parse(text: &str) -> Result<Script, ParseError> {
    let mut seed: Option<u64> = None;
    let mut steps: Option<u64> = None;
    let mut commands = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = raw.split('#').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        let mut words = content.split_whitespace();
        let first = words.next().unwrap_or("");
        match first {
            "seed" => {
                if seed
                    .replace(number(words.next(), line, "a seed")?)
                    .is_some()
                {
                    return Err(ParseError {
                        line,
                        message: "seed given twice".into(),
                    });
                }
            }
            "steps" => {
                if steps
                    .replace(number(words.next(), line, "a step count")?)
                    .is_some()
                {
                    return Err(ParseError {
                        line,
                        message: "steps given twice".into(),
                    });
                }
            }
            _ => {
                let step: u64 = first.parse().map_err(|_| ParseError {
                    line,
                    message: format!("unknown line start '{first}'"),
                })?;
                let command = match words.next() {
                    Some("move") => {
                        expect_survivor(words.next(), line)?;
                        let x = number(words.next(), line, "a cell x")?;
                        let y = number(words.next(), line, "a cell y")?;
                        let run = match words.next() {
                            Some("walk") => false,
                            Some("run") => true,
                            _ => {
                                return Err(ParseError {
                                    line,
                                    message: "expected walk or run".into(),
                                })
                            }
                        };
                        Command::Move {
                            cell: Cell { x, y },
                            run,
                        }
                    }
                    Some("stop") => {
                        expect_survivor(words.next(), line)?;
                        Command::Stop
                    }
                    Some("wound") => {
                        expect_survivor(words.next(), line)?;
                        Command::Wound
                    }
                    Some("bandage") => {
                        expect_survivor(words.next(), line)?;
                        Command::Bandage
                    }
                    Some("pray") => {
                        expect_survivor(words.next(), line)?;
                        Command::Pray
                    }
                    Some("multiplier") => {
                        Command::Multiplier(number(words.next(), line, "a multiplier")?)
                    }
                    other => {
                        return Err(ParseError {
                            line,
                            message: format!("unknown command {other:?}"),
                        })
                    }
                };
                commands.push((step, command));
            }
        }
        if let Some(extra) = words.next() {
            return Err(ParseError {
                line,
                message: format!("unexpected extra word '{extra}'"),
            });
        }
    }
    Ok(Script {
        seed: seed.ok_or(ParseError {
            line: 0,
            message: "the file has no seed line".into(),
        })?,
        steps: steps.ok_or(ParseError {
            line: 0,
            message: "the file has no steps line".into(),
        })?,
        commands,
    })
}

fn expect_survivor(word: Option<&str>, line: usize) -> Result<(), ParseError> {
    if word == Some("survivor") {
        Ok(())
    } else {
        Err(ParseError {
            line,
            message: "expected 'survivor'".into(),
        })
    }
}

/// Runs a script from the start. `seed` and `steps` override the file's values when given.
pub fn run(script: &Script, seed: Option<u64>, steps: Option<u64>) -> Runner<Capsule> {
    let mut runner = runner(seed.unwrap_or(script.seed));
    let Some(survivor) = runner.simulation().survivor() else {
        return runner;
    };
    // Group by step once, keeping file order within a step, so each step is a lookup.
    let mut by_step: BTreeMap<u64, Vec<&Command>> = BTreeMap::new();
    for (at, command) in &script.commands {
        by_step.entry(*at).or_default().push(command);
    }
    for step in 0..steps.unwrap_or(script.steps) {
        let mut intents = Vec::new();
        for command in by_step.get(&step).into_iter().flatten() {
            match command {
                Command::Multiplier(multiplier) => runner.set_clock_multiplier(*multiplier),
                Command::Move { cell, run } => intents.push(Intent::MoveTo {
                    entity: survivor,
                    cell: *cell,
                    run: *run,
                }),
                Command::Stop => intents.push(Intent::Stop { entity: survivor }),
                Command::Wound => intents.push(Intent::Wound { entity: survivor }),
                Command::Bandage => intents.push(Intent::Bandage { entity: survivor }),
                Command::Pray => intents.push(Intent::Pray { entity: survivor }),
            }
        }
        runner.step_once(&intents);
    }
    runner
}
