<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0071 Neutral names, and renames that move hashes (decided)

Status: decided and implemented; closes departure P28.

## Decision

Names in code, data, tests and docs are neutral engine terms. When a name feeds a fixture hash,
such as a random stream name or the text of a recorded script, the name is still changed, the
affected `.hash` files and their WebAssembly twins are regenerated, and the commit message names the
rule that changed (for example: random stream renamed).

The renames:

- Random stream names `"loot"` and `"valley"` became `"draws"` and `"terrain"`; the capsule stream
  `"wounds"` became `"injuries"`.
- The `game_minutes` identifiers, including the `Context` field `elapsed_game_minutes`, became
  `simulated_minutes` and `elapsed_simulated_minutes`, with their decision tests and records.
- The capsule intents `Wound`, `Bandage` and `Pray` became `Injure`, `Treat` and `Restore`, the
  events `Bandaged` and `Prayed` became `Treated` and `Restored`, the script words follow, and the
  actor name became `Operator`.

The fixture hashes of `capsule` and `estate` changed because of the stream names and the actor name;
the other examples keep theirs.

## Alternatives rejected

- Keeping the old names to protect hashes: a hash is a record of one version of the rules, and the
  rule here is the vocabulary. Keeping the names left the engine tied to one kind of consumer.
- Aliases for the old names: they keep the old words in the code and change nothing for a replay.

## Decision test

`engine_names_stay_neutral` scans the crates, examples, scripts and the README for the old
vocabulary and fails on any hit. It would change if a game-flavoured word came back.
