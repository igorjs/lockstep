<!-- SPDX-License-Identifier: Apache-2.0 -->

# capsule

An example for [Lockstep](https://github.com/igorjs/lockstep). One actor in a sixteen by sixteen room walks along A* paths around a wall. Health, hunger, thirst and sanity are attributes read from JSON, their decay is a set of effects, injuries bleed by a chance roll for half an hour or until treated, and restoration restores sanity up to a daily budget. Left alone, the actor starves. The single-actor consumer scenario.

Its fixture hash is committed in `fixtures/capsule.hash` and checked natively and under WebAssembly on
every change.

## Run

From the repository root:

```sh
cargo test -p capsule
wasm-pack test --node examples/capsule
cargo run -p lockstep-headless -- verify capsule --expect examples/capsule/fixtures/capsule.hash
```

See the [full description](../../docs/capsule.md).
