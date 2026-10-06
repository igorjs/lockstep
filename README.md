<!-- SPDX-License-Identifier: Apache-2.0 -->

# Lockstep

A deterministic, replayable simulation framework in Rust.

A simulation is a state machine that advances in fixed steps from intents. The same configuration,
seed and intents produce the same state, and the same state hash, on every platform: Linux, macOS,
Windows and WebAssembly. That one guarantee gives you replays, save files that load exactly, bug
reports you can reproduce, and lockstep multiplayer.

The core never renders, never reads input, does no input or output, and never knows what a game is.
Games are the first use, but the same machine runs a ledger, a fleet of rovers, or a crowd.

## Crates

| Crate | What it is |
|---|---|
| [`lockstep-core`](crates/lockstep-core) | The `Simulation` trait, the `Runner`, the game clock, entity storage, named random streams, chance rolls, hashing, and integer math. |
| [`lockstep-spatial`](crates/lockstep-spatial) | Grids: topologies, maps with elevation, occupancy, A* paths, path batches on a thread pool, flow fields, line of sight. |
| [`lockstep-attributes`](crates/lockstep-attributes) | Stats as data: a registry read from JSON, modifiers, thresholds that fire once per crossing, and derived values from curves. Timed effects are in progress. |
| [`lockstep-headless`](crates/lockstep-headless) | A command line runner that prints and verifies fixture hashes. |

The crates are not on crates.io yet. Depend on them from Git:

```toml
[dependencies]
lockstep-core = { git = "https://github.com/igorjs/lockstep" }
```

## Examples

Each example is a consumer scenario with a committed fixture hash, checked natively and under
WebAssembly on every change.

| Example | Shows |
|---|---|
| [`capsule`](examples/capsule) | A body with needs walks a walled room along A* paths and starves if left alone. |
| [`ledger`](examples/ledger) | Accounts and transfers: exact money, rejected transfers with reasons, nothing game specific. |
| [`mars-rovers`](examples/mars-rovers) | The Mars Rovers kata on any topology, with collisions as occupancy. |
| [`crowd`](examples/crowd) | Sixty bodies route to one goal, solving each step's paths as one batch, with the same hash on one thread or many. |

## The rules

Everything in a simulation obeys these, and `scripts/lint-determinism.sh` enforces the first:

- No hash maps, wall clocks, threads, or transcendental float functions. Use ordered maps and
  `lockstep_core::math`. The one exception is the path batch in `lockstep-spatial` (decision 0006).
- Entities are handles into a `StableVector`; components are `Column`s keyed by those handles.
- Randomness comes only from named streams, so adding a stream never changes another.
- A fixture hash changes only with a commit message that names the rule that changed.

## Development

You need stable Rust with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target,
[`just`](https://github.com/casey/just), and [`wasm-pack`](https://github.com/rustwasm/wasm-pack)
with Node.

```sh
just test          # every test, including the hex and parallel features
just check         # format, clippy, the determinism lint, and the SPDX check
just determinism   # verify every fixture hash, then run the tests under WebAssembly
just ci            # all of the above
just bench         # the spatial benchmark against benches/baseline.txt
```

Continuous integration runs `just ci` on Linux for pull requests and on Linux, macOS and Windows for
`main`.

## Documentation

Each crate and example has a page in [`docs`](docs), and every behavioural decision has a record in
[`docs/decisions`](docs/decisions) and a test in `tests/decisions` that fails if the decision stops
holding.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
