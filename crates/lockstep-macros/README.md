<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-macros

The derive macros for [Lockstep](https://github.com/igorjs/lockstep). Use them through
`lockstep-core`, which re-exports them; this crate is not meant to be a direct dependency.

`#[derive(Message)]` with `#[message(version = N)]` implements two traits:

- `Message`, with `VERSION = N`. The attribute is required: a shipped shape is never edited, so every
  change is a new version.
- `Indexable`, for the timeline. `kind` is an enum's variant index (0 for a struct), and `handles`
  pushes every `Handle` a field holds, directly or inside `Option`, `Vec`, `Box`, an array, a slice
  or a tuple, in field order. A type whose handles live elsewhere (a map) writes
  `#[message(version = N, manual_indexable)]` and implements `Indexable` itself.

```rust
use lockstep_core::{Handle, Indexable, Message};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Deposited { account: Handle, amount_minor: i64 },
    Transferred { from: Handle, to: Handle, amount_minor: i64 },
}
```

A generic message is a message only when its whole shape can be saved; the derive states that bound
for you.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
