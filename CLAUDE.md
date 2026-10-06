# Lockstep: deterministic, replayable simulation framework

A state machine: Simulation::step advances state in fixed steps from intents.
Same configuration, seed, and intents produce the same state on every platform.
Core never renders, never does input or output, never knows what a game is.

## Rules
- No HashMap, clocks, threads, or transcendental floats in simulation code;
  scripts/lint-determinism.sh enforces it. Use lockstep_core::math.
- Entities are Handles into StableVector; components are Columns; no references
  between entities; group fields by access pattern.
- Every feature ships with a test that fails without it, a decision test when it
  embodies a choice, a WebAssembly twin of any fixture hash, a consumer scenario,
  and a docs page.
- No crate without a consumer scenario in the same milestone.
- No abbreviations in identifiers, comments, or docs.
- A failing decision test is a design question: stop and report.
- A fixture hash changes only with a commit message naming the rule that changed.

## Loop
- just test · just check · just determinism · just ci before every commit
