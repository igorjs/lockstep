<!-- SPDX-License-Identifier: Apache-2.0 -->

# crater-survey

An example for [Lockstep](https://github.com/igorjs/lockstep) that is not a game. Survey rovers
use `lockstep-agents`: they hear landers through the wind, investigate when it is worth it, stay
within their crater when leashed, and accept or refuse survey requests with a reason.

Its fixture hash is committed in `fixtures/crater-survey.hash` and checked natively and under
WebAssembly on every change.

## Run

From the repository root:

```sh
cargo test -p crater-survey
wasm-pack test --node examples/crater-survey
cargo run -p lockstep-headless -- verify crater-survey --expect examples/crater-survey/fixtures/crater-survey.hash
```

See the [full description](../../docs/crater-survey.md).
