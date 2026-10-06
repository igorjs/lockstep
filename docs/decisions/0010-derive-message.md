<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0010 Messages are derived, with a required version (decided)

Status: decided and implemented in milestone M5.

## Decision

`#[derive(Message)]` with `#[message(version = N)]` replaces the hand written `impl Message` blocks.
The version attribute is required, and the derive also implements `Indexable`: the kind is an
enum's variant index (0 for a struct), and the handles are every `Handle`, `Option<Handle>` and
`Vec<Handle>` field in field order. `lockstep-core` re-exports the derive, so one
`use lockstep_core::Message` brings both the trait and the derive.

The derive only declares facts the type already has. It does not change how a value serializes, so
no fixture hash changed when the examples moved onto it.

## Alternatives rejected

- A default version of 1 when the attribute is missing: a changed shape would ship under the old
  number without anyone deciding it.
- A separate `#[derive(Indexable)]`: every event needs both, and two derives that must agree are two
  places to forget.
- Detecting handles by a field attribute: the field type already says it, and an attribute is one
  more thing to forget.

Handles are found inside `Option`, `Vec`, `Box`, arrays, slices and tuples, nested to any depth.
A type that keeps handles somewhere the derive cannot see, such as a map, writes
`#[message(version = N, manual_indexable)]` and implements `Indexable` itself. A suffixed version
(`3u8`) and `#[message]` on a variant or a field are compile errors.

## Deferred

The spec also has the derive register the type for schema export. Nothing reads a schema yet, so
that waits for a consumer, as the backlog rule says.

Conversions between message versions also wait: a shape is frozen once a save holding it ships,
and no save ships before saves and migrations (milestone M16). Until then a version can change
without a conversion.

## Would change if

A type needs handles in a shape the derive does not see (a map or a nested struct): then a field
attribute or a manual `Indexable` becomes necessary.
