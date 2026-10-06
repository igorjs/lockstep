<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-core

A deterministic state machine advanced in fixed steps from intents. The same configuration, seed,
and intents produce the same state and the same hash on every platform.

## Public surface

| Item | Purpose |
| --- | --- |
| `Simulation` | The one trait a game or model implements: `create`, `step`, `snapshot`, `restore`. |
| `Context` | Everything a simulation may touch during one step: clock, elapsed game minutes (as a float and as exact `Fixed32` in `elapsed_minutes`), streams, events. The fixed point minutes of any number of steps add up exactly, so state that runs on game minutes never drifts. |
| `Runner` | Owns time. Fixed steps, an accumulator, the clock, the streams, and the intent queue. |
| `StepConfiguration` | Step length (default one thirtieth of a second) and the cap on steps per advance (default eight). |
| `Clock`, `ClockConfiguration`, `ClockEvent` | The game clock: a rate with a multiplier, sunrise, sunset, new day. Time of day is an exact integer position, so a day is an exact number of steps. |
| `StableVector`, `Column`, `Handle` | Entities as generational handles; components as columns keyed by those handles. Inserts reuse the lowest free slot, and a restored store continues exactly as the original. The handle `Handle::from_raw(0)` never refers to anything. |
| `Streams` | Named random streams, each seeded from the master seed and its name. |
| `Chance`, `SmoothedState`, `CERTAIN` | Percentages as basis points, and the three rolls on `Streams`: `roll`, `roll_with_luck`, `roll_smoothed`. See decision 0007. |
| `hash_of` | xxh3 over fixed-width, little-endian bincode bytes. |
| `math` | Deterministic math: `Fixed32` (16.16 fixed point), `isqrt`, `Vector2`, and angles as whole turns (`sin`, `cos`, `unit`, `atan2`, `unit_circle_table`). Integer only. See decision 0004. |
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

## Chance and rolls

A percentage crosses the roll boundary as basis points: `Chance(2_500)` is 25 percent, and
`Chance::percent(25)` builds the same value. Every roll compares integer draws.

- `roll(name, chance)`: honest and streaky. Use it for loot.
- `roll_with_luck(name, chance, luck)`: rolls `1 + |luck|` times and keeps the best draw for positive
  luck, the worst for negative. Luck 3 at 25 percent lands about 68 percent. Show players the nominal
  chance, never this one.
- `roll_smoothed(name, chance, state)`: a pseudo-random distribution. Attempt n after a success
  succeeds with probability n times an increment, so droughts are short and the long-run rate equals
  the nominal chance. Keep one `SmoothedState` per source and save it with the entity. Use it for hits
  and criticals, never for farmable rolls, and never for a roll that affects another player in
  multiplayer.

The increment for every basis point is in `fixtures/smoothing.bin`, found once by a search
(`cargo run --release -p lockstep-core --example generate_smoothing`). A decision test runs the search
again and compares every entry. See decision 0007.

## Rules the core obeys

No hash maps or sets, no wall clocks, no threads, no transcendental float functions. The lint in
`scripts/lint-determinism.sh` fails the build on them. Maps are ordered maps.

## Tests

- `tests/behaviour`: runner, intent queue, clock, store, streams, chance.
- `tests/decisions`: one file per design decision, with the rejected alternative and the number that would change it.
- `tests/spikes`: experiments that check a design before other code builds on it. The restore spike found a defect that became decision 0002.
- Every test also runs under WebAssembly in Node (`wasm-pack test --node crates/lockstep`).
- `fixtures/million_rolls.hash`: the hash of one million rolls, asserted natively and under WebAssembly.
- `fixtures/chance_rolls.hash`: the hash of 300,000 plain, lucky and smoothed rolls, asserted the same way.

## Not in milestone M1

Money, grid, attributes, chance rolls (milestone M4), saves and migrations, record and replay, the
timeline, the derive macro for `Message`. See the roadmap.
