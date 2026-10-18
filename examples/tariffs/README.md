<!-- SPDX-License-Identifier: Apache-2.0 -->

# tariffs

An example for [Lockstep](https://github.com/igorjs/lockstep). Accounts and transfers that charge a fee, and a save written before the fee existed that still loads. The consumer scenario for saves and migrations. Not a game.

Its fixture hash is committed in `fixtures/tariffs.hash`, and the old save and the hash of its
continuation in `fixtures/legacy-v1.bin` and `fixtures/legacy-v1.hash`. All are checked natively and
under WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p tariffs
wasm-pack test --node examples/tariffs
cargo run -p lockstep-headless -- verify tariffs --expect examples/tariffs/fixtures/tariffs.hash
```

See the [full description](../../docs/tariffs.md).
