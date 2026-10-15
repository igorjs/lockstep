<!-- SPDX-License-Identifier: Apache-2.0 -->

# estate

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A garden estate
is rolled from chunk templates with `lockstep-procgen`, and a flood barrier is raised across it
and worn down by surges with `lockstep-structures`.

Its fixture hash is committed in `fixtures/estate.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p estate
wasm-pack test --node examples/estate
cargo run -p lockstep-headless -- verify estate --expect examples/estate/fixtures/estate.hash
```

See the [full description](../../docs/estate.md).
