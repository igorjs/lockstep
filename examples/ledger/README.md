<!-- SPDX-License-Identifier: Apache-2.0 -->

# ledger

An example for [Lockstep](https://github.com/igorjs/lockstep). Accounts and transfers in whole minor units. A transfer moves exactly the amount or is rejected with a reason, and money is only created by deposits. The consumer scenario that is not a game.

Its fixture hash is committed in `fixtures/ledger.hash` and checked natively and under WebAssembly on
every change.

## Run

From the repository root:

```sh
cargo test -p ledger
wasm-pack test --node examples/ledger
cargo run -p lockstep-headless -- verify ledger --expect examples/ledger/fixtures/ledger.hash
```

See the [full description](../../docs/ledger.md).
