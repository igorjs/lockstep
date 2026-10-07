<!-- SPDX-License-Identifier: Apache-2.0 -->

# training

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. Technicians use
`lockstep-progression` to spend points from work on a six-node web with an exclusive
specialisation, experience gates and one irreversible keystone, all in `data/training.json`.

Its fixture hash is committed in `fixtures/training.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p training
wasm-pack test --node examples/training
cargo run -p lockstep-headless -- verify training --expect examples/training/fixtures/training.hash
```

See the [full description](../../docs/training.md).
