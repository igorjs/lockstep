<!-- SPDX-License-Identifier: Apache-2.0 -->

# audit

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. An auditor uses
`lockstep-knowledge` to gather fragments from transfers until questions open and findings fire,
once, from data in `data/knowledge.json`.

Its fixture hash is committed in `fixtures/audit.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p audit
wasm-pack test --node examples/audit
cargo run -p lockstep-headless -- verify audit --expect examples/audit/fixtures/audit.hash
```

See the [full description](../../docs/audit.md).
