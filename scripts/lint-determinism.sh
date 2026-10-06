#!/usr/bin/env bash
set -euo pipefail
banned='HashMap|HashSet|SystemTime|Instant::now|thread::spawn|rayon|f32::sin|f32::cos|f32::tan|f32::exp|f32::powf|f32::ln|thread_rng'
if grep -rnE "$banned" crates/*/src examples/*/src; then
  echo "determinism lint: use lockstep_core::math and BTreeMap instead of the identifier above"; exit 1
fi
