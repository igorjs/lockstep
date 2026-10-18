<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-meta

What outlives a single run of a [Lockstep](https://github.com/igorjs/lockstep) simulation: a
record of what happened across runs, the hand-over of control when the one in control is lost, and
archetypes bought with points.

- `Chronicle`: the cross-run record. It holds the lines that were lost, with their names retired,
  and named tallies. It is not part of any simulation's state; the host folds a run's events into
  it and carries it into the next run.
- `seal` and `open`: save any `Message` in a versioned envelope (magic, envelope version, message
  version, checksum) and read it back, refusing anything damaged or from another version.
- `Control`: who is in control now, and the fall-back order when they are lost.
- `Templates`: archetypes and a point-buy budget, read from JSON and checked.

## Example

```rust
use lockstep_meta::{open, seal, Chronicle};

let mut chronicle = Chronicle::new();
chronicle.begin_run();
chronicle.retire("Okafor", 1, 3);
let bytes = seal(&chronicle);
let again: Chronicle = open(&bytes).unwrap();
assert!(again.is_retired("Okafor"));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-meta.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
