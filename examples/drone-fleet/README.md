<!-- SPDX-License-Identifier: Apache-2.0 -->

# drone-fleet

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. Delivery drones fly
from a depot; each battery's charge, wear and capacity are attributes read from JSON, flights and
charging are effects, faults are `Chance` rolls, and wear marks are a ratchet that service cannot
undo.

Its fixture hash is committed in `fixtures/drone-fleet.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p drone-fleet
wasm-pack test --node examples/drone-fleet
cargo run -p lockstep-headless -- verify drone-fleet --expect examples/drone-fleet/fixtures/drone-fleet.hash
```

See the [full description](../../docs/drone-fleet.md).
