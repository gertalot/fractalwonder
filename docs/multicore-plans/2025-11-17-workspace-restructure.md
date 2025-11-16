# Workspace Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Separate codebase into workspace crates based on DOM dependencies to enable Web Workers

**Architecture:** Split single crate into three: fractalwonder-core (shared types), fractalwonder-compute (computation engine, no DOM), fractalwonder-ui (Leptos + DOM). This separation allows workers to load compute WASM without DOM dependencies.

**Tech Stack:** Rust 1.80+, Cargo workspaces, Leptos 0.6, Trunk

---

## Task 1: Create Workspace Root Cargo.toml

**Files:**
- Modify: `Cargo.toml` (entire file)

**Step 1: Back up current Cargo.toml**

```bash
cp Cargo.toml Cargo.toml.backup
```

**Step 2: Replace with workspace manifest**

Replace entire contents of `Cargo.toml`:

```toml
[workspace]
members = [
    "fractalwonder-core",
    "fractalwonder-compute",
    "fractalwonder-ui",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies - versions defined once
fractalwonder-core = { path = "./fractalwonder-core" }
fractalwonder-compute = { path = "./fractalwonder-compute" }

# External dependencies
leptos = { version = "0.6", features = ["csr"] }
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"
console_log = "1.0"
leptos-use = "0.13"
js-sys = "0.3"
wasm-bindgen-futures = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dashu = "0.4"
dashu-float = "0.4"
dyn-clone = "1.0.20"
web-sys = "0.3"

[workspace.dependencies.web-sys]
version = "0.3"
features = [
  "Window",
  "Document",
  "HtmlCanvasElement",
  "CanvasRenderingContext2d",
  "ContextAttributes2d",
  "ImageData",
  "MouseEvent",
  "EventTarget",
  "CssStyleDeclaration",
  "Element",
  "HtmlElement",
  "PointerEvent",
  "WheelEvent",
  "Performance",
  "console",
  "AddEventListenerOptions",
  "Storage",
]

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

**Step 3: Verify syntax**

```bash
cargo read-manifest --manifest-path Cargo.toml
```

Expected: Valid JSON output (no errors)

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore: convert to workspace manifest"
```

---

## Task 2: Create fractalwonder-core Crate

**Files:**
- Create: `fractalwonder-core/Cargo.toml`
- Create: `fractalwonder-core/src/lib.rs`

**Step 1: Create directory structure**

```bash
mkdir -p fractalwonder-core/src
```

**Step 2: Create Cargo.toml**

Create `fractalwonder-core/Cargo.toml`:

```toml
[package]
name = "fractalwonder-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[dependencies]
dashu.workspace = true
dashu-float.workspace = true
serde.workspace = true
dyn-clone.workspace = true
```

**Step 3: Create lib.rs with placeholder**

Create `fractalwonder-core/src/lib.rs`:

```rust
//! Shared types and utilities for Fractal Wonder
//!
//! This crate contains types used by both compute and UI layers,
//! with NO DOM dependencies.

// Will be populated in next tasks
```

**Step 4: Verify build**

```bash
cargo build -p fractalwonder-core
```

Expected: "Compiling fractalwonder-core" → success

**Step 5: Commit**

```bash
git add fractalwonder-core/
git commit -m "chore: create fractalwonder-core crate"
```

---

## Task 3: Create fractalwonder-compute Crate

**Files:**
- Create: `fractalwonder-compute/Cargo.toml`
- Create: `fractalwonder-compute/src/lib.rs`

**Step 1: Create directory structure**

```bash
mkdir -p fractalwonder-compute/src
```

**Step 2: Create Cargo.toml**

Create `fractalwonder-compute/Cargo.toml`:

```toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[dependencies]
fractalwonder-core.workspace = true
dyn-clone.workspace = true
```

**Step 3: Create lib.rs with placeholder**

Create `fractalwonder-compute/src/lib.rs`:

```rust
//! Fractal computation engine
//!
//! This crate contains all rendering and computation logic,
//! with NO DOM dependencies (can be loaded in Web Workers).

// Re-export core types
pub use fractalwonder_core::*;

// Will be populated in next tasks with rendering logic
```

**Step 4: Verify build**

```bash
cargo build -p fractalwonder-compute
```

Expected: "Compiling fractalwonder-compute" → success

**Step 5: Commit**

```bash
git add fractalwonder-compute/
git commit -m "chore: create fractalwonder-compute crate"
```

---

## Task 4: Create fractalwonder-ui Crate

**Files:**
- Create: `fractalwonder-ui/Cargo.toml`
- Create: `fractalwonder-ui/src/lib.rs`

**Step 1: Create directory structure**

```bash
mkdir -p fractalwonder-ui/src
```

**Step 2: Create Cargo.toml**

Create `fractalwonder-ui/Cargo.toml`:

```toml
[package]
name = "fractalwonder-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-compute.workspace = true
leptos.workspace = true
wasm-bindgen.workspace = true
console_error_panic_hook.workspace = true
console_log.workspace = true
leptos-use.workspace = true
js-sys.workspace = true
wasm-bindgen-futures.workspace = true
serde.workspace = true
serde_json.workspace = true
web-sys.workspace = true

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

**Step 3: Create lib.rs with hydrate function**

Create `fractalwonder-ui/src/lib.rs`:

```rust
//! Fractal Wonder UI
//!
//! Leptos-based user interface with DOM dependencies.

use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| {
        view! {
          <p>"Fractal Wonder - Workspace Migration"</p>
        }
    });
}
```

**Step 4: Verify build**

```bash
cargo build -p fractalwonder-ui --target wasm32-unknown-unknown
```

Expected: "Compiling fractalwonder-ui" → success

**Step 5: Commit**

```bash
git add fractalwonder-ui/
git commit -m "chore: create fractalwonder-ui crate with hydrate"
```

---

## Task 5: Move Core Types to fractalwonder-core

**Files:**
- Move: `src/rendering/points.rs` → `fractalwonder-core/src/points.rs`
- Move: `src/rendering/viewport.rs` → `fractalwonder-core/src/viewport.rs`
- Move: `src/rendering/pixel_rect.rs` → `fractalwonder-core/src/pixel_rect.rs`
- Move: `src/rendering/numeric.rs` → `fractalwonder-core/src/numeric.rs`
- Move: `src/rendering/transforms.rs` → `fractalwonder-core/src/transforms.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Copy files**

```bash
cp src/rendering/points.rs fractalwonder-core/src/
cp src/rendering/viewport.rs fractalwonder-core/src/
cp src/rendering/pixel_rect.rs fractalwonder-core/src/
cp src/rendering/numeric.rs fractalwonder-core/src/
cp src/rendering/transforms.rs fractalwonder-core/src/
```

**Step 2: Update lib.rs**

Replace `fractalwonder-core/src/lib.rs`:

```rust
//! Shared types and utilities for Fractal Wonder
//!
//! This crate contains types used by both compute and UI layers,
//! with NO DOM dependencies.

pub mod numeric;
pub mod pixel_rect;
pub mod points;
pub mod transforms;
pub mod viewport;

// Re-exports for convenience
pub use numeric::{BigFloat, ToF64};
pub use pixel_rect::PixelRect;
pub use points::{Point, Rect};
pub use transforms::{
    apply_pixel_transform_to_viewport, calculate_aspect_ratio, calculate_visible_bounds,
    image_to_pixel, pan_viewport, pixel_to_image, zoom_viewport_at_point,
};
pub use viewport::Viewport;
```

**Step 3: Fix import paths in moved files**

In each moved file, change imports from `crate::rendering::` to `crate::`:

```bash
# In fractalwonder-core/src/viewport.rs
sed -i '' 's/crate::rendering::/crate::/g' fractalwonder-core/src/viewport.rs

# Repeat for other files
sed -i '' 's/crate::rendering::/crate::/g' fractalwonder-core/src/transforms.rs
sed -i '' 's/crate::rendering::/crate::/g' fractalwonder-core/src/pixel_rect.rs
```

**Step 4: Verify build**

```bash
cargo build -p fractalwonder-core
```

Expected: Build succeeds

**Step 5: Commit**

```bash
git add fractalwonder-core/src/
git commit -m "feat: move core types to fractalwonder-core"
```

---

## Task 6: Move Compute Logic to fractalwonder-compute

**Files:**
- Move: `src/rendering/` → `fractalwonder-compute/src/` (except colorizers, tiling_canvas_renderer, canvas_*)
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create subdirectories**

```bash
mkdir -p fractalwonder-compute/src/computers
```

**Step 2: Copy renderer trait and implementations**

```bash
cp src/rendering/renderer_trait.rs fractalwonder-compute/src/
cp src/rendering/point_compute.rs fractalwonder-compute/src/
cp src/rendering/pixel_renderer.rs fractalwonder-compute/src/
cp src/rendering/app_data.rs fractalwonder-compute/src/
cp src/rendering/app_data_renderer.rs fractalwonder-compute/src/
cp src/rendering/adaptive_mandelbrot_renderer.rs fractalwonder-compute/src/
cp src/rendering/precision.rs fractalwonder-compute/src/
cp src/rendering/render_config.rs fractalwonder-compute/src/
cp src/rendering/renderer_info.rs fractalwonder-compute/src/

cp src/rendering/computers/mandelbrot.rs fractalwonder-compute/src/computers/
cp src/rendering/computers/test_image.rs fractalwonder-compute/src/computers/
cp src/rendering/computers/mod.rs fractalwonder-compute/src/computers/
```

**Step 3: Update lib.rs**

Replace `fractalwonder-compute/src/lib.rs`:

```rust
//! Fractal computation engine
//!
//! This crate contains all rendering and computation logic,
//! with NO DOM dependencies (can be loaded in Web Workers).

pub mod adaptive_mandelbrot_renderer;
pub mod app_data;
pub mod app_data_renderer;
pub mod computers;
pub mod pixel_renderer;
pub mod point_compute;
pub mod precision;
pub mod render_config;
pub mod renderer_info;
pub mod renderer_trait;

// Re-export core types
pub use fractalwonder_core::*;

// Re-export compute types
pub use adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
pub use app_data::{AppData, TestImageData};
pub use app_data_renderer::AppDataRenderer;
pub use computers::{MandelbrotComputer, TestImageComputer};
pub use pixel_renderer::PixelRenderer;
pub use point_compute::ImagePointComputer;
pub use precision::PrecisionCalculator;
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
pub use renderer_info::RendererInfo;
pub use renderer_trait::Renderer;
```

**Step 4: Fix import paths**

In all moved files, change:
- `crate::rendering::` → `crate::` or `fractalwonder_core::`
- `use crate::rendering::BigFloat` → `use fractalwonder_core::BigFloat`

```bash
# This needs to be done carefully for each file
# Example for renderer_trait.rs:
# Change: use crate::rendering::{points::Rect, viewport::Viewport, PixelRect};
# To: use fractalwonder_core::{Rect, Viewport, PixelRect};
```

**Step 5: Verify build**

```bash
cargo build -p fractalwonder-compute
```

Expected: Build succeeds (may need to fix imports iteratively)

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/
git commit -m "feat: move computation logic to fractalwonder-compute"
```

---

## Task 7: Move UI Components to fractalwonder-ui

**Files:**
- Move: `src/rendering/colorizers.rs` → `fractalwonder-ui/src/colorizers.rs`
- Move: `src/rendering/canvas_renderer.rs` → `fractalwonder-ui/src/canvas_renderer.rs`
- Move: `src/rendering/canvas_utils.rs` → `fractalwonder-ui/src/canvas_utils.rs`
- Move: `src/rendering/tiling_canvas_renderer.rs` → `fractalwonder-ui/src/tiling_canvas_renderer.rs`
- Move: `src/app.rs` → `fractalwonder-ui/src/app.rs`
- Move: `src/components/` → `fractalwonder-ui/src/components/`
- Move: `src/hooks/` → `fractalwonder-ui/src/hooks/`
- Move: `src/state/` → `fractalwonder-ui/src/state/`

**Step 1: Copy files and directories**

```bash
cp src/rendering/colorizers.rs fractalwonder-ui/src/
cp src/rendering/canvas_renderer.rs fractalwonder-ui/src/
cp src/rendering/canvas_utils.rs fractalwonder-ui/src/
cp src/rendering/tiling_canvas_renderer.rs fractalwonder-ui/src/

cp src/app.rs fractalwonder-ui/src/
cp -r src/components fractalwonder-ui/src/
cp -r src/hooks fractalwonder-ui/src/
cp -r src/state fractalwonder-ui/src/
```

**Step 2: Update lib.rs**

Replace `fractalwonder-ui/src/lib.rs`:

```rust
//! Fractal Wonder UI
//!
//! Leptos-based user interface with DOM dependencies.

mod app;
pub mod canvas_renderer;
pub mod canvas_utils;
pub mod colorizers;
pub mod components;
pub mod hooks;
pub mod state;
pub mod tiling_canvas_renderer;

use leptos::*;
use wasm_bindgen::prelude::*;

// Re-export for convenience
pub use canvas_renderer::CanvasRenderer;
pub use canvas_utils::render_with_viewport;
pub use colorizers::{test_image_default_colorizer, Colorizer};
pub use tiling_canvas_renderer::TilingCanvasRenderer;

// Re-export compute types
pub use fractalwonder_compute::*;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| {
        view! {
          <app::App />
        }
    });
}
```

**Step 3: Fix import paths**

In all moved files, change:
- `crate::rendering::` → `fractalwonder_compute::`
- `use crate::app` → `use crate::app` (stays same)

**Step 4: Verify build**

```bash
cargo build -p fractalwonder-ui --target wasm32-unknown-unknown
```

Expected: Build succeeds (fix imports if needed)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/
git commit -m "feat: move UI components to fractalwonder-ui"
```

---

## Task 8: Update index.html for Trunk

**Files:**
- Modify: `index.html`

**Step 1: Update Trunk references**

Replace the rust link in `index.html`:

```html
<!-- OLD: -->
<link data-trunk rel="rust" />

<!-- NEW: -->
<link data-trunk rel="rust" href="./fractalwonder-ui/Cargo.toml"/>
```

Full updated `index.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Fractal Wonder</title>

    <!-- Main UI WASM -->
    <link data-trunk rel="rust" href="./fractalwonder-ui/Cargo.toml"/>

    <!-- Tailwind CSS -->
    <link data-trunk rel="tailwind-css" href="./input.css"/>
</head>
<body></body>
</html>
```

**Step 2: Commit**

```bash
git add index.html
git commit -m "chore: update index.html for workspace structure"
```

---

## Task 9: Delete Old src Directory

**Files:**
- Delete: `src/` (entire directory)

**Step 1: Verify all files have been moved**

```bash
# Check what's left in src/
ls -la src/
```

Expected: Only `lib.rs` should remain (if anything)

**Step 2: Delete src directory**

```bash
git rm -r src/
```

**Step 3: Commit**

```bash
git commit -m "chore: remove old src directory after workspace migration"
```

---

## Task 10: Run Tests and Verify

**Files:**
- Modify: `tests/wasm.rs` (if it exists, update imports)

**Step 1: Check for test files**

```bash
ls tests/
```

**Step 2: Update test imports if tests exist**

If `tests/wasm.rs` exists, update imports:

```rust
// OLD:
use fractalwonder::*;

// NEW:
use fractalwonder_ui::*;
```

**Step 3: Run workspace tests**

```bash
cargo test --workspace
```

Expected: All tests pass

**Step 4: Run WASM tests**

```bash
wasm-pack test --headless --chrome --workspace
```

Expected: All tests pass (or skip if no WASM tests exist yet)

**Step 5: Commit test updates**

```bash
git add tests/
git commit -m "test: update test imports for workspace structure"
```

---

## Task 11: Run Format and Lint

**Files:**
- All workspace files

**Step 1: Format code**

```bash
cargo fmt --all
```

**Step 2: Run Clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: No warnings or errors

**Step 3: Commit any formatting changes**

```bash
git add -A
git commit -m "chore: format code after workspace migration"
```

---

## Task 12: Build with Trunk and Verify App Works

**Files:**
- None (verification step)

**Step 1: Build with Trunk**

```bash
trunk build
```

Expected: Build succeeds, outputs to `dist/`

**Step 2: Serve with Trunk**

```bash
trunk serve
```

Expected: Server starts on `http://localhost:8080`

**Step 3: Manual verification**

Open `http://localhost:8080` in browser

Expected: App loads and displays (even if it's just placeholder content)

**Step 4: Stop server**

```
Ctrl+C
```

---

## Task 13: Final Verification Checklist

**Step 1: Run all checks**

```bash
# Format
cargo fmt --all --check

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build all crates
cargo build --workspace

# Build for WASM
cargo build -p fractalwonder-ui --target wasm32-unknown-unknown

# Tests
cargo test --workspace

# Trunk build
trunk build
```

Expected: All pass

**Step 2: Check git status**

```bash
git status
```

Expected: Working directory clean (or only expected untracked files)

**Step 3: Create final summary commit**

```bash
git add -A
git commit -m "chore: complete workspace restructure

Split single crate into three workspace crates:
- fractalwonder-core: shared types (no DOM)
- fractalwonder-compute: computation engine (no DOM)
- fractalwonder-ui: Leptos UI (DOM)

All tests pass, app builds and runs successfully.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Success Criteria

- [ ] Workspace builds successfully: `cargo build --workspace`
- [ ] UI WASM builds: `cargo build -p fractalwonder-ui --target wasm32-unknown-unknown`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Trunk serves app: `trunk serve` → app loads at localhost:8080
- [ ] No Clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Code formatted: `cargo fmt --all --check`
- [ ] Git history clean: meaningful commits for each step

## Notes

- If any build errors occur, carefully check import paths in the moved files
- The most common issue will be incorrect module paths after moving files
- Core types (Point, Rect, etc.) should import from `fractalwonder_core`
- Compute logic should import core via `fractalwonder_core::` or re-exports
- UI should import compute via `fractalwonder_compute::`
- Each commit should leave the workspace in a buildable state (though some tests may fail mid-migration)
