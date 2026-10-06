set shell := ["bash", "-cu"]

default:
    @just --list

test:
    cargo test --workspace
    cargo test -p lockstep-spatial --features hex

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p lockstep-spatial --all-targets --features hex -- -D warnings
    ./scripts/lint-determinism.sh

determinism:
    cargo run -p lockstep-headless -- verify capsule --expect examples/capsule/fixtures/capsule.hash
    cargo run -p lockstep-headless -- verify ledger --expect examples/ledger/fixtures/ledger.hash
    cargo run -p lockstep-headless -- verify mars-rovers --expect examples/mars-rovers/fixtures/mars-rovers.hash
    wasm-pack test --node crates/lockstep-core
    wasm-pack test --node crates/lockstep-spatial
    wasm-pack test --node crates/lockstep-spatial --features hex
    wasm-pack test --node examples/capsule
    wasm-pack test --node examples/ledger
    wasm-pack test --node examples/mars-rovers

bench:
    cargo bench -p lockstep-spatial

# Measure again and replace the committed benchmark baseline. Run after an intended change.
bench-baseline:
    cargo bench -p lockstep-spatial --bench spatial -- --write-baseline

ci: check test determinism
