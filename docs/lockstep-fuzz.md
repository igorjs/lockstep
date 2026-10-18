<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-fuzz

A deterministic fuzz harness. It feeds bytes to decoders, data loaders and arithmetic and fails on
a panic or a broken invariant. It is a test tool: nothing depends on it and it is not published.
See decision 0039 for why it is not cargo-fuzz.

## Targets

| Target | What it checks |
|---|---|
| `decode_recording` | `Recording::from_bytes` and `recorded_simulation_id` never panic; a decoded recording encodes stably and replays without panicking |
| `decode_save` | A decoded save (the ledger's books) encodes stably, restores and snapshots back to itself |
| `decode_message` | Intents, events, configurations, `Money` and `Currency` decode or are refused, and encode stably |
| `json_registry`, `json_inventory_catalogue`, `json_recipes`, `json_relations`, `json_templates`, `json_knowledge`, `json_progression`, `json_regions`, `json_calendar` | Each crate's `from_json` returns a value or an error, never a panic |
| `arithmetic_fixed` | `Fixed32` laws (commutative add and multiply, inverse subtract, mirrored ratios, floor, round and square root bounds) and `parse_decimal` |
| `arithmetic_money` | `Money` laws: checked add and subtract, the parts of `allocate` and `split` add up to the amount and differ by at most one unit |

"Encodes stably" means encoding a decoded value and decoding that again gives the same bytes. A
decode may tidy its input (a column drops trailing empty slots), so the first encoding need not
equal the input.

## Running

- `cargo test -p lockstep-fuzz` (part of `just test`) runs every committed seed and 1,500
  mutations per target. `LOCKSTEP_FUZZ_ITERATIONS` changes the count.
- `just fuzz` runs a million mutations per target; `just fuzz 5000000 7` takes an iteration count
  and a seed. It uses the `fuzz` profile, which keeps overflow checks and unwinding on.
- `cargo run --profile fuzz -p lockstep-fuzz -- --target decode_recording --iterations 100000`
  runs one target.
- `just miri` runs the smoke under Miri (decision 0041).

A failure prints the target, seed, iteration and the input as hex. Turn it into a regression test
in `crates/lockstep-fuzz/tests/regressions.rs`, add the input as a seed (a function in
`src/seeds.rs`, then `cargo run --profile fuzz -p lockstep-fuzz -- write-corpus`), and fix the
bug.

## Corpus

`corpus/<target>/` holds the seeds: the committed example data files for the loaders, and encoded
recordings, saves and messages for the decoders. `write-corpus` regenerates the binary ones.

## Found so far

- A recording with an astronomical day length or a zero-length step overflowed the clock's
  64-bit units (decision 0042).
