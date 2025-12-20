# Fractal Wonder

Fractal Wonder is a high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels
(up to 10^100 and beyond). Built entirely in Rust using Leptos and compiled to WebAssembly.

YOU NEVER FORGET THE CORE GOAL OF WHAT WE ARE TRYING TO ACHIEVE: DESIGNING A MANDELBROT EXPLORER FROM THE GROUND UP TO
SUPPORT WORLD-RECORD DEEP ZOOMS OF 10e2000, WHICH MEANS DEALING WITH EXTREMELY LARGE AND SMALL NUMBERS. YOU WILL NEVER
USE .to_f64() ANYWHERE IN THE CODE UNLESS ABSOLUTELY NECESSARY, AND THEN YOU STOP AND ASK ME BEFORE IMPLEMENTING.


**Key Technologies:**

- Rust 1.80+ compiled to WASM (100% Rust, no TypeScript/JavaScript)
- Leptos 0.6+ (reactive frontend framework)
- Trunk (build tool and dev server)
- Cargo
- Tailwind CSS (styling)

## ARCHITECTURE

- we distinguish between "pixel space" which is represented by `f64` types, and "image space" which is a generic type,
  potentially using arbitrary precision.
- calculations in image space **MUST ALWAYS USE** the generic types in `src/rendering/coords.rs` and **NEVER**
  hardcode `f64` types for these calculations.
- Rust supports **RUNTIME POLYMORPHISM** via **TRAITS**. Anything that implements Trait X can be used AT
  RUNTIME where something needs to call a function defined by X. Traits are **EXACTLY LIKE INTERFACES IN OOP**.
- Note that the code uses **BOTH** Traits (runtime) **AND** generic types (compile time) where appropriate.

## DEVELOPMENT

**Development tools**

- assume `trunk serve` is **ALREADY** running on the host on <http://localhost:8080>.
  If it is not. **STOP** and ask the user
- use `context7` MCP and `WebSearch` to ensure you have up-to-date information
- use `chrome-devtools` MCP for browser testing/interactions

**Code Style:**

- Line length: 120 characters
- Indentation: 4 spaces
- Use strong types and explicit error handling
- Format/lint with Clippy/rustfmt
- strive for clean modular "DRY" reusable generic code
- refactor, no legacy code, no backwards compatibility, always clean up
- all code is production code; no placeholders or temporary solutions
- comments show the _why_ of the code
- no temporal names or comments ("new", "legacy", "updated", etc)

### Testing

These must complete with no warnings or errors:

```bash
# Format code
cargo fmt --all

# Run Clippy, the Rust linting tool
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all

# Check for compile/build errors
cargo check --workspace --all-targets --all-features

# Run tests with output visible
cargo test --workspace --all-targets --all-features -- --nocapture

# WASM browser tests
wasm-pack test --headless --chrome
```

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
