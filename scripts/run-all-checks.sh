#! /bin/sh

echo "—————————————— Running cargo fmt ——————————————"
cargo fmt --all -- --check

echo "—————————————— Running cargo clippy ——————————————"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "—————————————— Running cargo check ——————————————"
cargo check --workspace --all-targets --all-features

echo "—————————————— Running cargo test ——————————————"
cargo test --workspace --all-targets --all-features