<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0035 A save carries its snapshot version, and old shapes migrate one version at a time (decided)

Status: decided and implemented in milestone M16.

## Decision

`Runner::save` writes a `Save`: an envelope holding the envelope format, the simulation id, the
version of the snapshot shape (`Message::VERSION` of `Simulation::Snapshot`), the step and clock
configuration, the step number, the clock, the streams, and the snapshot as bytes. `Runner::from_save`
continues from it. The format and the simulation id are the first fields, so a tool can refuse an
unknown format and pick the simulation (`saved_simulation_id`) before it decodes anything else.

An older snapshot is loaded through `Migrations`: a list of typed conversions, each from one version's
type to the next version's type. Loading walks the chain from the save's version to the current one.
A shape that shipped in a save stays in the code, frozen, so its bytes can still be read; a changed
shape is a new version plus a conversion from the one before. This closes deferral D4 for snapshots.

- A version with no step to its successor is `MissingMigration`, a save newer than the code is
  `Newer`, and bytes that do not read as the shape their version names are a `Migration` error.
  Nothing is guessed.
- Two conversions from one version, or a conversion to an older version, panic when the chain is
  built, because they are mistakes in the code that builds it.
- A conversion keeps behaviour: it fills a new field with the value that makes the next step do what
  it did before the field existed. The consumer's old save, taken before transfers charged a fee,
  loads with a fee of nothing.
- Each shipped shape gets a committed save as bytes. A test loads it, continues it, and compares the
  hash with a committed file, natively and under WebAssembly, so an edit to a frozen shape or to a
  conversion fails.

## Alternatives rejected

- A conversion from every old version straight to the current one: each new shape would need all of
  them rewritten, and a missed pair fails only for the one save that needs it.
- Self-describing saves (field names in the bytes): the encoding is the fixed-endian bytes `hash_of`
  hashes, and a second format would be a second thing that must agree with it.
- Converting in the snapshot's `restore`: it would hide the version from the envelope and could not be
  tested against a committed old save without the old type.

## Deferred

Intents, events and configurations carry versions too, and a recording made by an older shape of
them does not decode. Converting those waits for a consumer that keeps old recordings; the
recordings remain valid for the code that made them.

## Would change if

A snapshot shape changes without a version bump, or an old committed save stops loading to its
committed hash. The first is not caught by types alone: the fixture test is the check.
