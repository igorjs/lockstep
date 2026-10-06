#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
banned='HashMap|HashSet|SystemTime|Instant::now|thread::spawn|rayon|f32::sin|f32::cos|f32::tan|f32::exp|f32::powf|f32::ln|thread_rng'
# The one exception: the path batch may use a thread pool, because each search is a pure function
# whose answer lands in its request's slot (docs/decisions/0006-path-batch.md).
allowed='^crates/lockstep-spatial/src/batch\.rs:[0-9]+:.*rayon'
if grep -rnE "$banned" crates/*/src examples/*/src | grep -vE "$allowed"; then
  echo "determinism lint: use lockstep_core::math and BTreeMap instead of the identifier above"; exit 1
fi
