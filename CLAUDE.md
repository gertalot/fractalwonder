# Fractal Wonder

Fractal Wonder is a high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels
(up to 10^100 and beyond). Built entirely in Rust using Leptos and compiled to WebAssembly.

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

## DEVELOPMENT

**CRITICAL: Using Skills**

- NEVER claim to be "using a skill" without FIRST using the Read tool to read the skill file
- ALWAYS use `Read` tool on the skill file BEFORE announcing you're using it
- ONLY announce skill usage AFTER you have actually read the skill file
- Example: First `Read /Users/gert/.claude/plugins/cache/superpowers/skills/SKILLNAME/SKILL.md`, THEN announce "I'm using the SKILLNAME skill to..."
- Violating this rule is completely unacceptable

**GATE 0 CRITICAL CHECK: ONLY PROCEED IF YOU HAVE FOUND AND READ THE SKILL FILE AND ARE FOLLOWING ITS INSTRUCTIONS**

**Development tools**

- assume `trunk serve` is already running on <http://localhost:8080>
- use `context7` MCP and `WebSearch` to ensure you have up-to-date information
- use `chrome-devtools` MCP for browser testing/interactions
- use superpowers skills from `~/.claude/plugins/cache/superpowers`

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
