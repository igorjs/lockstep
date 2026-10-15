<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchard

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A blight uses
`lockstep-regions` to spread across orchard blocks before the wind, and `lockstep-calendar` to
idle in the cold as winter shortens the days.

Its fixture hash is committed in `fixtures/orchard.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p orchard
wasm-pack test --node examples/orchard
cargo run -p lockstep-headless -- verify orchard --expect examples/orchard/fixtures/orchard.hash
```

See the [full description](../../docs/orchard.md).
