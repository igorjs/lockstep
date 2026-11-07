<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0084 A saved chronicle converts forward through a chain (decided)

Status: decided and implemented. It closes deferral D7.

## Decision

- The chronicle is now message version 2: it gains `notes` (named text, set with `set_note`, read with `note`). Version 1 shipped without them.
- The version 1 shape stays in the code as `ChronicleV1`, frozen, and `chronicle_migrations()` holds the one conversion from it to the current shape. This is the same frozen-old-type plus `Migrations` pattern as snapshots in saves (decision 0035).
- `open_with(bytes, &migrations)` reads an envelope sealed at an older version and carries it up the chain. `open` stays strict, so a caller that wants no conversion still gets a refusal.
- The checksum is checked before any conversion runs, so a damaged old chronicle is `Corrupt`.
- A version newer than the code, or older with no conversion starting at it, gives `MessageVersion { found, expected }`, which names both versions. A conversion that cannot read its bytes gives `Decode`.
- The retired names, the run count and the tallies survive a conversion unchanged.

## Alternatives rejected

- `open` converting silently: a caller could not tell a current save from an old one, and could not refuse one.
- Refusing every old chronicle: a host would lose the retired names it kept, which must stay retired for good (decision 0031).
- A new error for a missing conversion: the useful fact is the pair of versions, which `MessageVersion` already carries.

## Would change if

A newer version opens, a refusal omits either version, or a conversion drops a retired name.

## Tests

`crates/lockstep-meta/tests/decisions/an_old_chronicle_converts_and_a_newer_one_is_refused.rs`, `tests/behaviour/envelope.rs`, and the roster fixture `examples/roster/fixtures/legacy-chronicle-v1.bin`, which converts to the hash in `legacy-chronicle-v1.hash` natively and under `wasm-pack test --node examples/roster`.
