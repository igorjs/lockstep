<!-- SPDX-License-Identifier: Apache-2.0 -->

# franchise

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A franchise
hires shift staff from archetypes with point-buy, hands the manager role down a family line when
the manager departs, and burns the name of a line that is lost. The chronicle, saved in a versioned
envelope, outlives each season.

Its fixture hash is committed in `fixtures/franchise.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p franchise
wasm-pack test --node examples/franchise
cargo run -p lockstep-headless -- verify franchise --expect examples/franchise/fixtures/franchise.hash
```

See the [full description](../../docs/franchise.md).
