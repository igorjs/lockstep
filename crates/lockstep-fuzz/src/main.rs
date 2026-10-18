// SPDX-License-Identifier: Apache-2.0
//! `lockstep-fuzz [--iterations <number>] [--seed <number>] [--target <name>]` fuzzes every target
//! (or one) and exits with an error and the failing input on the first failure.
//! `lockstep-fuzz write-corpus` writes the binary seeds into `corpus/`.

use lockstep_fuzz::{binary_seeds, corpus_directory, fuzz, targets};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().map(String::as_str) == Some("write-corpus") {
        for (target, name, bytes) in binary_seeds() {
            let directory = corpus_directory().join(target);
            std::fs::create_dir_all(&directory).expect("create the corpus directory");
            std::fs::write(directory.join(name), bytes).expect("write the seed");
        }
        return ExitCode::SUCCESS;
    }
    let (mut iterations, mut seed, mut only) = (100_000u64, 20_261_019u64, None);
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next();
        match (flag.as_str(), value) {
            ("--iterations", Some(value)) => iterations = value.parse().expect("a number"),
            ("--seed", Some(value)) => seed = value.parse().expect("a number"),
            ("--target", Some(value)) => only = Some(value.clone()),
            _ => {
                eprintln!("usage: lockstep-fuzz [--iterations N] [--seed N] [--target NAME]");
                return ExitCode::from(2);
            }
        }
    }
    let mut ran = 0;
    for target in targets() {
        if only.as_ref().is_some_and(|name| name != target.name) {
            continue;
        }
        ran += 1;
        match fuzz(&target, seed, iterations) {
            Ok(()) => println!("ok {} ({iterations} iterations, seed {seed})", target.name),
            Err(failure) => {
                eprintln!("{failure}");
                return ExitCode::FAILURE;
            }
        }
    }
    if ran == 0 {
        eprintln!("no target has that name");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
