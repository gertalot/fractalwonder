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
cargo clippy --workspace --all-targets --all-features -- -D warnings

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

## Architecture

### High-Level Design Philosophy

This application separates **computation** from **visualization**:

1. **Computation engine**: Calculates Mandelbrot iteration data (iterations, escape flags, z values, derivatives) using arbitrary precision math
2. **Coloring/visualization**: Transforms iteration data into RGB pixels using various color schemes

This separation allows re-coloring without recomputation and enables future backend/distributed rendering without rewriting math code.

### Core Architectural Concepts

**Extreme Precision Requirements:**

- Variables must be explicitly marked as **pixel-space** (no arbitrary precision needed) or **fractal-space** (extreme precision required)
- At zoom 10^100, coordinate changes can be 250+ decimal places
- Pixel-to-fractal and fractal-to-pixel transformations must maintain full precision at all zoom levels
- String serialization is used to pass coordinates between Rust/WASM and JavaScript without precision loss

**Perturbation Theory (planned):**

- Reference orbit: High-precision calculation at viewport center using `rug::Float`
- Delta orbits: Per-pixel deltas from reference using f64 (much faster)
- Enables extreme zoom performance without computing every pixel in arbitrary precision

**Progressive Rendering:**

- Tile-based rendering with adaptive tile size based on zoom level
- Spiral pattern from center outward (most interesting content first)
- Render cancellation on user interaction to save CPU/GPU cycles

**User Interaction Model:**

- Fast previews: Transform existing imageData in pixel-space during drag/zoom (must feel smooth even at 10^100 zoom)
- Debounce delay: 1.5 seconds after interaction stops before new render starts
- UI auto-hide: Fades out after 4 seconds of mouse inactivity

### Current Code Structure

```
src/
├── lib.rs              # WASM entry point (hydrate function)
├── app.rs              # Main App component (combines Canvas + UI)
└── components/
    ├── mod.rs
    ├── canvas.rs       # Full-screen canvas rendering (placeholder checkerboard)
    ├── ui.rs           # Bottom UI bar component
    └── ui_visibility.rs # Auto-hide/show logic (4-second timeout, 300ms fade)
```

**Current Implementation Status:**

- ✅ Basic Leptos app structure with CSR (client-side rendering)
- ✅ Full-screen canvas component
- ✅ UI auto-hide/show behavior (4s timeout, 300ms fade)
- ✅ Tailwind CSS styling
- ⏳ Fractal computation engine (not yet implemented)
- ⏳ User interactions (pan, zoom, resize)
- ⏳ Coordinate transformation math
- ⏳ State persistence (URL + localStorage)
- ⏳ Color schemes

### Planned Architecture (Future Modules)

Based on DESIGN.md, the system will grow into these modules:

- **fractal-core**: Mandelbrot computation with arbitrary precision (rug), perturbation theory
- **fractal-render**: Tile-based rendering orchestration, spiral pattern generation
- **fractal-ui**: Current UI components (already scaffolded)
- **fractal-state**: Viewport state management, URL/localStorage persistence
- **fractal-export**: PNG export with embedded metadata

### Critical Implementation Notes

**Testing Philosophy:**

- Tests MUST be designed from first principles to validate mathematical correctness
- Tests should FAIL until implementation is correct (not designed to easily pass)
- Round-trip invariants: `pixel → fractal → pixel = original` at zoom levels 10^15, 10^30, 10^50, 10^100
- Zoom invariant: "Fractal point under mouse stays under mouse during zoom"
- Use property-based testing (proptest) for coordinate transformations

**Precision Calculations:**

```
precision_bits = ceil(decimal_places × 3.322)
decimal_places = max(30, ceil(log10(zoom) × 2.5 + 20))

Examples:
  Zoom 10^15  → 58 decimal places  → 193 bits
  Zoom 10^50  → 145 decimal places → 482 bits
  Zoom 10^100 → 270 decimal places → 897 bits
```

**Coordinate Transformation Example (from DESIGN.md):**

```rust
// Pixel to fractal MUST maintain arbitrary precision
fn pixel_to_fractal(
    pixel_x: f64,
    pixel_y: f64,
    viewport: &ViewportParams,
) -> (rug::Float, rug::Float) {
    // Calculate precision based on zoom
    // Use rug::Float for all fractal-space calculations
    // Avoid f64 truncation in fractal coordinates
}
```

**Render Cancellation:**

- Drag: Cancel immediately when mouse moves (not on mousedown alone)
- Zoom: Cancel on wheel event
- Resize: Cancel ONLY if canvas size actually changes (ignore resize handle clicks without size change)

**State Persistence:**

- Priority: URL params → localStorage → defaults
- URL encoding: Base64-encoded JSON with arbitrary precision coordinates as strings
- Default viewport: real [-2, 1], imaginary [-1.5, 1.5], zoom 1.0, iterations 500

## Important Documentation

**Read these for detailed specifications:**

- `docs/PROJECT.md` - Original project vision and UI mockups
- `docs/DESIGN.md` - Complete architectural design (203 lines, extremely detailed)
- `docs/REQUIREMENTS.md` - Formal requirements document with acceptance criteria
- `docs/STORY-001-project-setup.md` - Story 1 (scaffolding, completed)

**UI Design Reference:**
Three screenshots in docs/ directory show exact UI layout:

- `fractal-ui-visible.png`
- `fractal-progress-indicator-no-ui.png`
- `fractal-progress-indicator-ui.png`

## Development Guidelines (from .cursor/rules/)

**CRITICAL: Address as "Big Boss"**

- You must critically evaluate ALL instructions before proceeding (GATE 0 check)
- Question flawed, ambiguous, or contradictory instructions immediately
- Use Context7 (MCP) and web search for up-to-date best practices
- Never use temporal qualifiers in names ("new", "old", "legacy", "wrapper")
- All code is production code - no workarounds or temporary solutions

**Code Style:**

- Line length: 120 characters max
- Indentation: 2 spaces
- Use strong types and explicit error handling
- Add ABOUTME comments (2 lines) at top of Rust files explaining file purpose
- Auto-format with Clippy/rustfmt

**Testing:**

- Follow TDD: red → green → refactor
- Write tests that FAIL until implementation is mathematically correct
- Validate mathematical invariants, not just "does it run"

**Do NOT:**

- Start a dev server (assume already running)
- Start Playwright browser (assume already running)
- Use `rm -rf`, `kill`, or `pkill` without approval
- Make up information or lie if uncertain - STOP and ask Big Boss

## Future Enhancements (Out of Scope for v1)

- GPU acceleration via WebGPU (v2)
- Bivariate Linear Approximation optimization (v3)
- Backend/distributed rendering (v4)
- Custom color scheme editor, animations, Julia sets (v5+)
