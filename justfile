# SPDX-License-Identifier: Apache-2.0
set shell := ["bash", "-cu"]

default:
    @just --list

test:
    cargo test --workspace
    cargo test -p lockstep-spatial --features hex
    cargo test -p lockstep-spatial --features parallel
    cargo test -p lockstep-combat --features hex

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p lockstep-spatial --all-targets --features hex -- -D warnings
    cargo clippy -p lockstep-spatial --all-targets --features parallel -- -D warnings
    cargo clippy -p lockstep-combat --all-targets --features hex -- -D warnings
    ./scripts/lint-determinism.sh
    ./scripts/check-spdx.sh

determinism:
    cargo run -p lockstep-headless -- verify capsule --expect examples/capsule/fixtures/capsule.hash
    cargo run -p lockstep-headless -- verify ledger --expect examples/ledger/fixtures/ledger.hash
    cargo run -p lockstep-headless -- record ledger --out "${CARGO_TARGET_DIR:-target}/ledger.recording" --steps 3000
    cargo run -p lockstep-headless -- replay "${CARGO_TARGET_DIR:-target}/ledger.recording"
    cargo run -p lockstep-headless -- verify mars-rovers --expect examples/mars-rovers/fixtures/mars-rovers.hash
    cargo run -p lockstep-headless -- verify crowd --expect examples/crowd/fixtures/crowd.hash
    cargo run -p lockstep-headless -- verify drone-fleet --expect examples/drone-fleet/fixtures/drone-fleet.hash
    cargo run -p lockstep-headless --features parallel -- verify crowd --expect examples/crowd/fixtures/crowd.hash
    wasm-pack test --node crates/lockstep-core
    wasm-pack test --node crates/lockstep-attributes
    wasm-pack test --node crates/lockstep-spatial
    wasm-pack test --node crates/lockstep-spatial --features hex
    wasm-pack test --node crates/lockstep-combat --features hex
    wasm-pack test --node examples/capsule
    wasm-pack test --node examples/ledger
    wasm-pack test --node examples/mars-rovers
    wasm-pack test --node examples/crowd
    wasm-pack test --node examples/drone-fleet

bench:
    cargo bench -p lockstep-spatial --features parallel

# Measure again and replace the committed benchmark baseline. Run after an intended change.
bench-baseline:
    cargo bench -p lockstep-spatial --features parallel --bench spatial -- --write-baseline

ci: check test determinism
