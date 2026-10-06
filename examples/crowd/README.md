<!-- SPDX-License-Identifier: Apache-2.0 -->

# crowd

An example for [Lockstep](https://github.com/igorjs/lockstep). Sixty bodies route to one goal around random walls. Each step solves every body's path in one `find_paths` batch, and the hash is the same whether the batch runs on one thread or many.

Its fixture hash is committed in `fixtures/crowd.hash` and checked natively and under WebAssembly on
every change.

## Run

From the repository root:

```sh
cargo test -p crowd
wasm-pack test --node examples/crowd
cargo run -p lockstep-headless -- verify crowd --expect examples/crowd/fixtures/crowd.hash
cargo run -p lockstep-headless --features parallel -- verify crowd --expect examples/crowd/fixtures/crowd.hash
```

See the [full description](../../docs/crowd.md).
