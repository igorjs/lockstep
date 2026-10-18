<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-fuzz

A deterministic fuzz harness for [Lockstep](https://github.com/igorjs/lockstep): decoders, data
loaders and arithmetic, with a committed corpus. Not published.

```sh
cargo test -p lockstep-fuzz   # the bounded run
just fuzz                     # the long run
```

See the [full description](../../docs/lockstep-fuzz.md).
