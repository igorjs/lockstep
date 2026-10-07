<!-- SPDX-License-Identifier: Apache-2.0 -->

# bakery

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A mixer and an
oven use `lockstep-crafting` to turn deliveries into bread through recipes in
`data/recipes.json` that consume all or nothing, with failures from an outcome table.

Its fixture hash is committed in `fixtures/bakery.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p bakery
wasm-pack test --node examples/bakery
cargo run -p lockstep-headless -- verify bakery --expect examples/bakery/fixtures/bakery.hash
```

See the [full description](../../docs/bakery.md).
