<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0085 A recording names its shapes and converts forward (decided)

Status: decided and implemented. It closes deferral D8.

## Decision

- `Recording::to_versioned_bytes(versions)` writes a 16 byte header, then the legacy bytes unchanged: the magic `LKRC`, then the versions of the intent, event and configuration shapes as little-endian 32-bit numbers.
- `to_bytes` and every committed recording stay byte for byte as they were. A reader takes both forms: bytes without the magic are a legacy recording and read as version 0 in each field.
- The magic read as a little-endian length is above `DECODE_LIMIT`, so it can never be the length prefix of a legacy recording. `recorded_simulation_id` skips the header when it is there, so the first-field rule holds in both forms. A layout test pins the 16 bytes, and the decode limit counts the total with the header.
- `from_bytes_for::<S>` refuses a header that names a newer version, or an older intent or configuration version, naming both versions. Version 0 is taken as current, since a legacy recording never said otherwise.
- Old intents and configurations convert through the `Migrations` pattern of decision 0035, with the old types frozen in the code. `Recording::convert` applies the intent chain to every recorded intent and the configuration chain to the configuration bytes.
- A converted recording keeps no checkpoints or snapshots. They hold hashes and states of the old shapes, which no longer equal what the current code produces, so comparing them would report a divergence that is not one. A consumer checks the converted replay against a committed final hash. A save converted from an old snapshot shape stays `NotComparable` (decision 0036).
- `lockstep-headless record` and the example fixtures write the versioned form.
- `RecordingSet` stays unversioned. Its sessions are read by code that knows the variant each one belongs to (a topology, a project), so one header on the set would name no single intent or configuration shape. A set that needs conversion converts each of its sessions, which are recordings with headers of their own if written that way.
- Events are not stored in a recording, so an older event version needs no conversion. A newer one is refused.

## Alternatives rejected

- `to_bytes` writing the header: every committed recording and fixture hash would change for no gain over reading both forms.
- A version field inside `Recording`: it would change the legacy bytes and the first-field layout.
- Keeping the old checkpoints on a converted recording: a changed shape changes the hash, so the first compare would fail.
- Versioning `RecordingSet` now: nothing needs it yet, and the right header is unclear until a set with a changed shape exists.

## Would change if

`to_bytes` output changes, a legacy recording stops reading, a version mismatch omits either version, or a converted recording compares a hash recorded under an old shape.

## Tests

`crates/lockstep-core/tests/decisions/a_recording_says_which_shapes_it_holds.rs`, `tests/behaviour/recording_versions.rs`, `crates/lockstep-headless/tests/recordings.rs`, and the tariffs fixture `examples/tariffs/fixtures/legacy-recording-v1.bin`, which converts to the hash in `legacy-recording-v1.hash` natively and under `wasm-pack test --node examples/tariffs`.
