<!-- SPDX-License-Identifier: Apache-2.0 -->

# cargo-bay

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A crew works a
cargo bay with `lockstep-inventory`: deliveries into stores with slots and weight limits, rations
that spoil by temperature, suits that raise oxygen, and faulty seals that bind until repaired.

Its fixture hash is committed in `fixtures/cargo-bay.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p cargo-bay
wasm-pack test --node examples/cargo-bay
cargo run -p lockstep-headless -- verify cargo-bay --expect examples/cargo-bay/fixtures/cargo-bay.hash
```

See the [full description](../../docs/cargo-bay.md).
