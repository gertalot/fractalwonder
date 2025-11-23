# Iteration 3: Config, Precision & Viewport Fitting - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement fractal configuration, precision calculation, and viewport aspect ratio fitting.

**Architecture:** Core layer gets precision calculation and viewport fitting functions. UI layer gets FractalConfig registry and updated panel display. State flows: config → natural viewport → fitted viewport → precision bits.

**Tech Stack:** Rust, Leptos, dashu (BigFloat), fractalwonder-core, fractalwonder-ui

---

## Task 1: Add `abs()` method to BigFloat

**Files:**
- Modify: `fractalwonder-core/src/bigfloat.rs:219` (after PartialOrd impl)
- Test: `fractalwonder-core/src/bigfloat.rs` (inline tests)

**Step 1: Write the failing test**

Add to the test module at the bottom of `bigfloat.rs`:

```rust
#[test]
fn abs_returns_positive_for_negative_value() {
    let neg = BigFloat::with_precision(-5.0, 64);
    let result = neg.abs();
    assert_eq!(result.to_f64(), 5.0);
}

#[test]
fn abs_returns_same_for_positive_value() {
    let pos = BigFloat::with_precision(3.0, 64);
    let result = pos.abs();
    assert_eq!(result.to_f64(), 3.0);
}

#[test]
fn abs_preserves_precision() {
    let neg = BigFloat::with_precision(-5.0, 256);
    let result = neg.abs();
    assert_eq!(result.precision_bits(), 256);
}

#[test]
fn abs_works_with_arbitrary_precision() {
    let neg = BigFloat::from_string("-1e-500", 7000).unwrap();
    let pos = BigFloat::from_string("1e-500", 7000).unwrap();
    assert_eq!(neg.abs(), pos);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p fractalwonder-core abs_ -- --nocapture
```

Expected: FAIL with "no method named `abs`"

**Step 3: Write minimal implementation**

Add to `impl BigFloat` block (around line 200):

```rust
/// Absolute value
pub fn abs(&self) -> Self {
    match &self.value {
        BigFloatValue::F64(v) => BigFloat {
            value: BigFloatValue::F64(v.abs()),
            precision_bits: self.precision_bits,
        },
        BigFloatValue::Arbitrary(v) => BigFloat {
            value: BigFloatValue::Arbitrary(v.clone().abs()),
            precision_bits: self.precision_bits,
        },
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p fractalwonder-core abs_ -- --nocapture
```

Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/bigfloat.rs
git commit -m "feat(core): add abs() method to BigFloat"
```

---

## Task 2: Add `log2_approx()` method to BigFloat

**Files:**
- Modify: `fractalwonder-core/src/bigfloat.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn log2_approx_returns_correct_value_for_powers_of_two() {
    let val = BigFloat::with_precision(8.0, 64); // 2^3
    let log2 = val.log2_approx();
    assert!((log2 - 3.0).abs() < 0.1);
}

#[test]
fn log2_approx_returns_negative_for_small_values() {
    let val = BigFloat::with_precision(0.125, 64); // 2^-3
    let log2 = val.log2_approx();
    assert!((log2 - (-3.0)).abs() < 0.1);
}

#[test]
fn log2_approx_works_with_extreme_values() {
    // 1e-500 ≈ 2^-1661 (since log2(10) ≈ 3.322)
    let val = BigFloat::from_string("1e-500", 7000).unwrap();
    let log2 = val.log2_approx();
    // Expected: -500 * 3.322 ≈ -1661
    assert!(log2 < -1600.0);
    assert!(log2 > -1700.0);
}

#[test]
fn log2_approx_handles_values_near_one() {
    let val = BigFloat::with_precision(1.0, 64);
    let log2 = val.log2_approx();
    assert!(log2.abs() < 0.1);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p fractalwonder-core log2_approx -- --nocapture
```

Expected: FAIL with "no method named `log2_approx`"

**Step 3: Write minimal implementation**

Add to `impl BigFloat`:

```rust
/// Approximate log2 using exponent extraction.
/// Accurate to ~1 bit, sufficient for precision calculation.
/// Returns f64::NEG_INFINITY for zero values.
pub fn log2_approx(&self) -> f64 {
    match &self.value {
        BigFloatValue::F64(v) => {
            if *v == 0.0 {
                f64::NEG_INFINITY
            } else {
                v.abs().log2()
            }
        }
        BigFloatValue::Arbitrary(v) => {
            if v.is_zero() {
                return f64::NEG_INFINITY;
            }
            // FBig uses base-2 representation internally
            // log2(value) ≈ exponent (crude but sufficient for precision calc)
            // For more accuracy, we convert to f64 if possible, else estimate from exponent
            let f64_val = v.to_f64().value();
            if f64_val != 0.0 && f64_val.is_finite() {
                f64_val.abs().log2()
            } else {
                // Value too extreme for f64, estimate from string representation
                // Parse scientific notation to extract exponent
                let s = v.to_string();
                estimate_log2_from_string(&s)
            }
        }
    }
}
```

Add helper function outside impl block:

```rust
/// Estimate log2 from string representation of a number.
/// Handles scientific notation like "1.23e-500".
fn estimate_log2_from_string(s: &str) -> f64 {
    let s_lower = s.to_lowercase();
    if let Some(e_pos) = s_lower.find('e') {
        // Scientific notation: extract base-10 exponent
        if let Ok(exp10) = s_lower[e_pos + 1..].parse::<i64>() {
            // log2(10^n) = n * log2(10) ≈ n * 3.321928
            return exp10 as f64 * 3.321928;
        }
    }
    // Fallback: try to parse as f64
    if let Ok(f) = s.parse::<f64>() {
        if f != 0.0 && f.is_finite() {
            return f.abs().log2();
        }
    }
    // Last resort: return 0
    0.0
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p fractalwonder-core log2_approx -- --nocapture
```

Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/bigfloat.rs
git commit -m "feat(core): add log2_approx() method to BigFloat"
```

---

## Task 3: Create precision.rs module

**Files:**
- Create: `fractalwonder-core/src/precision.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Create `fractalwonder-core/src/precision.rs`:

```rust
//! Precision calculation for fractal rendering.
//!
//! Determines how many mantissa bits are needed to accurately compute
//! fractal values at a given viewport and resolution.

use crate::Viewport;

/// Safety margin for rounding errors in arithmetic operations.
const SAFETY_BITS: u64 = 64;

/// Default maximum iterations for Mandelbrot computation.
const DEFAULT_MAX_ITERATIONS: u64 = 10_000;

/// Calculate required precision bits for fractal computation.
///
/// Determines how many mantissa bits BigFloat values need to:
/// 1. Represent coordinates at the viewport's zoom level
/// 2. Distinguish adjacent pixels in the computation
/// 3. Survive error amplification over many iterations
///
/// # Arguments
/// * `viewport` - The fractal-space region to render
/// * `canvas_size` - The pixel resolution (width, height)
///
/// # Returns
/// Required precision bits, rounded up to a power of 2 for efficiency.
pub fn calculate_precision_bits(viewport: &Viewport, canvas_size: (u32, u32)) -> usize {
    calculate_precision_bits_with_iterations(viewport, canvas_size, DEFAULT_MAX_ITERATIONS)
}

/// Calculate precision bits with custom iteration count.
pub fn calculate_precision_bits_with_iterations(
    viewport: &Viewport,
    canvas_size: (u32, u32),
    max_iterations: u64,
) -> usize {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_at_1x_zoom_is_reasonable() {
        // At 1x zoom with 4K canvas, should need ~128 bits
        let viewport = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let bits = calculate_precision_bits(&viewport, (3840, 2160));
        assert!(bits >= 64);
        assert!(bits <= 256);
    }

    #[test]
    fn precision_increases_with_zoom() {
        let viewport_1x = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let viewport_1000x = Viewport::from_f64(-0.5, 0.0, 0.004, 0.004, 128);

        let bits_1x = calculate_precision_bits(&viewport_1x, (1920, 1080));
        let bits_1000x = calculate_precision_bits(&viewport_1000x, (1920, 1080));

        assert!(bits_1000x > bits_1x);
    }

    #[test]
    fn precision_at_extreme_zoom() {
        // At 10^500 zoom, width is ~10^-500
        let viewport = Viewport::from_strings(
            "-0.5", "0.0",
            "1e-500", "1e-500",
            7000
        ).unwrap();

        let bits = calculate_precision_bits(&viewport, (1920, 1080));

        // Should need ~1700+ bits (500 * 3.322 + safety)
        assert!(bits >= 1024);
        assert!(bits <= 4096);
    }

    #[test]
    fn precision_is_power_of_two() {
        let viewport = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let bits = calculate_precision_bits(&viewport, (1920, 1080));
        assert!(bits.is_power_of_two());
    }

    #[test]
    fn precision_minimum_is_64() {
        let viewport = Viewport::from_f64(0.0, 0.0, 1000.0, 1000.0, 64);
        let bits = calculate_precision_bits(&viewport, (100, 100));
        assert!(bits >= 64);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p fractalwonder-core precision -- --nocapture
```

Expected: FAIL with "not yet implemented"

**Step 3: Write minimal implementation**

Replace the `todo!()` in `calculate_precision_bits_with_iterations`:

```rust
pub fn calculate_precision_bits_with_iterations(
    viewport: &Viewport,
    canvas_size: (u32, u32),
    max_iterations: u64,
) -> usize {
    let (cx, cy) = &viewport.center;
    let width = &viewport.width;
    let height = &viewport.height;

    let px = canvas_size.0 as f64;
    let py = canvas_size.1 as f64;

    // log2(min_delta) where delta = dimension / pixels
    let log2_delta_x = width.log2_approx() - px.log2();
    let log2_delta_y = height.log2_approx() - py.log2();
    let log2_min_delta = log2_delta_x.min(log2_delta_y);

    // M = max(|cx| + width/2, |cy| + height/2)
    // Approximate log2(M) conservatively
    let log2_half_width = width.log2_approx() - 1.0;
    let log2_half_height = height.log2_approx() - 1.0;
    let log2_cx = cx.abs().log2_approx();
    let log2_cy = cy.abs().log2_approx();

    // For sums like |cx| + width/2, use max and add 1 bit for safety
    let log2_mx = log2_cx.max(log2_half_width) + 1.0;
    let log2_my = log2_cy.max(log2_half_height) + 1.0;
    let log2_m = log2_mx.max(log2_my);

    // bits_from_ratio = ceil(log2(M / min_delta))
    let log2_ratio = log2_m - log2_min_delta;
    let bits_from_ratio = log2_ratio.ceil().max(0.0) as u64;

    // Bits for iteration error amplification: log2(iterations)
    let iter_bits = if max_iterations > 1 {
        (max_iterations as f64).log2().ceil() as u64
    } else {
        0
    };

    let total_bits = bits_from_ratio + iter_bits + SAFETY_BITS;

    // Round to power of 2, minimum 64 bits
    (total_bits as usize).next_power_of_two().max(64)
}
```

**Step 4: Update lib.rs to export the module**

In `fractalwonder-core/src/lib.rs`, add:

```rust
pub mod precision;

pub use precision::calculate_precision_bits;
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p fractalwonder-core precision -- --nocapture
```

Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add fractalwonder-core/src/precision.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add precision calculation module"
```

---

## Task 4: Add `fit_viewport_to_canvas()` to transforms.rs

**Files:**
- Modify: `fractalwonder-core/src/transforms.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-core/src/transforms.rs` test module:

```rust
// ============================================================================
// fit_viewport_to_canvas tests
// ============================================================================

#[test]
fn fit_viewport_expands_width_for_landscape_canvas() {
    // Square viewport (4x4) on landscape canvas (1920x1080)
    let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
    let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

    // Width should be expanded: new_width = height * (1920/1080) ≈ 7.11
    assert!(fitted.width.to_f64() > 4.0);
    assert!((fitted.height.to_f64() - 4.0).abs() < 0.001);
}

#[test]
fn fit_viewport_expands_height_for_portrait_canvas() {
    // Square viewport (4x4) on portrait canvas (1080x1920)
    let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
    let fitted = fit_viewport_to_canvas(&natural, (1080, 1920));

    // Height should be expanded: new_height = width / (1080/1920) ≈ 7.11
    assert!((fitted.width.to_f64() - 4.0).abs() < 0.001);
    assert!(fitted.height.to_f64() > 4.0);
}

#[test]
fn fit_viewport_preserves_center() {
    let natural = Viewport::from_f64(-0.5, 0.3, 4.0, 4.0, 128);
    let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

    assert_eq!(fitted.center.0, natural.center.0);
    assert_eq!(fitted.center.1, natural.center.1);
}

#[test]
fn fit_viewport_preserves_precision() {
    let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 512);
    let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

    assert_eq!(fitted.precision_bits(), 512);
}

#[test]
fn fit_viewport_unchanged_for_matching_aspect() {
    // Viewport with same aspect as canvas
    let natural = Viewport::from_f64(0.0, 0.0, 16.0, 9.0, 128);
    let fitted = fit_viewport_to_canvas(&natural, (1920, 1080)); // 16:9

    assert!((fitted.width.to_f64() - 16.0).abs() < 0.001);
    assert!((fitted.height.to_f64() - 9.0).abs() < 0.001);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p fractalwonder-core fit_viewport -- --nocapture
```

Expected: FAIL with "cannot find function `fit_viewport_to_canvas`"

**Step 3: Write minimal implementation**

Add to `fractalwonder-core/src/transforms.rs`:

```rust
/// Expand viewport to match canvas aspect ratio.
///
/// The viewport is expanded (never shrunk) so the natural bounds remain
/// fully visible regardless of canvas shape. Center stays fixed.
///
/// # Arguments
/// * `natural_viewport` - The viewport with natural fractal bounds
/// * `canvas_size` - Canvas dimensions in pixels (width, height)
///
/// # Returns
/// A new viewport with dimensions adjusted to match canvas aspect ratio.
pub fn fit_viewport_to_canvas(
    natural_viewport: &Viewport,
    canvas_size: (u32, u32),
) -> Viewport {
    let canvas_aspect = canvas_size.0 as f64 / canvas_size.1 as f64;

    // Safe at any zoom: ratio of similar-magnitude values gives reasonable f64
    let viewport_aspect = natural_viewport
        .width
        .div(&natural_viewport.height)
        .to_f64();

    let precision = natural_viewport.precision_bits();

    if canvas_aspect > viewport_aspect {
        // Canvas wider than viewport: expand width
        // new_width = height * canvas_aspect
        let aspect = BigFloat::with_precision(canvas_aspect, precision);
        let new_width = natural_viewport.height.mul(&aspect);

        Viewport::with_bigfloat(
            natural_viewport.center.0.clone(),
            natural_viewport.center.1.clone(),
            new_width,
            natural_viewport.height.clone(),
        )
    } else {
        // Canvas taller than viewport: expand height
        // new_height = width / canvas_aspect
        let aspect = BigFloat::with_precision(canvas_aspect, precision);
        let new_height = natural_viewport.width.div(&aspect);

        Viewport::with_bigfloat(
            natural_viewport.center.0.clone(),
            natural_viewport.center.1.clone(),
            natural_viewport.width.clone(),
            new_height,
        )
    }
}
```

**Step 4: Update lib.rs to export**

In `fractalwonder-core/src/lib.rs`, add to the `pub use transforms::` line:

```rust
pub use transforms::{
    apply_pixel_transform_to_viewport, calculate_aspect_ratio, compose_affine_transformations,
    fit_viewport_to_canvas, fractal_to_pixel, pixel_to_fractal, AffinePrimitive, PixelMat3,
    PixelTransform,
};
```

**Step 5: Run test to verify it passes**

```bash
cargo test -p fractalwonder-core fit_viewport -- --nocapture
```

Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add fractalwonder-core/src/transforms.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add fit_viewport_to_canvas function"
```

---

## Task 5: Create FractalConfig in UI layer

**Files:**
- Create: `fractalwonder-ui/src/config.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create config module with tests**

Create `fractalwonder-ui/src/config.rs`:

```rust
//! Fractal configuration registry.
//!
//! Defines available fractal types with their natural bounds and metadata.

use fractalwonder_core::Viewport;

/// Configuration for a fractal type.
#[derive(Clone, Copy, Debug)]
pub struct FractalConfig {
    /// Unique identifier (matches renderer ID in compute layer)
    pub id: &'static str,
    /// Human-readable name for UI display
    pub display_name: &'static str,
    /// Default center coordinates as strings (preserves precision)
    pub default_center: (&'static str, &'static str),
    /// Default width in fractal space as string
    pub default_width: &'static str,
    /// Default height in fractal space as string
    pub default_height: &'static str,
}

impl FractalConfig {
    /// Create the default viewport for this fractal at the given precision.
    pub fn default_viewport(&self, precision_bits: usize) -> Viewport {
        Viewport::from_strings(
            self.default_center.0,
            self.default_center.1,
            self.default_width,
            self.default_height,
            precision_bits,
        )
        .expect("Invalid default viewport coordinates in FractalConfig")
    }
}

/// Registry of available fractal configurations.
pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        default_center: ("0.0", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
    },
];

/// Look up a fractal configuration by ID.
pub fn get_config(id: &str) -> Option<&'static FractalConfig> {
    FRACTAL_CONFIGS.iter().find(|c| c.id == id)
}

/// Get the default fractal configuration.
pub fn default_config() -> &'static FractalConfig {
    get_config("mandelbrot").expect("Default config must exist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_finds_mandelbrot() {
        let config = get_config("mandelbrot");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name, "Mandelbrot Set");
    }

    #[test]
    fn get_config_finds_test_image() {
        let config = get_config("test_image");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name, "Test Pattern");
    }

    #[test]
    fn get_config_returns_none_for_unknown() {
        let config = get_config("unknown_fractal");
        assert!(config.is_none());
    }

    #[test]
    fn default_viewport_creates_valid_viewport() {
        let config = get_config("mandelbrot").unwrap();
        let viewport = config.default_viewport(128);

        assert!((viewport.center.0.to_f64() - (-0.5)).abs() < 0.001);
        assert!((viewport.center.1.to_f64() - 0.0).abs() < 0.001);
        assert!((viewport.width.to_f64() - 4.0).abs() < 0.001);
        assert!((viewport.height.to_f64() - 4.0).abs() < 0.001);
        assert_eq!(viewport.precision_bits(), 128);
    }

    #[test]
    fn default_config_returns_mandelbrot() {
        let config = default_config();
        assert_eq!(config.id, "mandelbrot");
    }
}
```

**Step 2: Update lib.rs**

Check current lib.rs and add config module. First read it:

```bash
# Read fractalwonder-ui/src/lib.rs
```

Then add:

```rust
pub mod config;
pub use config::{default_config, get_config, FractalConfig, FRACTAL_CONFIGS};
```

**Step 3: Run tests**

```bash
cargo test -p fractalwonder-ui config -- --nocapture
```

Expected: All 5 tests PASS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/config.rs fractalwonder-ui/src/lib.rs
git commit -m "feat(ui): add FractalConfig registry"
```

---

## Task 6: Update UIPanel with viewport info

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs`

**Step 1: Update UIPanel component**

Replace `fractalwonder-ui/src/components/ui_panel.rs`:

```rust
// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, InfoButton};
use crate::config::FractalConfig;
use crate::hooks::{use_ui_visibility, UiVisibility};
use fractalwonder_core::Viewport;
use leptos::*;

#[component]
pub fn UIPanel(
    /// Canvas dimensions (width, height)
    canvas_size: Signal<(u32, u32)>,
    /// Current viewport in fractal space
    viewport: Signal<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Calculated precision bits
    precision_bits: Signal<usize>,
) -> impl IntoView {
    let UiVisibility {
        is_visible,
        is_hovering: _,
        set_is_visible: _,
        set_is_hovering,
    } = use_ui_visibility();

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0 pointer-events-none"
        }
    };

    view! {
        <div
            class=move || format!(
                "fixed inset-x-0 bottom-0 z-50 transition-opacity duration-300 {}",
                opacity_class()
            )
            on:mouseenter=move |_| set_is_hovering.set(true)
            on:mouseleave=move |_| set_is_hovering.set(false)
        >
            <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
                // Left section: info button
                <div class="flex items-center space-x-2">
                    <InfoButton />
                </div>

                // Center section: fractal info
                <div class="flex-1 text-center text-white text-sm font-mono">
                    {move || {
                        let cfg = config.get();
                        let vp = viewport.get();
                        let bits = precision_bits.get();
                        let (canvas_w, canvas_h) = canvas_size.get();

                        let cx = format_coordinate(vp.center.0.to_f64());
                        let cy = format_coordinate(vp.center.1.to_f64());
                        let w = format_dimension(vp.width.to_f64());
                        let h = format_dimension(vp.height.to_f64());

                        format!(
                            "Fractal: {} | Viewport: ({}, {}) | {} × {} | Precision: {} bits | Canvas: {} × {}",
                            cfg.display_name, cx, cy, w, h, bits, canvas_w, canvas_h
                        )
                    }}
                </div>

                // Right section: fullscreen
                <div>
                    <FullscreenButton />
                </div>
            </div>
        </div>
    }
}

/// Format a coordinate for display (6 significant figures)
fn format_coordinate(val: f64) -> String {
    if val.abs() < 0.0001 || val.abs() >= 10000.0 {
        format!("{:.4e}", val)
    } else {
        format!("{:.6}", val)
    }
}

/// Format a dimension for display (scientific notation for small values)
fn format_dimension(val: f64) -> String {
    if val < 0.001 {
        format!("{:.2e}", val)
    } else if val < 1.0 {
        format!("{:.4}", val)
    } else {
        format!("{:.2}", val)
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check -p fractalwonder-ui
```

Note: This will fail until we update App to pass the new props. That's Task 7.

**Step 3: Commit partial progress**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git commit -m "feat(ui): update UIPanel with viewport and precision display"
```

---

## Task 7: Update App to wire everything together

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update App component**

Replace `fractalwonder-ui/src/app.rs`:

```rust
// fractalwonder-ui/src/app.rs
use fractalwonder_core::{calculate_precision_bits, fit_viewport_to_canvas, Viewport};
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};
use crate::config::{default_config, FractalConfig};

#[component]
pub fn App() -> impl IntoView {
    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Current fractal configuration
    let (config, _set_config) = create_signal(default_config());

    // Viewport signal - computed from config and canvas size
    let viewport = create_memo(move |_| {
        let cfg = config.get();
        let size = canvas_size.get();

        // Skip if canvas not yet sized
        if size.0 == 0 || size.1 == 0 {
            // Return a default viewport
            return cfg.default_viewport(128);
        }

        // Create natural viewport at initial precision
        let natural = cfg.default_viewport(128);

        // Fit to canvas aspect ratio
        let fitted = fit_viewport_to_canvas(&natural, size);

        // Calculate required precision
        let required_bits = calculate_precision_bits(&fitted, size);

        // If we need more precision, recreate with correct precision
        if required_bits > fitted.precision_bits() {
            let natural_high_prec = cfg.default_viewport(required_bits);
            fit_viewport_to_canvas(&natural_high_prec, size)
        } else {
            fitted
        }
    });

    // Precision bits - derived from viewport and canvas
    let precision_bits = create_memo(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            128 // Default
        } else {
            calculate_precision_bits(&vp, size)
        }
    });

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    view! {
        <InteractiveCanvas on_resize=on_resize />
        <UIPanel
            canvas_size=canvas_size.into()
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
        />
    }
}
```

**Step 2: Verify compilation**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles successfully

**Step 3: Run all tests**

```bash
cargo test --workspace -- --nocapture
```

Expected: All tests pass

**Step 4: Build and test in browser**

```bash
trunk serve
```

Open http://localhost:8080 and verify:
- UI panel shows "Fractal: Mandelbrot Set"
- Shows viewport coordinates and dimensions
- Shows precision bits (should be ~128 at 1x zoom)
- Shows canvas dimensions
- Resize browser window - values update

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(ui): wire App with viewport, config, and precision"
```

---

## Task 8: Run quality checks and final commit

**Files:** All modified files

**Step 1: Format code**

```bash
cargo fmt --all
```

**Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Fix any warnings.

**Step 3: Run all tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: format and lint fixes for Iteration 3"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add `abs()` to BigFloat | bigfloat.rs |
| 2 | Add `log2_approx()` to BigFloat | bigfloat.rs |
| 3 | Create precision.rs module | precision.rs, lib.rs |
| 4 | Add `fit_viewport_to_canvas()` | transforms.rs, lib.rs |
| 5 | Create FractalConfig registry | config.rs, lib.rs |
| 6 | Update UIPanel display | ui_panel.rs |
| 7 | Wire App with new state | app.rs |
| 8 | Quality checks | All |

**Browser verification checklist:**
- [ ] UI panel shows fractal name
- [ ] UI panel shows viewport center coordinates
- [ ] UI panel shows viewport dimensions
- [ ] UI panel shows precision bits
- [ ] UI panel shows canvas dimensions
- [ ] Values update on window resize
