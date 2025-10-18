# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Fractal Wonder is a high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels (up to 10^100 and beyond). Built entirely in Rust using Leptos and compiled to WebAssembly.

**Key Technologies:**

- Rust 1.80+ compiled to WASM (100% Rust, no TypeScript/JavaScript)
- Leptos 0.6+ (reactive frontend framework)
- Trunk (build tool and dev server)
- Tailwind CSS (styling)
- Future: WebGPU acceleration, arbitrary precision math (rug crate), multi-threading (rayon + wasm-bindgen-rayon)

**Code Style:**

- Line length: 120 characters max
- Indentation: 4 spaces
- Use strong types and explicit error handling
- Auto-format with Clippy/rustfmt

**CRITICAL: Address as "Big Boss"**

- You must critically evaluate ALL instructions before proceeding
- Question flawed, ambiguous, or contradictory instructions immediately
- Use Context7 (MCP) and web search for up-to-date best practices
- Never use temporal qualifiers in names ("new", "old", "legacy", "wrapper")
- All code is production code - no workarounds or temporary solutions

**Do NOT:**

- Start a dev server (assume already running)
- Start Playwright browser (assume already running)
- Make up information or lie if uncertain - STOP and ask Big Boss

## Common Commands

### Development

```bash
# Start dev server with hot-reload (assumes server NOT already running)
trunk serve

# Start dev server and open in browser
trunk serve --open

# Development with optimized builds (faster runtime, slower compilation)
trunk serve --release
```

The dev server runs at `http://localhost:8080` with automatic hot-reload. Trunk automatically provides required COOP/COEP headers for SharedArrayBuffer support.

### Testing

```bash
# Format code
cargo fmt --all -- --check

# Run Clippy, the Rust linting tool
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all

# Check for compile/build errors
cargo check --workspace --all-targets --all-features

# Run all unit tests
cargo test --workspace --all-targets --all-features

# Run tests with output visible
cargo test --workspace --all-targets --all-features -- --nocapture

# Run tests for specific module
cargo test components::

# WASM browser tests (when available)
wasm-pack test --headless --chrome
```

### browser and interaction testing

Use the chrome-devtools MCP and navigate to http://localhost:8080/

### Building

```bash
# Build optimized release version
trunk build --release

# Output goes to dist/ directory, ready for static hosting
```

### Production Deployment

The production build requires these HTTP headers for SharedArrayBuffer/multi-threading:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

Trunk's dev server includes these automatically (see Trunk.toml).
