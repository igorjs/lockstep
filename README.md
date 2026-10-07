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
| [`lockstep-combat`](crates/lockstep-combat) | Real-time combat in integers: damage in a fixed order, hit shapes on any topology, knockback, actions and dodges with commitment, projectiles, and movement with stamina and noise. |
| [`lockstep-inventory`](crates/lockstep-inventory) | Items as entities, containers with slots and weight, stacking, equipment whose modifiers reach the wearer's attributes, binding affixes, and spoilage by temperature that ignores the step rate. |
| [`lockstep-agents`](crates/lockstep-agents) | Local awareness: sight cones gated by line of sight, staggered checks, noise carried by the wind with exact ranges, memory that fades, alert states, a director that rations Alert, steering through occupancy that never lets two bodies share a cell, and integer utility decisions with delegation. |
| [`lockstep-knowledge`](crates/lockstep-knowledge) | Facts learned from fragments of distinct sources, questions that open after enough evidence, and revelations, secrets and achievements as data predicates that fire once. |
| [`lockstep-progression`](crates/lockstep-progression) | Node graphs bought with points from named triggers: requirements, exclusions that lock both ways, attribute gates, irreversible keystones, refunds and modifiers. |
| [`lockstep-attributes`](crates/lockstep-attributes) | Stats as data: a registry read from JSON, modifiers, thresholds that fire once per crossing, derived values from curves, and timed effects with stacking rules. |
| [`lockstep-macros`](crates/lockstep-macros) | `#[derive(Message)]`, re-exported by `lockstep-core`. |
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
| [`capsule`](examples/capsule) | A survivor walks a walled room along A* paths; needs, health and sanity are attributes from JSON, and decay, bleeding and prayer are effects. |
| [`ledger`](examples/ledger) | Accounts and transfers: exact money, rejected transfers with reasons. Not a game. |
| [`mars-rovers`](examples/mars-rovers) | The Mars Rovers kata on any topology, with collisions as occupancy and rovers that ram each other. |
| [`drone-fleet`](examples/drone-fleet) | Delivery drones whose batteries charge, drain and wear: attributes from JSON, effects, a ratchet, and fault rolls. Not a game. |
| [`cargo-bay`](examples/cargo-bay) | A crew works a cargo bay: deliveries into stores with slots and weight limits, rations that spoil by temperature (and faster after a power cut), suits that raise oxygen, and faulty seals that bind until repaired. Not a game. |
| [`crater-survey`](examples/crater-survey) | Survey rovers hear landers through the wind, investigate when it is worth it, stay within their crater when leashed, and accept or refuse survey requests with a reason. Not a game. |
| [`audit`](examples/audit) | An auditor reviews transfers as they happen and gathers fragments (round amounts, small-hours transfers, shared receivers) until questions open and findings fire, once, from data. Not a game. |
| [`training`](examples/training) | Technicians earn points at work and study a six-node web: an exclusive specialisation, gates on experience, and an irreversible chief keystone. Not a game. |
| [`sparring`](examples/sparring) | Partners spar in pairs: strikes, sweeps that knock back, dodges, thrown balls, walking and running on one stamina pool, and a 200-partner spike. |
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

[`docs/roadmap.md`](docs/roadmap.md) shows each milestone's status and every gap, deferral and
departure from the reference.

Each crate and example has a page in [`docs`](docs), and every behavioural decision has a record in
[`docs/decisions`](docs/decisions) and a test in `tests/decisions` that fails if the decision stops
holding.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
