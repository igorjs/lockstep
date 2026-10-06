<!-- SPDX-License-Identifier: Apache-2.0 -->

# mars-rovers

An example for [Lockstep](https://github.com/igorjs/lockstep). The Mars Rovers kata on any topology (`Square4`, `Square8` and `Hex`), with rocks, the plateau edge, and rovers that collide through occupancy.

Its fixture hash is committed in `fixtures/mars-rovers.hash` and checked natively and under WebAssembly on
every change.

## Run

From the repository root:

```sh
cargo test -p mars-rovers
wasm-pack test --node examples/mars-rovers
cargo run -p lockstep-headless -- verify mars-rovers --expect examples/mars-rovers/fixtures/mars-rovers.hash
```

See the [full description](../../docs/mars-rovers.md).
