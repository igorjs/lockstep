<!-- SPDX-License-Identifier: Apache-2.0 -->

# capsule (example)

The game shaped consumer scenario. One survivor lives in a sixteen by sixteen room.

- Intents: `MoveTo { entity, cell, run }`, `Stop { entity }`.
- Walking: one cell every six steps, three when running, along the cheapest path (A*) around a wall that crosses the room at column 8 from row 2 to row 12, with gaps above and below. A move onto the wall, outside the room, or to an unreachable cell is rejected.
- Needs: hunger, thirst, and sanity drain per game minute, so a rest at twenty times drains twenty times faster.
- Starvation: when hunger or thirst is empty, health drains. At zero the survivor dies and every column entry is removed.
- Events: `Arrived`, `Hungry`, `Starving`, `Died`, `Rejected`.

The fixture script walks, runs, sends one bad move that is rejected, then rests at twenty times until
the survivor starves. Its hash is committed in `fixtures/capsule.hash`.

## The session file

`fixtures/capsule.intents` is a plain text file, one line per item, `#` starts a comment:

```text
seed 20260925                          # required, exactly once
steps 9000                             # required, exactly once
10 move survivor 15 15 walk            # <step> move survivor <x> <y> walk|run
700 stop survivor                      # <step> stop survivor
800 multiplier 20                      # <step> multiplier <number>
```

Commands at one step apply before that step runs, in file order. Lines may not carry extra words,
and `seed` or `steps` missing or repeated is an error (a `ParseError` with line 0 means the whole
file). The Rust parser in `src/script.rs` is the reference.

`lockstep-headless` runs this file, so `just determinism` checks the same session on every platform. The headless runner takes its default seed and step count from the file.
