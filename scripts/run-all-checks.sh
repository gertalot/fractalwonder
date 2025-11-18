#! /bin/sh

RUSTFLAGS="-D warnings"

echo "—————————————— Running cargo fmt ——————————————"
cargo fmt --all

echo "—————————————— Running cargo clippy ——————————————"
cargo clippy --workspace --all-targets --all-features -- -W clippy::all

echo "—————————————— Running cargo check ——————————————"
cargo check --workspace --all-targets --all-features

echo "—————————————— Running cargo test ——————————————"
cargo test --workspace --all-targets --all-features

echo "—————————————— Running wasm-pack test ——————————————"
wasm-pack test --headless --chrome fractalwonder-ui