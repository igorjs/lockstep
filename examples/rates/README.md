<!-- SPDX-License-Identifier: Apache-2.0 -->

# rates

An example for [Lockstep](https://github.com/igorjs/lockstep). A parcel desk that quotes prices from a rate table while the table is swapped, including a broken table that is refused. The consumer scenario for data tables with hot reload. Not a game.

Its fixture hash is committed in `fixtures/rates.hash` and checked natively and under WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p rates
wasm-pack test --node examples/rates
cargo run -p lockstep-headless -- verify rates --expect examples/rates/fixtures/rates.hash
```

See the [full description](../../docs/rates.md).
