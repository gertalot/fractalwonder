# Workspace Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Separate codebase by DOM dependencies to enable Web Workers

**Architecture:** Create Cargo workspace with three crates: fractalwonder-core (shared types, no DOM), fractalwonder-compute (computation engine, no DOM), fractalwonder-ui (presentation layer with DOM). Dependency chain: ui → compute → core.

**Tech Stack:** Rust 1.80+, Cargo workspaces, Leptos 0.6, dashu (arbitrary precision)

---

## Task 1: Create Workspace Root Manifest

**Files:**
- Modify: `Cargo.toml` (convert to workspace manifest)

**Step 1: Back up current Cargo.toml**

```bash
cp Cargo.toml Cargo.toml.backup
```

**Step 2: Replace root Cargo.toml with workspace manifest**

```toml
[workspace]
members = [
    "fractalwonder-ui",
    "fractalwonder-compute",
    "fractalwonder-core",
]
resolver = "2"

[workspace.dependencies]
# Shared core crate
fractalwonder-core = { path = "./fractalwonder-core" }
fractalwonder-compute = { path = "./fractalwonder-compute" }

# Arbitrary precision
dashu = "0.4"
dashu-float = "0.4"

# WASM/JS bindings
wasm-bindgen = "0.2"
js-sys = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
console_error_panic_hook = "0.1"
console_log = "1.0"
dyn-clone = "1.0.20"

# Testing
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

**Step 3: Verify workspace manifest syntax**

Run: `cargo metadata --no-deps --format-version 1 > /dev/null 2>&1 && echo "✓ Valid" || echo "✗ Invalid"`
Expected: `✓ Valid` (workspace parses correctly)

**Step 4: Commit workspace manifest**

```bash
git add Cargo.toml
git commit -m "chore: convert to Cargo workspace structure"
```

---

## Task 2: Create fractalwonder-core Crate (Shared Types)

**Files:**
- Create: `fractalwonder-core/Cargo.toml`
- Create: `fractalwonder-core/src/lib.rs`
- Create: `fractalwonder-core/src/points.rs`
- Create: `fractalwonder-core/src/viewport.rs`
- Create: `fractalwonder-core/src/pixel_rect.rs`
- Create: `fractalwonder-core/src/numeric.rs`
- Create: `fractalwonder-core/src/transforms.rs`
- Create: `fractalwonder-core/src/app_data.rs`

**Step 1: Create fractalwonder-core directory and Cargo.toml**

```bash
mkdir -p fractalwonder-core/src
```

Create `fractalwonder-core/Cargo.toml`:

```toml
[package]
name = "fractalwonder-core"
version = "0.1.0"
edition = "2021"

[dependencies]
dashu.workspace = true
dashu-float.workspace = true
serde.workspace = true
dyn-clone.workspace = true
```

**Step 2: Copy shared type files from src/rendering to fractalwonder-core/src**

```bash
cp src/rendering/points.rs fractalwonder-core/src/
cp src/rendering/viewport.rs fractalwonder-core/src/
cp src/rendering/pixel_rect.rs fractalwonder-core/src/
cp src/rendering/numeric.rs fractalwonder-core/src/
cp src/rendering/transforms.rs fractalwonder-core/src/
cp src/rendering/app_data.rs fractalwonder-core/src/
```

**Step 3: Create fractalwonder-core/src/lib.rs**

```rust
pub mod app_data;
pub mod numeric;
pub mod pixel_rect;
pub mod points;
pub mod transforms;
pub mod viewport;

pub use app_data::{AppData, TestImageData};
pub use numeric::{BigFloat, ToF64};
pub use pixel_rect::PixelRect;
pub use points::{Point, Rect};
pub use transforms::{
    apply_pixel_transform_to_viewport, calculate_aspect_ratio, calculate_visible_bounds,
    image_to_pixel, pan_viewport, pixel_to_image, zoom_viewport_at_point,
};
pub use viewport::Viewport;
```

**Step 4: Update imports in copied files to use crate-relative paths**

In `fractalwonder-core/src/app_data.rs`, replace:
```rust
use crate::rendering::computers::mandelbrot::MandelbrotData;
```

With (temporarily comment out until we create compute crate):
```rust
// Note: MandelbrotData will be defined in fractalwonder-compute
// For now, define it here temporarily to allow core crate to compile

/// Data computed by MandelbrotRenderer
#[derive(Clone, Copy, Debug)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
}
```

In `fractalwonder-core/src/transforms.rs`, replace:
```rust
use crate::rendering::{points::{Point, Rect}, viewport::Viewport};
```

With:
```rust
use crate::points::{Point, Rect};
use crate::viewport::Viewport;
```

In `fractalwonder-core/src/numeric.rs`, no changes needed (self-contained).

In `fractalwonder-core/src/points.rs`, replace:
```rust
use crate::rendering::numeric::ToF64;
```

With:
```rust
use crate::numeric::ToF64;
```

In `fractalwonder-core/src/pixel_rect.rs`, no changes needed (self-contained).

In `fractalwonder-core/src/viewport.rs`, replace:
```rust
use crate::rendering::points::Point;
```

With:
```rust
use crate::points::Point;
```

**Step 5: Build fractalwonder-core to verify it compiles**

Run: `cargo build -p fractalwonder-core`
Expected: `Finished` with no errors

**Step 6: Commit fractalwonder-core**

```bash
git add fractalwonder-core/
git commit -m "feat: create fractalwonder-core with shared types"
```

---

## Task 3: Create fractalwonder-compute Crate (Computation Engine)

**Files:**
- Create: `fractalwonder-compute/Cargo.toml`
- Create: `fractalwonder-compute/src/lib.rs`
- Create: `fractalwonder-compute/src/` (all computation files from src/rendering/)

**Step 1: Create fractalwonder-compute directory and Cargo.toml**

```bash
mkdir -p fractalwonder-compute/src
```

Create `fractalwonder-compute/Cargo.toml`:

```toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-core.workspace = true
dashu.workspace = true
dashu-float.workspace = true
serde.workspace = true
dyn-clone.workspace = true
```

**Step 2: Copy computation files from src/rendering to fractalwonder-compute/src**

```bash
# Copy individual files (excluding DOM-dependent ones)
cp src/rendering/adaptive_mandelbrot_renderer.rs fractalwonder-compute/src/
cp src/rendering/app_data_renderer.rs fractalwonder-compute/src/
cp src/rendering/pixel_renderer.rs fractalwonder-compute/src/
cp src/rendering/point_compute.rs fractalwonder-compute/src/
cp src/rendering/precision.rs fractalwonder-compute/src/
cp src/rendering/render_config.rs fractalwonder-compute/src/
cp src/rendering/renderer_info.rs fractalwonder-compute/src/
cp src/rendering/renderer_trait.rs fractalwonder-compute/src/

# Copy computers directory
cp -r src/rendering/computers fractalwonder-compute/src/
```

**Step 3: Create fractalwonder-compute/src/lib.rs**

```rust
pub mod adaptive_mandelbrot_renderer;
pub mod app_data_renderer;
pub mod computers;
pub mod pixel_renderer;
pub mod point_compute;
pub mod precision;
pub mod render_config;
pub mod renderer_info;
pub mod renderer_trait;

pub use adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
pub use app_data_renderer::AppDataRenderer;
pub use computers::{MandelbrotComputer, TestImageComputer};
pub use pixel_renderer::PixelRenderer;
pub use point_compute::ImagePointComputer;
pub use precision::PrecisionCalculator;
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
pub use renderer_info::RendererInfo;
pub use renderer_trait::Renderer;

// Re-export core types for convenience
pub use fractalwonder_core::*;
```

**Step 4: Update imports in all fractalwonder-compute files**

This is tedious but critical. In each file in `fractalwonder-compute/src/`:

Replace:
```rust
use crate::rendering::*;
```

With:
```rust
use fractalwonder_core::*;
use crate::*;
```

Replace:
```rust
use crate::rendering::something::Thing;
```

With:
```rust
use crate::something::Thing;
```

Or if it's a core type:
```rust
use fractalwonder_core::Thing;
```

**Specific file fixes:**

In `fractalwonder-compute/src/renderer_trait.rs`:
```rust
use fractalwonder_core::{Point, Rect, Viewport, PixelRect, AppData};
```

In `fractalwonder-compute/src/point_compute.rs`:
```rust
use fractalwonder_core::Point;
```

In `fractalwonder-compute/src/pixel_renderer.rs`:
```rust
use crate::point_compute::ImagePointComputer;
use crate::renderer_trait::Renderer;
use fractalwonder_core::{Point, Rect, Viewport, PixelRect, AppData};
```

In `fractalwonder-compute/src/adaptive_mandelbrot_renderer.rs`:
```rust
use crate::renderer_trait::Renderer;
use fractalwonder_core::{Point, Rect, Viewport, PixelRect, AppData, BigFloat, ToF64};
use crate::computers::MandelbrotComputer;
```

In `fractalwonder-compute/src/app_data_renderer.rs`:
```rust
use crate::renderer_trait::Renderer;
use fractalwonder_core::{Rect, Viewport, PixelRect, AppData};
```

In `fractalwonder-compute/src/precision.rs`:
```rust
use fractalwonder_core::BigFloat;
```

In `fractalwonder-compute/src/render_config.rs`:
```rust
use serde::{Deserialize, Serialize};
```

In `fractalwonder-compute/src/computers/mod.rs`:
```rust
pub mod mandelbrot;
pub mod test_image;

pub use mandelbrot::{MandelbrotComputer, MandelbrotData};
pub use test_image::TestImageComputer;
```

In `fractalwonder-compute/src/computers/mandelbrot.rs`:
```rust
use crate::point_compute::ImagePointComputer;
use fractalwonder_core::{Point, Viewport, ToF64};
use serde::{Deserialize, Serialize};
```

In `fractalwonder-compute/src/computers/test_image.rs`:
```rust
use crate::point_compute::ImagePointComputer;
use fractalwonder_core::{Point, Viewport, TestImageData};
```

**Step 5: Move MandelbrotData from core back to compute**

In `fractalwonder-core/src/app_data.rs`, remove the temporary MandelbrotData definition and restore:

```rust
use fractalwonder_compute::computers::MandelbrotData;

/// Unified data type for all renderer implementations
///
/// Each renderer wraps its specific data type in this enum to enable
/// runtime polymorphism via trait objects.
#[derive(Clone, Debug)]
pub enum AppData {
    TestImageData(TestImageData),
    MandelbrotData(MandelbrotData),
}

impl Default for AppData {
    fn default() -> Self {
        // Default to black pixel (0 iterations, not escaped)
        AppData::MandelbrotData(MandelbrotData {
            iterations: 0,
            escaped: false,
        })
    }
}

/// Data computed by TestImageRenderer
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

impl TestImageData {
    pub fn new(checkerboard: bool, circle_distance: f64) -> Self {
        Self {
            checkerboard,
            circle_distance,
        }
    }
}
```

Wait - this creates a circular dependency! Core needs MandelbrotData from compute, but compute depends on core. Let me fix this properly:

**Better approach:** Keep MandelbrotData in core, move it from computers/mandelbrot.rs to core/app_data.rs.

In `fractalwonder-core/src/app_data.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Data computed by MandelbrotRenderer
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
}

/// Unified data type for all renderer implementations
///
/// Each renderer wraps its specific data type in this enum to enable
/// runtime polymorphism via trait objects.
#[derive(Clone, Debug)]
pub enum AppData {
    TestImageData(TestImageData),
    MandelbrotData(MandelbrotData),
}

impl Default for AppData {
    fn default() -> Self {
        // Default to black pixel (0 iterations, not escaped)
        AppData::MandelbrotData(MandelbrotData {
            iterations: 0,
            escaped: false,
        })
    }
}

/// Data computed by TestImageRenderer
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

impl TestImageData {
    pub fn new(checkerboard: bool, circle_distance: f64) -> Self {
        Self {
            checkerboard,
            circle_distance,
        }
    }
}
```

And update `fractalwonder-core/src/lib.rs`:

```rust
pub mod app_data;
pub mod numeric;
pub mod pixel_rect;
pub mod points;
pub mod transforms;
pub mod viewport;

pub use app_data::{AppData, MandelbrotData, TestImageData};
pub use numeric::{BigFloat, ToF64};
pub use pixel_rect::PixelRect;
pub use points::{Point, Rect};
pub use transforms::{
    apply_pixel_transform_to_viewport, calculate_aspect_ratio, calculate_visible_bounds,
    image_to_pixel, pan_viewport, pixel_to_image, zoom_viewport_at_point,
};
pub use viewport::Viewport;
```

In `fractalwonder-compute/src/computers/mandelbrot.rs`, remove MandelbrotData definition and import from core:

```rust
use crate::point_compute::ImagePointComputer;
use fractalwonder_core::{Point, Viewport, ToF64, MandelbrotData};
```

In `fractalwonder-compute/src/computers/mod.rs`:

```rust
pub mod mandelbrot;
pub mod test_image;

pub use mandelbrot::MandelbrotComputer;
pub use test_image::TestImageComputer;
// Re-export MandelbrotData from core for convenience
pub use fractalwonder_core::MandelbrotData;
```

**Step 6: Build fractalwonder-compute to verify it compiles**

Run: `cargo build -p fractalwonder-compute`
Expected: `Finished` with no errors (will fail initially, need to fix import errors iteratively)

**Step 7: Fix any remaining compilation errors**

Run: `cargo build -p fractalwonder-compute 2>&1 | head -20`
Expected: Read error messages, fix imports, repeat until clean build

**Step 8: Commit fractalwonder-compute**

```bash
git add fractalwonder-compute/
git add fractalwonder-core/  # Updated app_data.rs
git commit -m "feat: create fractalwonder-compute with computation engine"
```

---

## Task 4: Create fractalwonder-ui Crate (Presentation Layer)

**Files:**
- Create: `fractalwonder-ui/Cargo.toml`
- Create: `fractalwonder-ui/src/lib.rs`
- Create: `fractalwonder-ui/src/` (all UI files from src/)

**Step 1: Create fractalwonder-ui directory and Cargo.toml**

```bash
mkdir -p fractalwonder-ui/src
```

Create `fractalwonder-ui/Cargo.toml`:

```toml
[package]
name = "fractalwonder-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-core.workspace = true
fractalwonder-compute.workspace = true
leptos = { version = "0.6", features = ["csr"] }
wasm-bindgen.workspace = true
wasm-bindgen-futures = "0.4"
console_error_panic_hook.workspace = true
console_log.workspace = true
leptos-use = "0.13"
js-sys.workspace = true
serde.workspace = true
serde_json.workspace = true

[dependencies.web-sys]
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
```

**Step 2: Copy UI files from src/ to fractalwonder-ui/src/**

```bash
# Copy main app and lib
cp src/lib.rs fractalwonder-ui/src/
cp src/app.rs fractalwonder-ui/src/

# Copy UI directories
cp -r src/components fractalwonder-ui/src/
cp -r src/hooks fractalwonder-ui/src/
cp -r src/state fractalwonder-ui/src/

# Create rendering module directory
mkdir -p fractalwonder-ui/src/rendering

# Copy DOM-dependent rendering files
cp src/rendering/canvas_renderer.rs fractalwonder-ui/src/rendering/
cp src/rendering/canvas_utils.rs fractalwonder-ui/src/rendering/
cp src/rendering/tiling_canvas_renderer.rs fractalwonder-ui/src/rendering/
cp src/rendering/colorizers.rs fractalwonder-ui/src/rendering/
```

**Step 3: Create fractalwonder-ui/src/rendering/mod.rs**

```rust
pub mod canvas_renderer;
pub mod canvas_utils;
pub mod colorizers;
pub mod tiling_canvas_renderer;

pub use canvas_renderer::CanvasRenderer;
pub use canvas_utils::render_with_viewport;
pub use colorizers::{test_image_default_colorizer, Colorizer};
pub use tiling_canvas_renderer::TilingCanvasRenderer;

// Re-export commonly used types from core and compute
pub use fractalwonder_compute::*;
pub use fractalwonder_core::*;
```

**Step 4: Update fractalwonder-ui/src/lib.rs**

```rust
mod app;
mod components;
pub mod hooks;
pub mod rendering;
pub mod state;

use leptos::*;
use wasm_bindgen::prelude::*;

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

**Step 5: Update imports in fractalwonder-ui files**

In `fractalwonder-ui/src/app.rs`:

Replace:
```rust
use crate::rendering::*;
```

With:
```rust
use crate::rendering::*;
use fractalwonder_core::*;
use fractalwonder_compute::*;
```

In `fractalwonder-ui/src/rendering/canvas_utils.rs`:

Replace:
```rust
use crate::rendering::{renderer_trait::Renderer, viewport::Viewport, Colorizer, PixelRect};
```

With:
```rust
use crate::rendering::Colorizer;
use fractalwonder_compute::Renderer;
use fractalwonder_core::{Viewport, PixelRect};
```

In `fractalwonder-ui/src/rendering/canvas_renderer.rs`:

Replace:
```rust
use crate::rendering::{points::Rect, renderer_trait::Renderer, viewport::Viewport, Colorizer};
```

With:
```rust
use crate::rendering::Colorizer;
use fractalwonder_compute::Renderer;
use fractalwonder_core::{Rect, Viewport};
```

In `fractalwonder-ui/src/rendering/tiling_canvas_renderer.rs`:

Replace all `crate::rendering::` imports with appropriate imports from `fractalwonder_core::` and `fractalwonder_compute::`.

Example:
```rust
use crate::rendering::{
    canvas_renderer::CanvasRenderer, renderer_trait::Renderer, viewport::Viewport, Colorizer,
};
```

Becomes:
```rust
use crate::rendering::{CanvasRenderer, Colorizer};
use fractalwonder_compute::Renderer;
use fractalwonder_core::Viewport;
```

In `fractalwonder-ui/src/rendering/colorizers.rs`:

Replace:
```rust
use super::app_data::{AppData, TestImageData};
#[cfg(test)]
use super::computers::mandelbrot::MandelbrotData;
```

With:
```rust
use fractalwonder_core::{AppData, TestImageData};
#[cfg(test)]
use fractalwonder_core::MandelbrotData;
```

In all component files (`fractalwonder-ui/src/components/*.rs`):

Replace:
```rust
use crate::rendering::*;
```

With:
```rust
use crate::rendering::*;
use fractalwonder_core::*;
use fractalwonder_compute::*;
```

In all hook files (`fractalwonder-ui/src/hooks/*.rs`):

Same pattern - replace `crate::rendering` with explicit imports from `fractalwonder_core` and `fractalwonder_compute`.

In all state files (`fractalwonder-ui/src/state/*.rs`):

Same pattern - replace `crate::rendering` with explicit imports from `fractalwonder_core` and `fractalwonder_compute`.

**Step 6: Build fractalwonder-ui to verify it compiles**

Run: `cargo build -p fractalwonder-ui`
Expected: `Finished` with no errors (will fail initially, need to fix import errors iteratively)

**Step 7: Fix any remaining compilation errors**

Run: `cargo build -p fractalwonder-ui 2>&1 | head -30`
Expected: Read error messages, fix imports, repeat until clean build

**Step 8: Commit fractalwonder-ui**

```bash
git add fractalwonder-ui/
git commit -m "feat: create fractalwonder-ui with presentation layer"
```

---

## Task 5: Update index.html to Reference UI Crate

**Files:**
- Modify: `index.html`

**Step 1: Update index.html to reference fractalwonder-ui**

Replace:
```html
<link data-trunk rel="rust" data-wasm-opt="z" />
```

With:
```html
<link data-trunk rel="rust" data-wasm-opt="z" href="./fractalwonder-ui/Cargo.toml" />
```

Full updated `index.html`:

```html
<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Fractal Wonder</title>
  <link data-trunk rel="tailwind-css" href="input.css" />
  <link data-trunk rel="rust" data-wasm-opt="z" href="./fractalwonder-ui/Cargo.toml" />
</head>

<body class="m-0 p-0 overflow-hidden">
  <div id="app"></div>
  <script type="module">
    window.addEventListener('TrunkApplicationStarted', () => {
      window.wasmBindings.hydrate();
    });
  </script>
</body>

</html>
```

**Step 2: Commit index.html update**

```bash
git add index.html
git commit -m "chore: update index.html to reference fractalwonder-ui crate"
```

---

## Task 6: Update Tests to Use New Crate Structure

**Files:**
- Modify: `tests/arbitrary_precision.rs`

**Step 1: Update test imports**

In `tests/arbitrary_precision.rs`:

Replace:
```rust
use fractalwonder::rendering::*;
```

With:
```rust
use fractalwonder_core::*;
use fractalwonder_compute::*;
```

**Step 2: Run tests to verify they work**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 3: Fix any test failures**

If tests fail, read error messages and update imports or test code accordingly.

**Step 4: Commit test updates**

```bash
git add tests/
git commit -m "test: update tests to use workspace crate structure"
```

---

## Task 7: Verify Build and Development Workflow

**Files:**
- None (verification only)

**Step 1: Clean build to start fresh**

Run: `cargo clean`
Expected: Removes target/ directory

**Step 2: Build entire workspace**

Run: `cargo build --workspace`
Expected: All three crates build successfully

**Step 3: Run all workspace tests**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 4: Run Clippy on workspace**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 5: Format workspace code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 6: Build WASM in release mode**

Run: `trunk build --release`
Expected: Builds successfully, outputs to dist/

**Step 7: Test development server**

Run: `trunk serve` (in background or separate terminal)
Then visit: `http://localhost:8080`
Expected: App loads and works identically to before restructure

**Step 8: Commit verification**

```bash
git add .
git commit -m "chore: verify workspace builds and runs correctly"
```

---

## Task 8: Clean Up Old src/ Directory

**Files:**
- Delete: `src/` directory
- Delete: `Cargo.toml.backup`

**Step 1: Verify workspace is fully functional**

Manually test app in browser:
- Pan and zoom works
- Rendering works
- No console errors

**Step 2: Remove old src/ directory**

```bash
rm -rf src/
```

**Step 3: Remove backup Cargo.toml**

```bash
rm Cargo.toml.backup
```

**Step 4: Verify build still works**

Run: `cargo build --workspace`
Expected: Builds successfully (proves old src/ not needed)

**Step 5: Commit cleanup**

```bash
git add .
git commit -m "chore: remove old src/ directory after workspace migration"
```

---

## Task 9: Update Documentation

**Files:**
- Modify: `README.md`
- Create: `docs/architecture/workspace-structure.md`

**Step 1: Update README.md with workspace structure**

Add section after "Key Technologies":

```markdown
## Project Structure

Fractal Wonder uses a Cargo workspace with three crates:

- **fractalwonder-core**: Shared types and utilities (no DOM dependencies)
  - Geometric types: `Point`, `Rect`, `Viewport`, `PixelRect`
  - Numeric types: `BigFloat`, `ToF64`
  - Coordinate transforms
  - Data types: `AppData`, `MandelbrotData`, `TestImageData`

- **fractalwonder-compute**: Computation engine (no DOM dependencies)
  - Renderer trait and implementations
  - Fractal computers (Mandelbrot, TestImage)
  - Pixel rendering logic
  - Designed to run in Web Workers

- **fractalwonder-ui**: UI/presentation layer (has DOM dependencies)
  - Leptos components and hooks
  - Canvas rendering utilities
  - Colorizers
  - Application state management

Dependency chain: `fractalwonder-ui` → `fractalwonder-compute` → `fractalwonder-core`

This separation enables future Web Worker parallelization for multi-core rendering.
```

**Step 2: Create workspace architecture documentation**

Create `docs/architecture/workspace-structure.md`:

```markdown
# Workspace Structure

## Overview

Fractal Wonder is organized as a Cargo workspace with three crates, separated by DOM dependencies to enable Web Worker parallelization.

## Crate Dependency Graph

```
fractalwonder-ui (DOM-dependent)
    ↓
fractalwonder-compute (pure computation, no DOM)
    ↓
fractalwonder-core (shared types, no DOM)
```

## fractalwonder-core

**Purpose:** Shared types used by both compute and UI layers.

**Key modules:**
- `points`: `Point<T>`, `Rect<T>` geometric primitives
- `viewport`: `Viewport<T>` for view transformations
- `pixel_rect`: `PixelRect` for pixel-space rectangles
- `numeric`: `BigFloat`, `ToF64` trait for arbitrary precision
- `transforms`: Coordinate transformation utilities
- `app_data`: `AppData` enum for renderer output data

**Dependencies:** Only `dashu`, `serde`, `dyn-clone` (no DOM)

## fractalwonder-compute

**Purpose:** Computation engine for fractal rendering.

**Key modules:**
- `renderer_trait`: `Renderer` trait for computation strategies
- `computers/`: Fractal algorithms (`MandelbrotComputer`, `TestImageComputer`)
- `pixel_renderer`: Maps pixels to computed data
- `adaptive_mandelbrot_renderer`: Adaptive precision rendering
- `render_config`: Configuration and color schemes
- `precision`: Arbitrary precision calculator

**Dependencies:** `fractalwonder-core` + `dashu` (no DOM)

**Design:** Can run in Web Workers for multi-core parallelism.

## fractalwonder-ui

**Purpose:** User interface and presentation layer.

**Key modules:**
- `app`: Main Leptos application
- `components/`: Reusable UI components
- `hooks/`: Custom Leptos hooks
- `state/`: Application state management
- `rendering/canvas_renderer`: Canvas rendering trait
- `rendering/tiling_canvas_renderer`: Progressive tiled rendering
- `rendering/canvas_utils`: Canvas rendering utilities
- `rendering/colorizers`: Data-to-RGBA color functions

**Dependencies:** `fractalwonder-compute`, `fractalwonder-core`, `leptos`, `web-sys` (has DOM)

**Design:** Main thread only, handles UI and canvas rendering.

## Why This Structure?

**Enables Web Workers:**
- Web Workers cannot access DOM
- `fractalwonder-compute` has no DOM dependencies → can run in workers
- `fractalwonder-ui` handles all DOM interaction on main thread

**Clean Separation:**
- Core types shared between layers
- Computation logic isolated from presentation
- Testable in isolation

**Future-Proof:**
- Workers can be added without refactoring computation code
- GPU acceleration can replace compute layer
- Core types remain stable
```

**Step 3: Commit documentation**

```bash
mkdir -p docs/architecture
git add README.md docs/architecture/workspace-structure.md
git commit -m "docs: document workspace structure and architecture"
```

---

## Task 10: Final Validation

**Files:**
- None (comprehensive validation)

**Step 1: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 2: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings

**Step 3: Format check**

Run: `cargo fmt --all -- --check`
Expected: All code formatted

**Step 4: Build release**

Run: `trunk build --release`
Expected: Successful build to dist/

**Step 5: Manual testing checklist**

Start dev server: `trunk serve`

Test in browser:
- [ ] App loads without errors
- [ ] Can pan viewport (click and drag)
- [ ] Can zoom in/out (scroll wheel)
- [ ] Rendering works correctly
- [ ] Switching renderers works (if UI supports it)
- [ ] Switching color schemes works (if UI supports it)
- [ ] No console errors
- [ ] Performance is identical to before

**Step 6: Create final commit**

```bash
git add .
git commit -m "feat: complete workspace restructure - ready for Web Workers"
```

**Step 7: Create annotated tag**

```bash
git tag -a v0.2.0-workspace -m "Workspace restructure complete - separated by DOM dependencies"
```

---

## Success Criteria

**All of these must be true:**

- [x] Workspace builds with `cargo build --workspace`
- [x] All tests pass with `cargo test --workspace`
- [x] No Clippy warnings
- [x] Code properly formatted
- [x] App loads and runs identically to before
- [x] All rendering features work (pan, zoom, render)
- [x] No console errors in browser
- [x] Documentation updated
- [x] Clean git history with logical commits

**Workspace structure:**
```
fractalwonder/
├── Cargo.toml (workspace manifest)
├── fractalwonder-core/ (shared types, no DOM)
├── fractalwonder-compute/ (computation, no DOM)
├── fractalwonder-ui/ (UI layer, has DOM)
├── tests/ (integration tests)
├── docs/
├── public/
└── index.html (references fractalwonder-ui)
```

**No old cruft remaining:**
- No `src/` directory at root
- No `Cargo.toml.backup`
- Clean workspace member list

---

## Next Steps After Completion

This workspace restructure (Iteration 1) enables:

**Iteration 2:** Progressive rendering with async tile scheduling
**Iteration 3:** Web Workers with wasm-bindgen-rayon
**Iteration 4:** Responsive cancellation
**Iteration 5:** Tile scheduling optimization
**Iteration 6:** Perturbation theory (single reference)
**Iteration 7:** Adaptive quadtree (multiple references)

Reference: `docs/multicore-plans/2025-11-17-progressive-parallel-rendering-design.md`
