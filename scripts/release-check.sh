#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# A release dry run (docs/release.md). It never publishes and never tags: every publish command
# carries --dry-run. A crate whose dependencies are not on the registry yet cannot be dry run
# (cargo looks them up), so only leaf crates are, with --no-verify where the build is already
# covered by `just test`.
cd "$(dirname "$0")/.."
# Dependency order: each crate comes after the crates it depends on.
order=(
  lockstep-macros lockstep-core lockstep-attributes lockstep-calendar lockstep-knowledge
  lockstep-meta lockstep-relations lockstep-spatial lockstep-tables lockstep-inventory
  lockstep-combat lockstep-procgen lockstep-structures lockstep-progression lockstep-crafting
  lockstep-agents lockstep-regions
)
leaves=(lockstep-macros)
for crate in "${order[@]}"; do
  echo "== package list: $crate"
  cargo package --list --allow-dirty -p "$crate" > /dev/null
  echo "   ok"
done
for crate in "${leaves[@]}"; do
  echo "== publish dry run: $crate"
  cargo publish --dry-run --allow-dirty -p "$crate"
done
skipped=$(( ${#order[@]} - ${#leaves[@]} ))
echo "release check done: ${#order[@]} package lists, ${#leaves[@]} dry run, $skipped crates not dry run (internal dependencies are not on the registry)"
