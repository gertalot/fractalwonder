# Iteration 4: Viewport-Driven Rendering - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Interactive test pattern rendering with coordinate transforms, pan/zoom, and dynamic ruler.

**Architecture:** App owns viewport signal, InteractiveCanvas receives read-only signal + callback, renders test pattern using pixel_to_fractal transforms.

**Tech Stack:** Rust, Leptos 0.6, WebAssembly, fractalwonder-core BigFloat

---

## Task 1: Test Pattern Module - Tick Calculation

**Files:**
- Create: `fractalwonder-ui/src/rendering/mod.rs`
- Create: `fractalwonder-ui/src/rendering/test_pattern.rs`
- Modify: `fractalwonder-ui/src/lib.rs:1-10`

**Step 1: Write the failing test for tick spacing calculation**

Create `fractalwonder-ui/src/rendering/test_pattern.rs`:

```rust
/// Tick spacing parameters for the ruler test pattern.
/// All values derived from major_spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct TickParams {
    /// Major tick interval (e.g., 1.0 when viewport width ~4)
    pub major_spacing: f64,
    /// Medium tick interval (major / 2)
    pub medium_spacing: f64,
    /// Minor tick interval (major / 10)
    pub minor_spacing: f64,
    /// Threshold for detecting major ticks (major / 50)
    pub major_threshold: f64,
    /// Threshold for detecting medium ticks (major / 75)
    pub medium_threshold: f64,
    /// Threshold for detecting minor ticks (major / 100)
    pub minor_threshold: f64,
    /// Threshold for detecting axis lines (major / 100)
    pub axis_threshold: f64,
}

/// Calculate tick parameters from viewport width.
///
/// Uses log10 to find appropriate scale, then derives all parameters
/// from a single major_spacing value.
pub fn calculate_tick_params(viewport_width_f64: f64) -> TickParams {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_spacing_at_width_4() {
        let params = calculate_tick_params(4.0);
        assert!((params.major_spacing - 1.0).abs() < 0.001);
        assert!((params.medium_spacing - 0.5).abs() < 0.001);
        assert!((params.minor_spacing - 0.1).abs() < 0.001);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui tick_spacing_at_width_4 -- --nocapture`

Expected: FAIL with "not yet implemented"

**Step 3: Write minimal implementation**

Replace `todo!()` with:

```rust
pub fn calculate_tick_params(viewport_width_f64: f64) -> TickParams {
    let log_width = viewport_width_f64.log10();
    let major_exp = (log_width - 0.5).floor() as i32;
    let major_spacing = 10.0_f64.powi(major_exp);

    TickParams {
        major_spacing,
        medium_spacing: major_spacing / 2.0,
        minor_spacing: major_spacing / 10.0,
        major_threshold: major_spacing / 50.0,
        medium_threshold: major_spacing / 75.0,
        minor_threshold: major_spacing / 100.0,
        axis_threshold: major_spacing / 100.0,
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-ui tick_spacing_at_width_4 -- --nocapture`

Expected: PASS

**Step 5: Add module to lib.rs**

Add to `fractalwonder-ui/src/lib.rs` after existing mod declarations:

```rust
pub mod rendering;
```

Create `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod test_pattern;

pub use test_pattern::{calculate_tick_params, TickParams};
```

**Step 6: Verify build compiles**

Run: `cargo check -p fractalwonder-ui`

Expected: Success

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/ fractalwonder-ui/src/lib.rs
git commit -m "feat(ui): add tick spacing calculation for test pattern"
```

---

## Task 2: Test Pattern - Additional Tick Tests

**Files:**
- Modify: `fractalwonder-ui/src/rendering/test_pattern.rs`

**Step 1: Add more tick spacing tests**

Add to the tests module:

```rust
    #[test]
    fn tick_spacing_at_width_0_04() {
        let params = calculate_tick_params(0.04);
        assert!((params.major_spacing - 0.01).abs() < 0.0001);
    }

    #[test]
    fn tick_spacing_at_width_40() {
        let params = calculate_tick_params(40.0);
        assert!((params.major_spacing - 10.0).abs() < 0.001);
    }

    #[test]
    fn tick_spacing_at_width_400() {
        let params = calculate_tick_params(400.0);
        assert!((params.major_spacing - 100.0).abs() < 0.001);
    }

    #[test]
    fn tick_thresholds_proportional_to_spacing() {
        let params = calculate_tick_params(4.0);
        assert!((params.major_threshold - params.major_spacing / 50.0).abs() < 0.0001);
        assert!((params.axis_threshold - params.major_spacing / 100.0).abs() < 0.0001);
    }
```

**Step 2: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui tick_spacing -- --nocapture`

Expected: All PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/test_pattern.rs
git commit -m "test(ui): add comprehensive tick spacing tests"
```

---

## Task 3: Test Pattern - Color Functions

**Files:**
- Modify: `fractalwonder-ui/src/rendering/test_pattern.rs`

**Step 1: Add color constant definitions and distance function test**

Add above the tests module:

```rust
use fractalwonder_core::BigFloat;

/// Colors for the test pattern (RGBA)
pub const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255];  // Light grey
pub const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255];   // White
pub const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255];        // Dark grey
pub const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255];     // Darker grey
pub const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
pub const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
pub const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255];          // Red

/// Calculate distance to nearest multiple of interval.
/// Returns a value in [0, interval/2].
pub fn distance_to_nearest_multiple(value: f64, interval: f64) -> f64 {
    let remainder = value.rem_euclid(interval);
    remainder.min(interval - remainder)
}
```

Add test:

```rust
    #[test]
    fn distance_to_nearest_multiple_at_boundary() {
        assert!((distance_to_nearest_multiple(1.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(0.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(2.5, 1.0) - 0.5).abs() < 0.0001);
    }

    #[test]
    fn distance_to_nearest_multiple_negative_values() {
        assert!((distance_to_nearest_multiple(-1.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(-0.3, 1.0) - 0.3).abs() < 0.0001);
    }
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-ui distance_to_nearest -- --nocapture`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/test_pattern.rs
git commit -m "feat(ui): add color constants and distance helper for test pattern"
```

---

## Task 4: Test Pattern - Checkerboard Function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/test_pattern.rs`

**Step 1: Add checkerboard test**

Add function and test:

```rust
/// Determine if a point is on a "light" or "dark" checkerboard cell.
/// Cells are aligned to major tick grid.
pub fn is_light_cell(fx: f64, fy: f64, major_spacing: f64) -> bool {
    let cell_x = (fx / major_spacing).floor() as i64;
    let cell_y = (fy / major_spacing).floor() as i64;
    (cell_x + cell_y) % 2 == 0
}
```

Test:

```rust
    #[test]
    fn checkerboard_alternates_at_integer_boundaries() {
        // With major_spacing=1.0, cells at (0.5, 0.5) and (1.5, 0.5) should differ
        assert!(is_light_cell(0.5, 0.5, 1.0));
        assert!(!is_light_cell(1.5, 0.5, 1.0));
        assert!(!is_light_cell(0.5, 1.5, 1.0));
        assert!(is_light_cell(1.5, 1.5, 1.0));
    }

    #[test]
    fn checkerboard_works_with_negative_coords() {
        assert!(is_light_cell(-0.5, -0.5, 1.0));
        assert!(!is_light_cell(-1.5, -0.5, 1.0));
    }
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-ui checkerboard -- --nocapture`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/test_pattern.rs
git commit -m "feat(ui): add checkerboard cell detection"
```

---

## Task 5: Test Pattern - Main Color Function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/test_pattern.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Write failing test for test_pattern_color**

```rust
/// Compute the RGBA color for a pixel at fractal coordinates (fx, fy).
///
/// Renders:
/// 1. Checkerboard background aligned to major tick grid
/// 2. Axis lines at x=0 and y=0
/// 3. Tick marks at major/medium/minor intervals
/// 4. Origin marker at (0,0)
pub fn test_pattern_color(fx: f64, fy: f64, params: &TickParams) -> [u8; 4] {
    todo!()
}
```

Test:

```rust
    #[test]
    fn test_pattern_axis_detected_near_zero() {
        let params = calculate_tick_params(4.0);
        // Point very close to y=0 axis should be axis color (or tick color)
        let color = test_pattern_color(0.5, 0.001, &params);
        // Should NOT be background color
        assert_ne!(color, BACKGROUND_LIGHT);
        assert_ne!(color, BACKGROUND_DARK);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-ui test_pattern_axis -- --nocapture`

Expected: FAIL with "not yet implemented"

**Step 3: Write implementation**

```rust
pub fn test_pattern_color(fx: f64, fy: f64, params: &TickParams) -> [u8; 4] {
    // 1. Check for origin marker (highest priority)
    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    if dist_to_origin < params.major_threshold * 2.0 {
        return ORIGIN_COLOR;
    }

    // 2. Check for horizontal axis (y ≈ 0)
    let dist_to_x_axis = fy.abs();
    if dist_to_x_axis < params.axis_threshold {
        // Check for tick marks along x-axis
        let dist_to_major = distance_to_nearest_multiple(fx, params.major_spacing);
        if dist_to_major < params.major_threshold {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium = distance_to_nearest_multiple(fx, params.medium_spacing);
        if dist_to_medium < params.medium_threshold {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor = distance_to_nearest_multiple(fx, params.minor_spacing);
        if dist_to_minor < params.minor_threshold {
            return MINOR_TICK_COLOR;
        }
        return AXIS_COLOR;
    }

    // 3. Check for vertical axis (x ≈ 0)
    let dist_to_y_axis = fx.abs();
    if dist_to_y_axis < params.axis_threshold {
        // Check for tick marks along y-axis
        let dist_to_major = distance_to_nearest_multiple(fy, params.major_spacing);
        if dist_to_major < params.major_threshold {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium = distance_to_nearest_multiple(fy, params.medium_spacing);
        if dist_to_medium < params.medium_threshold {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor = distance_to_nearest_multiple(fy, params.minor_spacing);
        if dist_to_minor < params.minor_threshold {
            return MINOR_TICK_COLOR;
        }
        return AXIS_COLOR;
    }

    // 4. Checkerboard background
    if is_light_cell(fx, fy, params.major_spacing) {
        BACKGROUND_LIGHT
    } else {
        BACKGROUND_DARK
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui test_pattern -- --nocapture`

Expected: PASS

**Step 5: Export from mod.rs**

Update `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod test_pattern;

pub use test_pattern::{
    calculate_tick_params, distance_to_nearest_multiple, is_light_cell, test_pattern_color,
    TickParams, AXIS_COLOR, BACKGROUND_DARK, BACKGROUND_LIGHT, MAJOR_TICK_COLOR,
    MEDIUM_TICK_COLOR, MINOR_TICK_COLOR, ORIGIN_COLOR,
};
```

**Step 6: Run all tests and verify build**

Run: `cargo test -p fractalwonder-ui -- --nocapture && cargo check -p fractalwonder-ui`

Expected: All pass, build succeeds

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/
git commit -m "feat(ui): add test pattern color computation"
```

---

## Task 6: Update InteractiveCanvas Signature

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Update imports and component signature**

Replace the component definition with:

```rust
// fractalwonder-ui/src/components/interactive_canvas.rs
use fractalwonder_core::{pixel_to_fractal, Viewport};
use leptos::*;
use leptos_use::use_window_size;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::hooks::use_canvas_interaction;
use crate::rendering::{calculate_tick_params, test_pattern_color};

#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
    /// Callback fired when canvas dimensions change
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Reactive window size
    let window_size = use_window_size();

    // TODO: Wire up use_canvas_interaction hook
    // TODO: Add render effect

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
```

**Step 2: Verify build**

Run: `cargo check -p fractalwonder-ui`

Expected: May have errors due to App.rs not passing new props yet - this is expected

**Step 3: Commit partial progress**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "refactor(ui): update InteractiveCanvas signature for viewport-driven rendering"
```

---

## Task 7: Update App to Use New InteractiveCanvas

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update App to pass viewport and callback**

Replace `fractalwonder-ui/src/app.rs`:

```rust
// fractalwonder-ui/src/app.rs
use fractalwonder_core::{apply_pixel_transform_to_viewport, calculate_precision_bits, fit_viewport_to_canvas, Viewport};
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};
use crate::config::default_config;

#[component]
pub fn App() -> impl IntoView {
    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Current fractal configuration
    let (config, _set_config) = create_signal(default_config());

    // Viewport signal - now writable for interaction updates
    let (viewport, set_viewport) = create_signal(Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64));

    // Initialize viewport when canvas size becomes available
    create_effect(move |prev_size| {
        let size = canvas_size.get();
        let cfg = config.get();

        // Only initialize once when size becomes non-zero
        if size.0 == 0 || size.1 == 0 {
            return size;
        }

        // If this is the first time we have a valid size, initialize viewport
        if prev_size.map(|(w, h)| w == 0 || h == 0).unwrap_or(true) {
            let natural = cfg.default_viewport(64);
            let fitted = fit_viewport_to_canvas(&natural, size);
            let required_bits = calculate_precision_bits(&fitted, size);

            let final_viewport = if required_bits > fitted.precision_bits() {
                let natural_high_prec = cfg.default_viewport(required_bits);
                fit_viewport_to_canvas(&natural_high_prec, size)
            } else {
                fitted
            };

            set_viewport.set(final_viewport);
        }

        size
    });

    // Precision bits - derived from viewport and canvas
    let precision_bits = create_memo(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            64
        } else {
            calculate_precision_bits(&vp, size)
        }
    });

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    let on_viewport_change = Callback::new(move |new_vp: Viewport| {
        set_viewport.set(new_vp);
    });

    view! {
        <InteractiveCanvas
            viewport=viewport.into()
            on_viewport_change=on_viewport_change
            on_resize=on_resize
        />
        <UIPanel
            canvas_size=canvas_size.into()
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
        />
    }
}
```

**Step 2: Verify build**

Run: `cargo check -p fractalwonder-ui`

Expected: Success (InteractiveCanvas body is minimal/stub)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "refactor(ui): update App to pass viewport signal and callback"
```

---

## Task 8: Wire use_canvas_interaction Hook

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Add hook wiring**

Update InteractiveCanvas body:

```rust
#[component]
pub fn InteractiveCanvas(
    viewport: Signal<Viewport>,
    on_viewport_change: Callback<Viewport>,
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    let window_size = use_window_size();

    // Store canvas size for use in callbacks
    let canvas_size = create_rw_signal((0u32, 0u32));

    // Wire up interaction hook
    let _interaction = use_canvas_interaction(canvas_ref, move |transform| {
        let current_vp = viewport.get_untracked();
        let size = canvas_size.get_untracked();

        if size.0 > 0 && size.1 > 0 {
            let precision = current_vp.precision_bits();
            let new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform, size, precision);
            on_viewport_change.call(new_vp);
        }
    });

    // Effect to handle resize and initial render
    create_effect(move |_| {
        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let width = window_size.width.get() as u32;
        let height = window_size.height.get() as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Update canvas dimensions
        canvas.set_width(width);
        canvas.set_height(height);

        // Store for interaction callback
        canvas_size.set((width, height));

        // Notify parent of dimensions
        if let Some(callback) = on_resize {
            callback.call((width, height));
        }
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
```

**Step 2: Verify build**

Run: `cargo check -p fractalwonder-ui`

Expected: Success

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): wire use_canvas_interaction hook to InteractiveCanvas"
```

---

## Task 9: Add Render Effect

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Add render effect that draws test pattern**

Add after the resize effect, before the view! macro:

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
        let precision = vp.precision_bits();

        // Calculate tick parameters from viewport width
        let tick_params = calculate_tick_params(vp.width.to_f64());

        // Create pixel buffer
        let mut data = vec![0u8; (width * height * 4) as usize];

        for py in 0..height {
            for px in 0..width {
                // Convert pixel to fractal coordinates
                let (fx, fy) = pixel_to_fractal(px as f64, py as f64, &vp, size, precision);

                // Compute color (using f64 for the pattern - ok for visualization)
                let color = test_pattern_color(fx.to_f64(), fy.to_f64(), &tick_params);

                let idx = ((py * width + px) * 4) as usize;
                data[idx] = color[0];
                data[idx + 1] = color[1];
                data[idx + 2] = color[2];
                data[idx + 3] = color[3];
            }
        }

        // Draw to canvas
        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
                .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });
```

**Step 2: Clean up unused imports**

Remove `gradient_color` function and its tests (no longer needed).

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-ui`

Expected: Success

**Step 4: Run full test suite**

Run: `cargo test --workspace -- --nocapture`

Expected: All pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): add viewport-driven test pattern rendering"
```

---

## Task 10: Browser Testing

**Files:** None (manual testing)

**Step 1: Start dev server**

Run: `trunk serve` (if not already running)

**Step 2: Open browser and verify**

Open: `http://localhost:8080`

**Verify:**
- [ ] See checkerboard pattern with origin marker
- [ ] Ruler lines visible at x=0 and y=0
- [ ] Tick marks at regular intervals
- [ ] Drag canvas - preview moves smoothly
- [ ] Release - pattern re-renders at new position
- [ ] Scroll to zoom - ticks rescale
- [ ] Double-click to zoom in
- [ ] Alt+double-click to zoom out
- [ ] UI panel shows viewport info updating

**Step 3: Commit verification note**

```bash
git commit --allow-empty -m "test: verify iteration 4 browser functionality"
```

---

## Task 11: Final Cleanup and Quality Checks

**Files:**
- All modified files

**Step 1: Run formatter**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All pass

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: iteration 4 cleanup and quality checks"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Tick spacing calculation with tests |
| 2 | Additional tick spacing tests |
| 3 | Color constants and distance helper |
| 4 | Checkerboard cell detection |
| 5 | Main test_pattern_color function |
| 6 | Update InteractiveCanvas signature |
| 7 | Update App to use new props |
| 8 | Wire use_canvas_interaction hook |
| 9 | Add viewport-driven render effect |
| 10 | Browser testing |
| 11 | Final cleanup |
