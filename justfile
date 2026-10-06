set shell := ["bash", "-cu"]

default:
    @just --list

test:
    cargo test --workspace

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    ./scripts/lint-determinism.sh

determinism:
    cargo run -p lockstep-headless -- verify capsule --expect examples/capsule/fixtures/capsule.hash
    cargo run -p lockstep-headless -- verify ledger --expect examples/ledger/fixtures/ledger.hash
    wasm-pack test --node crates/lockstep-core
    wasm-pack test --node examples/capsule
    wasm-pack test --node examples/ledger

bench:
    cargo bench -p lockstep-spatial

ci: check test determinism
