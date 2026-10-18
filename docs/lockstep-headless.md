<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-headless

Runs a named fixture without a host and prints its state hash.

```
lockstep-headless fixture <name> [--seed <number>] [--steps <number>]
lockstep-headless verify  <name> --expect <file> [--seed <number>] [--steps <number>]
```

- `fixture` prints the hash as sixteen lowercase hexadecimal digits.
- `verify` compares the hash with a committed file, ignoring surrounding whitespace. It prints `ok <name> <hash>` on a match. On a mismatch it prints the expected and actual hashes to standard error and exits with status 1.
- Without `--seed` and `--steps`, each fixture uses its own committed defaults.

Fixtures: `capsule`, `crowd`, `drone-fleet`, `ledger`, `ledger-bills`, `tariffs`, `mars-rovers`, `sparring`, `cargo-bay`, `crater-survey`, `audit`, `training`, `dispatch`, `bakery`, `estate`, `orchard`, `checkout` and `roster`. Each committed hash lives in
`examples/<name>/fixtures/<name>.hash`. With the `parallel` feature the crowd solves its paths on a
thread pool, and its hash must not change.

A fixture hash changes only with a commit message that names the rule that changed.

## Recordings

```
lockstep-headless record <name> --out <file> [--seed <number>] [--steps <number>] [--every <number>]
lockstep-headless replay <file> [--until <step>] [--save <file>]
lockstep-headless resume <save> --recording <file>
lockstep-headless bisect <file>
lockstep-headless stats  <file>
```

- `record` writes the fixture session as a recording, with a checkpoint every 100 steps unless
  `--every` says otherwise, keeping the snapshot at each checkpoint. Every example can be recorded except `mars-rovers` (many sessions in one fixture), `roster` (two successive projects) and `orchard` (the host sets daylight between steps); see decision 0043.
- `replay` reads the simulation id at the front of the file, runs the recording again and prints
  `identical <id> after <steps> steps, hash <hash>`. A divergence prints the step and both hashes to
  standard error and exits with status 1. `--until` stops after that many steps. `--save` writes a save where the replay stopped, so
  `--until 1200 --save session.save` saves the session at step 1,200. It writes nothing when the replay diverges.
- `bisect` prints the last checkpoint that matched and the first that differs, then the lines of the
  two snapshots that differ, or says the snapshots match and only the recorded hash differs. A
  divergence exits with status 1, as with `replay`.
- `resume` reads the save, converts an older snapshot shape through the simulation's migrations, and
  runs the recording from the save's step on, comparing every checkpoint from there. It prints
  `identical <id> from step <a> to step <b>, hash <hash>`, and a divergence exits with status 1. A
  save for another simulation is refused, and so is a save converted from an older shape, which the
  recording's hashes cannot be compared with (decision 0036).
- Each command refuses options it does not use, so `--until` and `--save` are for `replay` only and
  `--recording` is for `resume` only.
- `stats` prints the seed, the length in steps and real seconds, intents per real minute, the steps
  spent at each clock multiplier, and the number of checkpoints and snapshots.

`just determinism` records a ledger session and a bakery session and replays them through the binary, and records a
`tariffs` session, saves it at step 1,200 and resumes it.
