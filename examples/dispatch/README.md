<!-- SPDX-License-Identifier: Apache-2.0 -->

# dispatch

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. Drivers use
`lockstep-relations` and the delegation from `lockstep-agents` to accept, delay or refuse routes by
their trust in the dispatcher against their fatigue, and say why.

Its fixture hash is committed in `fixtures/dispatch.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p dispatch
wasm-pack test --node examples/dispatch
cargo run -p lockstep-headless -- verify dispatch --expect examples/dispatch/fixtures/dispatch.hash
```

See the [full description](../../docs/dispatch.md).
