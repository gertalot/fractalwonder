# Iteration 6: Compute Crate & Renderer Trait Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create the compute layer foundation with proper separation of computation from colorization, formalizing the test pattern into the Renderer -> Colorizer pipeline.

**Architecture:** TestImage uses normalized viewport coordinates (f64). The compute layer produces `TestImageData` (boolean flags), and the UI colorizer maps that to RGBA. This separation allows future renderers (Mandelbrot) to use BigFloat coordinates while sharing the same pipeline structure.

**Tech Stack:** Rust, fractalwonder-core (types), fractalwonder-compute (new crate), fractalwonder-ui (colorizers)

**Reference:** See `docs/plans/2025-01-24-iteration-6-compute-crate-design.md` for design decisions.

---

## Task 1: Add TestImageData to Core

**Files:**
- Create: `fractalwonder-core/src/compute_data.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1.1: Create compute_data.rs with TestImageData struct**

```rust
// fractalwonder-core/src/compute_data.rs

/// Data computed for a test image pixel.
/// All fields are bools derived from normalized coordinate comparisons.
#[derive(Clone, Debug, PartialEq)]
pub struct TestImageData {
    pub is_on_origin: bool,
    pub is_on_x_axis: bool,
    pub is_on_y_axis: bool,
    pub is_on_major_tick_x: bool,
    pub is_on_medium_tick_x: bool,
    pub is_on_minor_tick_x: bool,
    pub is_on_major_tick_y: bool,
    pub is_on_medium_tick_y: bool,
    pub is_on_minor_tick_y: bool,
    pub is_light_cell: bool,
}

impl Default for TestImageData {
    fn default() -> Self {
        Self {
            is_on_origin: false,
            is_on_x_axis: false,
            is_on_y_axis: false,
            is_on_major_tick_x: false,
            is_on_medium_tick_x: false,
            is_on_minor_tick_x: false,
            is_on_major_tick_y: false,
            is_on_medium_tick_y: false,
            is_on_minor_tick_y: false,
            is_light_cell: true,
        }
    }
}

/// Unified enum for all compute results.
#[derive(Clone, Debug)]
pub enum ComputeData {
    TestImage(TestImageData),
    // Mandelbrot(MandelbrotData),  // iteration 7
}
```

**Step 1.2: Export from lib.rs**

Add to `fractalwonder-core/src/lib.rs`:
```rust
pub mod compute_data;

pub use compute_data::{ComputeData, TestImageData};
```

**Step 1.3: Verify compilation**

Run: `cargo check -p fractalwonder-core`
Expected: Compiles with no errors

**Step 1.4: Commit**

```bash
git add fractalwonder-core/src/compute_data.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add TestImageData and ComputeData types"
```

---

## Task 2: Create fractalwonder-compute Crate

**Files:**
- Create: `fractalwonder-compute/Cargo.toml`
- Create: `fractalwonder-compute/src/lib.rs`
- Modify: `Cargo.toml` (workspace)

**Step 2.1: Create crate directory**

```bash
mkdir -p fractalwonder-compute/src
```

**Step 2.2: Create Cargo.toml**

```toml
# fractalwonder-compute/Cargo.toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[dependencies]
fractalwonder-core = { path = "../fractalwonder-core" }
```

**Step 2.3: Create lib.rs with Renderer trait**

```rust
// fractalwonder-compute/src/lib.rs

use fractalwonder_core::Viewport;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}
```

**Step 2.4: Add to workspace**

Modify `Cargo.toml` (workspace root):
```toml
[workspace]
members = ["fractalwonder-core", "fractalwonder-compute", "fractalwonder-ui"]
```

Also add to `[workspace.dependencies]`:
```toml
fractalwonder-compute = { path = "./fractalwonder-compute" }
```

**Step 2.5: Verify compilation**

Run: `cargo check --workspace`
Expected: Compiles with no errors

**Step 2.6: Commit**

```bash
git add fractalwonder-compute/ Cargo.toml
git commit -m "feat(compute): create fractalwonder-compute crate with Renderer trait"
```

---

## Task 3: Implement TestImageRenderer

**Files:**
- Create: `fractalwonder-compute/src/test_image.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 3.1: Write failing test for TestImageRenderer**

Add test to `fractalwonder-compute/src/lib.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{BigFloat, Viewport};

    #[test]
    fn test_image_renderer_produces_correct_size() {
        let renderer = TestImageRenderer;
        let vp = Viewport::new(
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(4.0, 128),
            BigFloat::from_f64(4.0, 128),
        );
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn test_image_renderer_origin_detected() {
        let renderer = TestImageRenderer;
        // Viewport centered at origin
        let vp = Viewport::new(
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(4.0, 128),
            BigFloat::from_f64(4.0, 128),
        );
        let result = renderer.render(&vp, (100, 100));
        // Center pixel should be on origin
        let center_idx = 50 * 100 + 50;
        assert!(result[center_idx].is_on_origin);
    }
}
```

**Step 3.2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute`
Expected: FAIL - `TestImageRenderer` not found

**Step 3.3: Create test_image.rs**

```rust
// fractalwonder-compute/src/test_image.rs

use crate::Renderer;
use fractalwonder_core::{BigFloat, TestImageData, Viewport};

/// Renderer for the test image pattern using normalized viewport coordinates.
pub struct TestImageRenderer;

impl Renderer for TestImageRenderer {
    type Data = TestImageData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<TestImageData> {
        let (width, height) = canvas_size;
        let precision = viewport.precision_bits();

        // Pre-compute origin offset in normalized viewport coordinates.
        // Normalize by height to preserve aspect ratio.
        let zero = BigFloat::zero(precision);
        let origin_norm_x = zero.sub(&viewport.center.0).div(&viewport.height).to_f64();
        let origin_norm_y = zero.sub(&viewport.center.1).div(&viewport.height).to_f64();

        (0..height)
            .flat_map(|py| {
                (0..width).map(move |px| {
                    // Pixel to normalized coords
                    // Map [0, width) -> [-0.5*aspect, 0.5*aspect] and [0, height) -> [-0.5, 0.5]
                    let aspect = width as f64 / height as f64;
                    let norm_x = ((px as f64 / width as f64) - 0.5) * aspect;
                    let norm_y = (py as f64 / height as f64) - 0.5;

                    compute_test_image_data(norm_x, norm_y, origin_norm_x, origin_norm_y)
                })
            })
            .collect()
    }
}

/// Compute TestImageData for a single pixel using normalized coordinates.
fn compute_test_image_data(
    norm_x: f64,
    norm_y: f64,
    origin_norm_x: f64,
    origin_norm_y: f64,
) -> TestImageData {
    // Fixed spacing in normalized coordinates (viewport-relative)
    const MAJOR_SPACING: f64 = 0.2;
    const MEDIUM_SPACING: f64 = 0.1;
    const MINOR_SPACING: f64 = 0.02;

    const MAJOR_THRESHOLD: f64 = 0.004;
    const MEDIUM_THRESHOLD: f64 = 0.003;
    const MINOR_THRESHOLD: f64 = 0.002;

    const AXIS_THRESHOLD: f64 = 0.003;
    const ORIGIN_THRESHOLD: f64 = 0.02;

    const MAJOR_TICK_LENGTH: f64 = 0.04;
    const MEDIUM_TICK_LENGTH: f64 = 0.03;
    const MINOR_TICK_LENGTH: f64 = 0.02;

    // Position relative to absolute origin (0,0)
    let fx = norm_x - origin_norm_x;
    let fy = norm_y - origin_norm_y;

    let origin_visible = origin_norm_x.abs() < 1.0 && origin_norm_y.abs() < 1.0;
    let x_axis_visible = origin_norm_y.abs() < 1.0;
    let y_axis_visible = origin_norm_x.abs() < 1.0;

    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    let dist_to_x_axis = fy.abs();
    let dist_to_y_axis = fx.abs();

    let dist_to_major_x = distance_to_nearest_multiple(fx, MAJOR_SPACING);
    let dist_to_medium_x = distance_to_nearest_multiple(fx, MEDIUM_SPACING);
    let dist_to_minor_x = distance_to_nearest_multiple(fx, MINOR_SPACING);

    let dist_to_major_y = distance_to_nearest_multiple(fy, MAJOR_SPACING);
    let dist_to_medium_y = distance_to_nearest_multiple(fy, MEDIUM_SPACING);
    let dist_to_minor_y = distance_to_nearest_multiple(fy, MINOR_SPACING);

    TestImageData {
        is_on_origin: origin_visible && dist_to_origin < ORIGIN_THRESHOLD,
        is_on_x_axis: x_axis_visible && dist_to_x_axis < AXIS_THRESHOLD,
        is_on_y_axis: y_axis_visible && dist_to_y_axis < AXIS_THRESHOLD,
        is_on_major_tick_x: x_axis_visible
            && dist_to_major_x < MAJOR_THRESHOLD
            && dist_to_x_axis < MAJOR_TICK_LENGTH,
        is_on_medium_tick_x: x_axis_visible
            && dist_to_medium_x < MEDIUM_THRESHOLD
            && dist_to_x_axis < MEDIUM_TICK_LENGTH,
        is_on_minor_tick_x: x_axis_visible
            && dist_to_minor_x < MINOR_THRESHOLD
            && dist_to_x_axis < MINOR_TICK_LENGTH,
        is_on_major_tick_y: y_axis_visible
            && dist_to_major_y < MAJOR_THRESHOLD
            && dist_to_y_axis < MAJOR_TICK_LENGTH,
        is_on_medium_tick_y: y_axis_visible
            && dist_to_medium_y < MEDIUM_THRESHOLD
            && dist_to_y_axis < MEDIUM_TICK_LENGTH,
        is_on_minor_tick_y: y_axis_visible
            && dist_to_minor_y < MINOR_THRESHOLD
            && dist_to_y_axis < MINOR_TICK_LENGTH,
        is_light_cell: is_light_cell(fx, fy, MAJOR_SPACING),
    }
}

/// Calculate distance to nearest multiple of interval.
fn distance_to_nearest_multiple(value: f64, interval: f64) -> f64 {
    let remainder = value.rem_euclid(interval);
    remainder.min(interval - remainder)
}

/// Determine if a point is on a "light" or "dark" checkerboard cell.
fn is_light_cell(fx: f64, fy: f64, major_spacing: f64) -> bool {
    let cell_x = (fx / major_spacing).floor() as i64;
    let cell_y = (fy / major_spacing).floor() as i64;
    (cell_x + cell_y) % 2 == 0
}
```

**Step 3.4: Export TestImageRenderer from lib.rs**

Update `fractalwonder-compute/src/lib.rs`:
```rust
// fractalwonder-compute/src/lib.rs

mod test_image;

use fractalwonder_core::Viewport;

pub use test_image::TestImageRenderer;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{BigFloat, Viewport};

    #[test]
    fn test_image_renderer_produces_correct_size() {
        let renderer = TestImageRenderer;
        let vp = Viewport::new(
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(4.0, 128),
            BigFloat::from_f64(4.0, 128),
        );
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn test_image_renderer_origin_detected() {
        let renderer = TestImageRenderer;
        // Viewport centered at origin
        let vp = Viewport::new(
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(0.0, 128),
            BigFloat::from_f64(4.0, 128),
            BigFloat::from_f64(4.0, 128),
        );
        let result = renderer.render(&vp, (100, 100));
        // Center pixel should be on origin
        let center_idx = 50 * 100 + 50;
        assert!(result[center_idx].is_on_origin);
    }
}
```

**Step 3.5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute`
Expected: 2 tests pass

**Step 3.6: Commit**

```bash
git add fractalwonder-compute/src/
git commit -m "feat(compute): implement TestImageRenderer"
```

---

## Task 4: Create Colorizer in UI

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/mod.rs`
- Create: `fractalwonder-ui/src/rendering/colorizers/test_image.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 4.1: Create colorizers directory**

```bash
mkdir -p fractalwonder-ui/src/rendering/colorizers
```

**Step 4.2: Create test_image colorizer**

```rust
// fractalwonder-ui/src/rendering/colorizers/test_image.rs

use fractalwonder_core::TestImageData;

pub const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255];
pub const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255];
pub const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255];
pub const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
pub const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
pub const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255];
pub const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255];

/// Default colorizer for TestImageData.
pub fn colorize(data: &TestImageData) -> [u8; 4] {
    if data.is_on_origin {
        return ORIGIN_COLOR;
    }
    if data.is_on_major_tick_x || data.is_on_major_tick_y {
        return MAJOR_TICK_COLOR;
    }
    if data.is_on_medium_tick_x || data.is_on_medium_tick_y {
        return MEDIUM_TICK_COLOR;
    }
    if data.is_on_minor_tick_x || data.is_on_minor_tick_y {
        return MINOR_TICK_COLOR;
    }
    if data.is_on_x_axis || data.is_on_y_axis {
        return AXIS_COLOR;
    }
    if data.is_light_cell {
        BACKGROUND_LIGHT
    } else {
        BACKGROUND_DARK
    }
}
```

**Step 4.3: Create colorizers/mod.rs**

```rust
// fractalwonder-ui/src/rendering/colorizers/mod.rs

pub mod test_image;

pub use test_image::colorize as colorize_test_image;
```

**Step 4.4: Update rendering/mod.rs**

```rust
// fractalwonder-ui/src/rendering/mod.rs

pub mod colorizers;
mod test_pattern;

pub use colorizers::colorize_test_image;
pub use test_pattern::*;
```

**Step 4.5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles with no errors

**Step 4.6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(ui): add TestImageData colorizer"
```

---

## Task 5: Update UI Crate Dependencies

**Files:**
- Modify: `fractalwonder-ui/Cargo.toml`

**Step 5.1: Add fractalwonder-compute dependency**

Add to `[dependencies]` in `fractalwonder-ui/Cargo.toml`:
```toml
fractalwonder-compute = { workspace = true }
```

**Step 5.2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles with no errors

**Step 5.3: Commit**

```bash
git add fractalwonder-ui/Cargo.toml
git commit -m "chore(ui): add fractalwonder-compute dependency"
```

---

## Task 6: Update InteractiveCanvas to Use Compute Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 6.1: Update imports**

Replace import line:
```rust
use crate::rendering::test_pattern_color_normalized;
```
With:
```rust
use crate::rendering::colorize_test_image;
use fractalwonder_compute::{Renderer, TestImageRenderer};
```

**Step 6.2: Update render loop**

Replace the render effect (lines 72-134) with:
```rust
    // Render effect - redraws when viewport changes
    create_effect(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            return;
        }

        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        let (width, height) = size;

        // Use compute pipeline: Renderer -> Colorizer
        let renderer = TestImageRenderer;
        let computed_data = renderer.render(&vp, size);

        // Create pixel buffer
        let mut data = vec![0u8; (width * height * 4) as usize];

        for (i, pixel_data) in computed_data.iter().enumerate() {
            let color = colorize_test_image(pixel_data);
            let idx = i * 4;
            data[idx] = color[0];
            data[idx + 1] = color[1];
            data[idx + 2] = color[2];
            data[idx + 3] = color[3];
        }

        // Draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
            .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });
```

**Step 6.3: Remove unused imports**

Remove these imports that are no longer needed:
```rust
use fractalwonder_core::pixel_to_fractal;
```

**Step 6.4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles with no errors (may have unused import warnings)

**Step 6.5: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): use compute pipeline in InteractiveCanvas"
```

---

## Task 7: Visual Verification

**Step 7.1: Run development server**

Ensure `trunk serve` is running on http://localhost:8080

**Step 7.2: Browser verification**

Open http://localhost:8080 and verify:
- [ ] See checkerboard pattern with visible origin marker (red dot)
- [ ] Grid lines at axes (x=0, y=0)
- [ ] Major/medium/minor tick marks on axes
- [ ] Pan: drag canvas, pattern moves smoothly, re-renders at new position
- [ ] Zoom: scroll wheel scales pattern around cursor

**Step 7.3: Compare with previous behavior**

The visual output should match the previous iteration exactly:
- Same colors
- Same tick spacing
- Same origin marker size
- Same pan/zoom behavior

---

## Task 8: Clean Up Test Pattern

**Files:**
- Modify: `fractalwonder-ui/src/rendering/test_pattern.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 8.1: Keep only constants and utility functions in test_pattern.rs**

The functions `test_pattern_color_normalized` and `test_pattern_color` are no longer used by the UI. However, keep the file for now as reference. The colorizer uses its own constants.

Update `fractalwonder-ui/src/rendering/mod.rs` to not export test_pattern items that are now in colorizers:
```rust
// fractalwonder-ui/src/rendering/mod.rs

pub mod colorizers;
mod test_pattern;

pub use colorizers::colorize_test_image;
// Remove: pub use test_pattern::*;
// Only export what's still needed for tests or reference
pub use test_pattern::{calculate_tick_params, calculate_tick_params_from_log2, TickParams};
```

**Step 8.2: Verify tests still pass**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 8.3: Commit**

```bash
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "refactor(ui): clean up test_pattern exports after compute migration"
```

---

## Task 9: Run Full Test Suite

**Step 9.1: Run all quality checks**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All commands complete successfully with no errors or warnings.

**Step 9.2: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

After completing all tasks:

1. `fractalwonder-core` has `TestImageData` and `ComputeData` types
2. `fractalwonder-compute` crate exists with `Renderer` trait and `TestImageRenderer`
3. `fractalwonder-ui` has colorizer for `TestImageData`
4. `InteractiveCanvas` uses the Renderer -> Colorizer pipeline
5. Visual output matches previous iteration exactly
6. All tests pass, code is clean

This sets up the architecture for Iteration 7 (MandelbrotRenderer) which will add `MandelbrotData` to `ComputeData` and implement `Renderer` with BigFloat coordinates.
