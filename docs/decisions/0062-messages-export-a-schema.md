<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0062 Messages export a schema (decided)

Status: decided and implemented in milestone M16. Closes deferral D1 of decision 0010.

## Decision

`#[derive(Message)]` writes `Message::SCHEMA`, a constant string of JSON: the type's name, version
and shape.

- A struct has `kind` `struct`, a `form` (`named`, `tuple` or `unit`) and its `fields`.
- An enum has `kind` `enum` and its `variants` in declaration order, each with its `index` (the
  kind the timeline indexes it by), `name`, `form` and `fields`.
- A field has its `name` (the position for a tuple field) and its `type` as written in the source,
  with the spaces the token printer adds removed, so `Vec<Option<(u8,[u16;2])>>`.
- A generic message lists its `generics`.
- A hand written `impl Message` leaves `SCHEMA` empty.

`lockstep-headless schema <name>` prints the four messages of a fixture's simulation (intent,
event, snapshot, configuration) as one JSON document. With `--out <directory>` it writes one file
for each message, named `<Type>.v<version>.json`, with a license sidecar.

The text is built at compile time from the syntax tree, so it costs nothing at run time and cannot
differ between platforms.

## Alternatives rejected

- Types resolved to their definitions: a macro sees syntax only, so an alias or a type from another
  crate stays a name. A tool that needs the shape of `KindId` reads the schema of `KindId`.
- A registry of every message type in a program: it needs life before main or a list kept by hand.
  The headless table already names each simulation's four types.
- A hash of the shape in place of text: a failing comparison would not say what changed.

## Would change if

`a_changed_shape_needs_a_new_version` fails: two shapes give one text, or one shape gives two.
