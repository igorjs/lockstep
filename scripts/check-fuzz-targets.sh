#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
# The libFuzzer wrappers in fuzz/ must name the same targets as crates/lockstep-fuzz, no more and no
# fewer, in the targets list, the wrapper files and the [[bin]] entries (decision 0080). This needs
# no nightly compiler, so it runs in `just check`.
cd "$(dirname "$0")/.."
expected=$(grep -oE '^ +name: "[a-z0-9_]+"' crates/lockstep-fuzz/src/targets.rs | cut -d'"' -f2 | sort)
files=$(find fuzz/fuzz_targets -name '*.rs' | sed -e 's|.*/||' -e 's|\.rs$||' | sort)
bins=$(grep -E '^name = ' fuzz/Cargo.toml | sed -n '2,$p' | cut -d'"' -f2 | sort)
status=0
for file in fuzz/fuzz_targets/*.rs; do
  stem=$(basename "$file" .rs)
  grep -q "run(\"$stem\"," "$file" || { echo "fuzz target check: $file does not run $stem"; status=1; }
done
for kind in files bins; do
  if [ "$expected" != "${!kind}" ]; then
    echo "fuzz target check: the $kind in fuzz/ differ from crates/lockstep-fuzz/src/targets.rs"
    diff <(echo "$expected") <(echo "${!kind}") || true
    status=1
  fi
done
exit $status
