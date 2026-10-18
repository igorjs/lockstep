// SPDX-License-Identifier: Apache-2.0
//! A deterministic fuzz harness. Each target takes arbitrary bytes and must never panic or break
//! an invariant. The inputs are the committed corpus (`corpus/<target>/`) plus mutations of it
//! drawn from a seeded generator, so a run is the same on every machine and a failure names the
//! seed, the iteration and the exact bytes. See docs/decisions/0039-fuzzing-without-libfuzzer.md.

mod mutation;
mod seeds;
mod targets;

pub use mutation::{mutate, SplitMix};
pub use seeds::binary_seeds;
pub use targets::{targets, Target};

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// The directory holding the committed seeds, one subdirectory per target.
pub fn corpus_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

/// Every file of a target's corpus, in name order, so the run order is stable.
pub fn read_corpus(target: &Target) -> Vec<Vec<u8>> {
    let directory = corpus_directory().join(target.name);
    let mut names: Vec<PathBuf> = std::fs::read_dir(&directory)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
        .into_iter()
        .filter_map(|path| std::fs::read(path).ok())
        .collect()
}

/// A failure the harness found: the target, the input that broke it and what it said.
#[derive(Debug)]
pub struct Failure {
    pub target: &'static str,
    pub seed: u64,
    pub iteration: u64,
    pub input: Vec<u8>,
    pub message: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: String = self
            .input
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        write!(
            formatter,
            "target {} failed at seed {} iteration {}: {}\ninput ({} bytes): {hex}",
            self.target,
            self.seed,
            self.iteration,
            self.message,
            self.input.len()
        )
    }
}

fn run_guarded(target: &Target, input: &[u8]) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(|| (target.run)(input))).map_err(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_else(|| "a panic".to_string())
    })
}

/// Runs the corpus unchanged, then `iterations` mutations of it. Stops at the first failure.
pub fn fuzz(target: &Target, seed: u64, iterations: u64) -> Result<(), Failure> {
    let corpus = read_corpus(target);
    let fail = |iteration, input: &[u8], message| Failure {
        target: target.name,
        seed,
        iteration,
        input: input.to_vec(),
        message,
    };
    for entry in &corpus {
        run_guarded(target, entry).map_err(|message| fail(0, entry, message))?;
    }
    let mut random = SplitMix::new(seed ^ fingerprint(target.name));
    let empty = Vec::new();
    for iteration in 1..=iterations {
        let base = if corpus.is_empty() {
            &empty
        } else {
            &corpus[random.below(corpus.len() as u64) as usize]
        };
        let other = if corpus.is_empty() {
            &empty
        } else {
            &corpus[random.below(corpus.len() as u64) as usize]
        };
        let input = mutate(base, other, &mut random);
        run_guarded(target, &input).map_err(|message| fail(iteration, &input, message))?;
    }
    Ok(())
}

/// A stable number for a name (FNV-1a), so each target gets its own mutation sequence.
fn fingerprint(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3)
    })
}
