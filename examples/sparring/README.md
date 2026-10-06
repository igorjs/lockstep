<!-- SPDX-License-Identifier: Apache-2.0 -->

# sparring

An example for [Lockstep](https://github.com/igorjs/lockstep). Partners spar in pairs with
`lockstep-combat`: strikes, sweeps that knock back, dodges, thrown balls, and walking and running
on one stamina pool. A spike has 200 partners spar for 10,000 steps.

Its fixture hash is committed in `fixtures/sparring.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p sparring
cargo test --release -p sparring --test spikes
wasm-pack test --node examples/sparring
cargo run -p lockstep-headless -- verify sparring --expect examples/sparring/fixtures/sparring.hash
```

See the [full description](../../docs/sparring.md).
