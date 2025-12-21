#! /bin/sh

RUSTFLAGS="-D warnings"

# Parse arguments
INCLUDE_IGNORED=""
for arg in "$@"; do
    case $arg in
        --full)
            INCLUDE_IGNORED="--include-ignored"
            ;;
    esac
done

echo "—————————————— Running cargo fmt ——————————————"
cargo fmt --all

echo "—————————————— Running cargo clippy ——————————————"
cargo clippy --workspace --all-targets --all-features -- -W clippy::all

echo "—————————————— Running cargo check ——————————————"
cargo check --workspace --all-targets --all-features

echo "—————————————— Running cargo test ——————————————"
cargo test --workspace --all-targets --all-features -- $INCLUDE_IGNORED

echo "—————————————— Running wasm-pack test ——————————————"
wasm-pack test --headless --chrome fractalwonder-ui

echo "—————————————— Validating WGSL shaders ——————————————"
find . -name "*.wgsl" -type f -print0 | xargs -0 naga --bulk-validate
