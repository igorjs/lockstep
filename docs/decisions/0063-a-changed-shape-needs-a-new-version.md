<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0063 A changed shape needs a new version (decided)

Status: decided and implemented in milestone M16.

## Decision

An example keeps its schemas in `examples/<name>/fixtures/schema/<Type>.v<version>.json`, one file
for each message type and each version it has shipped. A test compares the type's current
`SCHEMA` with the file for its current version (headless `schema` tests for every example that has
a `fixtures/schema` directory, and a test in `bakery` and `tariffs` that also runs under
WebAssembly). So:

- A field added, removed, renamed or retyped under the same version no longer equals its file, and
  the test fails.
- A version bump has no file yet, and the test fails until `lockstep-headless schema <name> --out
  examples/<name>/fixtures/schema` writes it.
- The file of an older version stays as it was, which is the record of what that version was.
  `schema --out` refuses to rewrite a file whose text differs, so a changed shape cannot be
  saved over its own version.

The limit is the one of decision 0062: a change inside a nested type without a schema of its own
is not seen. Deriving `Message` on that type, and listing it, would close it.

`bakery` and `tariffs` keep their schemas. Another example joins by running the same command.

## Alternatives rejected

- One schema file per type, overwritten on a change: the test would pass after any edit, and the
  old shape would be lost.
- A check in the macro itself: it cannot see the saved files, and a compile error on every edit
  would block work in progress.
- Schemas for every example now: dozens of files nobody reads yet. The command writes them on
  demand.

## Would change if

`every_saved_schema_matches_its_message_at_the_current_version` fails: a type changed shape without
a new version, which is a design question, not a file to refresh.
