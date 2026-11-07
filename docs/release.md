<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release

**No tag is pushed and no crate is published in this work.** This page describes the steps for
the person who releases, and what `just release-check` proves before they start. Nothing here
runs `cargo publish` without `--dry-run`.

## What is checked on every change

- `scripts/check-publish.sh` (in `just check`): library crates carry keywords, categories and a
  rust-version, every path dependency has a `version`, and the headless crate, the fuzz crate and
  every example set `publish = false`.
- `scripts/check-changelog.sh` (in `just check`): `CHANGELOG.md` starts with the SPDX line, has no
  em or en dash, has an entry for each of M1 to M16, and every entry names decision numbers that
  exist in `docs/decisions`.

## The dry run

`just release-check` runs `scripts/release-check.sh`:

1. `cargo package --list` for each of the 17 publishable crates, in dependency order. It shows
   which files each package would contain.
2. `cargo publish --dry-run` only for crates whose dependencies need no registry lookup. Cargo
   resolves a crate's dependencies from the registry, and no `lockstep-*` crate is there yet, so
   only `lockstep-macros` (no internal dependency) can be dry run. It is verified by a build in a
   clean directory; `--no-verify` is not needed for it. The other 16 crates cannot be dry run
   until their dependencies are published, and the script says so.

The script passes `--allow-dirty` so it runs on a working tree with changes. Run it on a clean
checkout before a release.

Result recorded with this change: 17 package lists passed, the `lockstep-macros` dry run passed
and stopped with "aborting upload due to dry run", 16 crates not dry run.

## Release steps (for the releaser, not done here)

1. Merge to `main` with `just ci` green on the three platforms.
2. Run `just release-check` on a clean checkout of `main`.
3. Publish in dependency order, waiting for the registry index between crates: `lockstep-macros`,
   `lockstep-core`, then the crates that depend only on core (`lockstep-attributes`,
   `lockstep-calendar`, `lockstep-knowledge`, `lockstep-meta`, `lockstep-relations`,
   `lockstep-spatial`, `lockstep-tables`), then `lockstep-inventory`, `lockstep-combat`,
   `lockstep-procgen`, `lockstep-structures`, `lockstep-progression`, `lockstep-crafting`,
   `lockstep-agents` and `lockstep-regions`. Dry run each crate just before it is published, since
   its dependencies are then on the registry.
4. Move the `0.1.0 (not released)` heading in `CHANGELOG.md` to a dated heading, tag, and push the
   tag.

Crate name availability on crates.io has not been checked.

## Known gaps in the release setup

- No continuous integration job builds with the declared `rust-version` of 1.97. The jobs use the stable compiler, so
  a use of a newer feature would not be caught. A job pinned to 1.97 would close this.
- The internal version `0.1.0` is typed on each internal dependency line (about 30 lines across
  the library crates), because workspace inheritance of path dependencies was not adopted. A
  version bump edits every line. `scripts/check-publish.sh` only checks that a version exists, not
  that it matches the crate's own.
