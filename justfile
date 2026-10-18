# SPDX-License-Identifier: Apache-2.0
set shell := ["bash", "-cu"]

default:
    @just --list

test:
    cargo test --workspace
    cargo test -p lockstep-spatial --features hex
    cargo test -p lockstep-spatial --features parallel
    cargo test -p lockstep-combat --features hex
    cargo test --release -p sparring --test spikes

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
    cargo run -p lockstep-headless -- verify ledger-bills --expect examples/ledger/fixtures/ledger-bills.hash
    cargo run -p lockstep-headless -- record ledger --out "${CARGO_TARGET_DIR:-target}/ledger.recording" --steps 3000
    cargo run -p lockstep-headless -- replay "${CARGO_TARGET_DIR:-target}/ledger.recording"
    cargo run -p lockstep-headless -- record bakery --out "${CARGO_TARGET_DIR:-target}/bakery.recording" --steps 3000
    cargo run -p lockstep-headless -- replay "${CARGO_TARGET_DIR:-target}/bakery.recording"
    cargo run -p lockstep-headless -- verify tariffs --expect examples/tariffs/fixtures/tariffs.hash
    cargo run -p lockstep-headless -- record tariffs --out "${CARGO_TARGET_DIR:-target}/tariffs.recording" --steps 3000
    cargo run -p lockstep-headless -- replay "${CARGO_TARGET_DIR:-target}/tariffs.recording" --until 1200 --save "${CARGO_TARGET_DIR:-target}/tariffs.save"
    cargo run -p lockstep-headless -- resume "${CARGO_TARGET_DIR:-target}/tariffs.save" --recording "${CARGO_TARGET_DIR:-target}/tariffs.recording"
    cargo run -p lockstep-headless -- verify rates --expect examples/rates/fixtures/rates.hash
    cargo run -p lockstep-headless -- record rates --out "${CARGO_TARGET_DIR:-target}/rates.recording" --steps 3500
    cargo run -p lockstep-headless -- replay "${CARGO_TARGET_DIR:-target}/rates.recording"
    cargo run -p lockstep-headless -- verify mars-rovers --expect examples/mars-rovers/fixtures/mars-rovers.hash
    cargo run -p lockstep-headless -- verify crowd --expect examples/crowd/fixtures/crowd.hash
    cargo run -p lockstep-headless -- verify drone-fleet --expect examples/drone-fleet/fixtures/drone-fleet.hash
    cargo run -p lockstep-headless -- verify sparring --expect examples/sparring/fixtures/sparring.hash
    cargo run -p lockstep-headless -- verify cargo-bay --expect examples/cargo-bay/fixtures/cargo-bay.hash
    cargo run -p lockstep-headless -- verify crater-survey --expect examples/crater-survey/fixtures/crater-survey.hash
    cargo run -p lockstep-headless -- verify audit --expect examples/audit/fixtures/audit.hash
    cargo run -p lockstep-headless -- verify training --expect examples/training/fixtures/training.hash
    cargo run -p lockstep-headless -- verify dispatch --expect examples/dispatch/fixtures/dispatch.hash
    cargo run -p lockstep-headless -- verify bakery --expect examples/bakery/fixtures/bakery.hash
    cargo run -p lockstep-headless -- verify estate --expect examples/estate/fixtures/estate.hash
    cargo run -p lockstep-headless -- verify roster --expect examples/roster/fixtures/roster.hash
    cargo run -p lockstep-headless -- verify checkout --expect examples/checkout/fixtures/checkout.hash
    cargo run -p lockstep-headless -- verify orchard --expect examples/orchard/fixtures/orchard.hash
    cargo run -p lockstep-headless --features parallel -- verify crowd --expect examples/crowd/fixtures/crowd.hash
    wasm-pack test --node crates/lockstep-core
    wasm-pack test --node crates/lockstep-attributes
    wasm-pack test --node crates/lockstep-spatial
    wasm-pack test --node crates/lockstep-spatial --features hex
    wasm-pack test --node crates/lockstep-combat --features hex
    wasm-pack test --node crates/lockstep-inventory
    wasm-pack test --node crates/lockstep-agents
    wasm-pack test --node crates/lockstep-knowledge
    wasm-pack test --node crates/lockstep-progression
    wasm-pack test --node crates/lockstep-relations
    wasm-pack test --node crates/lockstep-crafting
    wasm-pack test --node crates/lockstep-meta
    wasm-pack test --node crates/lockstep-calendar
    wasm-pack test --node crates/lockstep-regions
    wasm-pack test --node crates/lockstep-structures
    wasm-pack test --node crates/lockstep-procgen
    wasm-pack test --node crates/lockstep-tables
    wasm-pack test --node examples/capsule
    wasm-pack test --node examples/ledger
    wasm-pack test --node examples/tariffs
    wasm-pack test --node examples/rates
    wasm-pack test --node examples/mars-rovers
    wasm-pack test --node examples/crowd
    wasm-pack test --node examples/drone-fleet
    wasm-pack test --node examples/sparring
    wasm-pack test --node examples/cargo-bay
    wasm-pack test --node examples/crater-survey
    wasm-pack test --node examples/audit
    wasm-pack test --node examples/training
    wasm-pack test --node examples/dispatch
    wasm-pack test --node examples/bakery
    wasm-pack test --node examples/estate
    wasm-pack test --node examples/roster
    wasm-pack test --node examples/checkout
    wasm-pack test --node examples/orchard

bench:
    cargo bench -p lockstep-spatial --features parallel
    cargo bench -p lockstep-agents

# Measure again and replace the committed benchmark baseline. Run after an intended change.
bench-baseline:
    cargo bench -p lockstep-spatial --features parallel --bench spatial -- --write-baseline
    cargo bench -p lockstep-agents --bench agents -- --write-baseline

ci: check test determinism
