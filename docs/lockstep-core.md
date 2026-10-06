# lockstep-core

A deterministic state machine advanced in fixed steps from intents. The same configuration, seed,
and intents produce the same state and the same hash on every platform.

## Public surface

| Item | Purpose |
| --- | --- |
| `Simulation` | The one trait a game or model implements: `create`, `step`, `snapshot`, `restore`. |
| `Context` | Everything a simulation may touch during one step: clock, elapsed game minutes, streams, events. |
| `Runner` | Owns time. Fixed steps, an accumulator, the clock, the streams, and the intent queue. |
| `StepConfiguration` | Step length (default one thirtieth of a second) and the cap on steps per advance (default eight). |
| `Clock`, `ClockConfiguration`, `ClockEvent` | The game clock: a rate with a multiplier, sunrise, sunset, new day. Time of day is an exact integer position, so a day is an exact number of steps. |
| `StableVector`, `Column`, `Handle` | Entities as generational handles; components as columns keyed by those handles. Inserts reuse the lowest free slot, and a restored store continues exactly as the original. The handle `Handle::from_raw(0)` never refers to anything. |
| `Streams` | Named random streams, each seeded from the master seed and its name. |
| `hash_of` | xxh3 over fixed-width, little-endian bincode bytes. |
| `Message` | Marker for anything that crosses the boundary or lands in a save. Written by hand in milestone M1. |

## Driving a simulation

```rust
let mut runner = Runner::<MySimulation>::new(configuration, seed, StepConfiguration::default(), clock);
runner.queue(intents_from_the_host);     // applies to the next step that runs
let advanced = runner.advance(frame_seconds);   // zero or more fixed steps
// advanced.events, advanced.clock_events go to the host
let hash = runner.hash();
```

`advance` is for hosts that draw frames. `step_once` runs exactly one step with the intents you
give it and ignores the accumulator and the queue; replay and turn based callers use it.
After a hitch longer than the cap, the extra time is dropped, never owed.

## Rules the core obeys

No hash maps or sets, no wall clocks, no threads, no transcendental float functions. The lint in
`scripts/lint-determinism.sh` fails the build on them. Maps are ordered maps.

## Tests

- `tests/behaviour`: runner, intent queue, clock, store, streams.
- `tests/decisions`: one file per design decision, with the rejected alternative and the number that would change it.
- `tests/spikes`: experiments that check a design before other code builds on it. The restore spike found a defect that became decision 0002.
- Every test also runs under WebAssembly in Node (`wasm-pack test --node crates/lockstep`).
- `fixtures/million_rolls.hash`: the hash of one million rolls, asserted natively and under WebAssembly.

## Not in milestone M1

Fixed point math, grid, attributes, chance rolls, saves and migrations, record and replay, the
timeline, the derive macro for `Message`. See the roadmap.
