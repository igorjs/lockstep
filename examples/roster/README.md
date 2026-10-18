<!-- SPDX-License-Identifier: Apache-2.0 -->

# roster

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. A crew is staffed
for one project after another with `lockstep-meta`: a retired name is never hired again, even in a
later project, and when the lead leaves the lead falls back to the next hired.

Its fixture hash is committed in `fixtures/roster.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p roster
wasm-pack test --node examples/roster
cargo run -p lockstep-headless -- verify roster --expect examples/roster/fixtures/roster.hash
```

See the [full description](../../docs/roster.md).
