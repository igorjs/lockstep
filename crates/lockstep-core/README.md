<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-core

The core of [Lockstep](https://github.com/igorjs/lockstep): a deterministic state machine advanced in
fixed steps from intents. The same configuration, seed and intents produce the same state and the
same hash natively and under WebAssembly.

- `Simulation`: the one trait you implement (`create`, `step`, `snapshot`, `restore`).
- `Runner`: owns time. Fixed steps at 30 per second by default, an accumulator with a cap, the game
  clock, the random streams, and the intent queue.
- `Clock`: game time as an exact integer position, with sunrise, sunset and new day events.
- `StableVector`, `Column`, `Handle`: entities as generational handles, components as columns.
- `Streams`: named random streams. `Chance` and the rolls `roll`, `roll_with_luck` and `roll_smoothed`.
- `hash_of`: xxh3 over fixed-width, little-endian bytes.
- `math`: `Fixed32` (16.16 fixed point), `Vector2`, and angles as whole turns, all integer.

## Example

A counter that adds what it is told, and a rare bonus from a named stream:

```rust
use lockstep_core::{
    Chance, ClockConfiguration, Context, Message, Runner, Simulation, StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Counter {
    total: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Add(i64);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Bonus;

impl Simulation for Counter {
    type Intent = Add;
    type Event = Bonus;
    type Snapshot = Counter;
    type Configuration = Counter;

    fn create(configuration: Counter, _randomness: &mut Streams) -> Self {
        configuration
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Add]) {
        for Add(amount) in intents {
            self.total += amount;
        }
        if context.randomness.roll("bonus", Chance::percent(5)) {
            self.total += 100;
            context.events.push(Bonus);
        }
    }

    fn snapshot(&self) -> Counter {
        self.clone()
    }

    fn restore(snapshot: Counter) -> Self {
        snapshot
    }
}

let clock = ClockConfiguration {
    day_length_real_minutes: 24.0,
    sunrise_minute: 360,
    sunset_minute: 1_200,
    starting_minute: 480,
    starting_day: 0,
};
let run = || {
    let mut runner = Runner::<Counter>::new(Counter { total: 0 }, 42, StepConfiguration::default(), clock);
    for step in 0..300 {
        runner.step_once(&[Add(step)]);
    }
    (runner.simulation().total, runner.hash())
};
// Same seed, same intents: same state and same hash, every time and on every platform.
assert_eq!(run(), run());
```

Hosts call `runner.advance(real_seconds)` once per frame instead, after queueing input with
`runner.queue(intents)`; replay and turn based callers use `step_once`.

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-core.md)
and the design decisions in
[`docs/decisions`](https://github.com/igorjs/lockstep/tree/main/docs/decisions).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
