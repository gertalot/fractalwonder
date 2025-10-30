# Arbitrary Precision Mandelbrot Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable Mandelbrot set rendering with arbitrary precision for deep zoom (up to 10^100+) using automatic f64/BigFloat switching based on zoom level.

**Architecture:** Use dashu (pure Rust, WASM-compatible) for arbitrary precision. MandelbrotComputer is generic over Scalar type. DynamicRenderer enum switches between f64 (zoom < 1e10) and BigFloat (zoom >= 1e10) renderers. Precision calculated as `precision_bits = zoom.log10() × 3.322 + 128`.

**Tech Stack:** Rust, dashu/dashu-float, Leptos, WASM, existing Point<T>/Rect<T> architecture

---

## Current State

**What exists (Phase 1 - DONE):**
- ✓ `dashu` and `dashu-float` dependencies added to Cargo.toml
- ✓ `src/rendering/numeric.rs` with `BigFloat` type and `ImageFloat` trait
- ✓ `MandelbrotComputer<T>` is generic (but uses ImageFloat trait - WRONG)
- ✓ All `type Coord` renamed to `type Scalar` (13 files)
- ✓ 89 tests passing, clippy clean

**What's wrong:**
- ❌ `ImageFloat` trait is redundant - `Point<T>` already defines requirements
- ❌ Should use standard Rust operators, not trait methods
- ❌ Pipeline hardcoded to `Scalar = f64`

**What we're building:**
- Fix ImageFloat → use standard traits
- Make pipeline support dynamic f64/BigFloat switching
- Auto-switch based on zoom level

---

## Phase 2: Fix ImageFloat and Pipeline Integration

### Task 2.0: Replace ImageFloat with Standard Traits

**Files:**
- Modify: `src/rendering/numeric.rs`
- Modify: `src/rendering/computers/mandelbrot.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.0.1: Write failing test for ToF64 trait

**File:** `src/rendering/numeric.rs` (add to existing test module at end)

```rust
#[test]
fn test_to_f64_trait() {
    // Test that f64 converts to itself
    let val_f64: f64 = 42.5;
    assert_eq!(val_f64.to_f64(), 42.5);

    // Test that BigFloat converts to f64
    let val_big = BigFloat::with_precision(42.5, 128);
    assert!((val_big.to_f64() - 42.5).abs() < 1e-10);
}
```

**Step 2.0.2: Run test to verify it fails**

```bash
cargo test test_to_f64_trait
```

Expected: FAIL - "no method named `to_f64` found"

#### Step 2.0.3: Create ToF64 trait

**File:** `src/rendering/numeric.rs` (replace ImageFloat trait section, lines 6-41)

Replace this:
```rust
pub trait ImageFloat:
    Clone
    + Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Mul<f64, Output = Self>
    + Div<f64, Output = Self>
{
    fn from_f64(val: f64) -> Self;
    fn from_i32(val: i32) -> Self;
    fn to_f64(&self) -> f64;
    fn mul(&self, other: &Self) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn div(&self, other: &Self) -> Self;
    fn gt(&self, other: &Self) -> bool;
}
```

With this:
```rust
/// Trait for converting numeric types to f64 for display purposes.
/// This is the ONLY non-standard trait we need - everything else uses
/// standard Rust operators (Add, Sub, Mul, Div, From<f64>, PartialOrd).
pub trait ToF64 {
    /// Convert to f64 (may lose precision for arbitrary precision types)
    fn to_f64(&self) -> f64;
}
```

#### Step 2.0.4: Implement ToF64 for f64

**File:** `src/rendering/numeric.rs` (replace existing ImageFloat impl for f64, lines 73-110)

Replace the entire `impl ImageFloat for f64` block with:

```rust
impl ToF64 for f64 {
    fn to_f64(&self) -> f64 {
        *self
    }
}
```

#### Step 2.0.5: Implement ToF64 for BigFloat

**File:** `src/rendering/numeric.rs` (update existing ImageFloat impl for BigFloat, lines 151-188)

Replace the entire `impl ImageFloat for BigFloat` block with:

```rust
impl ToF64 for BigFloat {
    fn to_f64(&self) -> f64 {
        self.value.to_f64().value()
    }
}
```

#### Step 2.0.6: Run test to verify ToF64 works

```bash
cargo test test_to_f64_trait
```

Expected: PASS

#### Step 2.0.7: Update BigFloat to implement From<f64>

**File:** `src/rendering/numeric.rs` (lines 145-150, already exists but verify)

Ensure this exists:
```rust
impl From<f64> for BigFloat {
    fn from(val: f64) -> Self {
        Self::with_precision(val, 256)
    }
}
```

#### Step 2.0.8: Remove old ImageFloat tests

**File:** `src/rendering/numeric.rs` (delete lines 226-244)

Remove `test_image_float_f64` and update remaining test to just test ToF64:

Replace lines 226-244 with:
```rust
    #[test]
    fn test_f64_to_f64() {
        let val: f64 = 42.5;
        assert_eq!(val.to_f64(), 42.5);
    }
```

#### Step 2.0.9: Update MandelbrotComputer trait bounds

**File:** `src/rendering/computers/mandelbrot.rs` (lines 41-46)

Replace:
```rust
impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: ImageFloat + From<f64>,
{
    type Scalar = T;
```

With:
```rust
impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: Clone
        + From<f64>
        + ToF64
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Div<Output = T>
        + PartialOrd,
{
    type Scalar = T;
```

#### Step 2.0.10: Update MandelbrotComputer::natural_bounds

**File:** `src/rendering/computers/mandelbrot.rs` (lines 48-53)

Replace:
```rust
    fn natural_bounds(&self) -> Rect<T> {
        Rect::new(
            Point::new(T::from_f64(-2.5), T::from_f64(-1.25)),
            Point::new(T::from_f64(1.0), T::from_f64(1.25)),
        )
    }
```

With:
```rust
    fn natural_bounds(&self) -> Rect<T> {
        Rect::new(
            Point::new(T::from(-2.5), T::from(-1.25)),
            Point::new(T::from(1.0), T::from(1.25)),
        )
    }
```

#### Step 2.0.11: Update MandelbrotComputer::compute to use operators

**File:** `src/rendering/computers/mandelbrot.rs` (lines 56-92)

Replace entire compute method with:
```rust
    fn compute(&self, point: Point<T>, viewport: &Viewport<T>) -> MandelbrotData {
        let cx = point.x().clone();
        let cy = point.y().clone();

        let max_iterations = calculate_max_iterations(viewport.zoom);

        let mut zx = T::from(0.0);
        let mut zy = T::from(0.0);

        let escape_radius_sq = T::from(4.0);
        let two = T::from(2.0);

        for i in 0..max_iterations {
            let zx_sq = zx.clone() * zx.clone();
            let zy_sq = zy.clone() * zy.clone();

            let magnitude_sq = zx_sq.clone() + zy_sq.clone();
            if magnitude_sq > escape_radius_sq {
                return MandelbrotData {
                    iterations: i,
                    escaped: true,
                };
            }

            let new_zx = zx_sq - zy_sq + cx.clone();
            let new_zy = two.clone() * zx.clone() * zy.clone() + cy.clone();

            zx = new_zx;
            zy = new_zy;
        }

        MandelbrotData {
            iterations: max_iterations,
            escaped: false,
        }
    }
```

#### Step 2.0.12: Update MandelbrotComputer RendererInfo impl

**File:** `src/rendering/computers/mandelbrot.rs` (lines 95-99)

Replace:
```rust
impl<T> RendererInfo for MandelbrotComputer<T>
where
    T: ImageFloat + From<f64>,
{
    type Scalar = T;
```

With:
```rust
impl<T> RendererInfo for MandelbrotComputer<T>
where
    T: Clone + From<f64> + ToF64,
{
    type Scalar = T;
```

#### Step 2.0.13: Add ToF64 import to mandelbrot.rs

**File:** `src/rendering/computers/mandelbrot.rs` (line 1)

Replace:
```rust
use crate::rendering::numeric::ImageFloat;
```

With:
```rust
use crate::rendering::numeric::ToF64;
```

#### Step 2.0.14: Update public exports

**File:** `src/rendering/mod.rs` (line 26)

Replace:
```rust
pub use numeric::{BigFloat, ImageFloat};
```

With:
```rust
pub use numeric::{BigFloat, ToF64};
```

#### Step 2.0.15: Run all tests

```bash
cargo test --workspace --all-targets --all-features
```

Expected: All 89 tests PASS

#### Step 2.0.16: Run clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: No warnings

#### Step 2.0.17: Commit ImageFloat removal

```bash
git add -A
git commit -m "refactor: replace ImageFloat with standard traits + ToF64

- Remove redundant ImageFloat trait
- Use standard Rust operators (Add, Sub, Mul, Div, From<f64>, PartialOrd)
- Add simple ToF64 trait for display conversion only
- Update MandelbrotComputer to use operators instead of trait methods
- All 89 tests passing"
```

---

### Task 2.1: Implement PrecisionCalculator

**Files:**
- Create: `src/rendering/precision.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.1.1: Write failing test for precision calculation

**File:** `src/rendering/precision.rs` (new file)

```rust
pub struct PrecisionCalculator;

impl PrecisionCalculator {
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        todo!()
    }

    pub fn needs_arbitrary_precision(zoom: f64) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_sufficient_for_low_zoom() {
        assert_eq!(PrecisionCalculator::calculate_precision_bits(1.0), 64);
        assert_eq!(PrecisionCalculator::calculate_precision_bits(1e5), 64);
        assert_eq!(PrecisionCalculator::calculate_precision_bits(1e10), 64);
    }

    #[test]
    fn test_needs_arbitrary_precision() {
        assert!(!PrecisionCalculator::needs_arbitrary_precision(1.0));
        assert!(!PrecisionCalculator::needs_arbitrary_precision(1e10));
        assert!(PrecisionCalculator::needs_arbitrary_precision(1e11));
        assert!(PrecisionCalculator::needs_arbitrary_precision(1e50));
    }

    #[test]
    fn test_precision_scales_with_zoom() {
        let bits_15 = PrecisionCalculator::calculate_precision_bits(1e15);
        let bits_30 = PrecisionCalculator::calculate_precision_bits(1e30);
        let bits_50 = PrecisionCalculator::calculate_precision_bits(1e50);

        assert!(bits_15 >= 128);
        assert!(bits_30 > bits_15);
        assert!(bits_50 > bits_30);
    }

    #[test]
    fn test_precision_is_power_of_two() {
        let bits = PrecisionCalculator::calculate_precision_bits(1e20);
        assert_eq!(bits.count_ones(), 1); // Power of 2
    }
}
```

#### Step 2.1.2: Run test to verify it fails

```bash
cargo test --test precision
```

Expected: FAIL - "not yet implemented: todo!()"

#### Step 2.1.3: Implement PrecisionCalculator

**File:** `src/rendering/precision.rs` (replace impl block)

```rust
impl PrecisionCalculator {
    /// Calculate required precision bits for given zoom level.
    ///
    /// For zoom <= 1e10: use f64 (64 bits sufficient)
    /// For zoom > 1e10: use formula precision_bits = zoom.log10() × 3.322 + 128
    ///
    /// Formula explanation:
    /// - Each decimal digit requires ~3.322 bits (log2(10))
    /// - Add 128 bit base for safety margin
    /// - Round up to next power of 2 for efficient allocation
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        if zoom <= 1e10 {
            64 // f64 is sufficient
        } else {
            let zoom_digits = zoom.log10();
            let required_bits = (zoom_digits * 3.322 + 128.0) as usize;
            required_bits.max(128).next_power_of_two()
        }
    }

    /// Determine if arbitrary precision is needed for given zoom level.
    ///
    /// Threshold: 1e10
    /// - Below: f64 precision is sufficient
    /// - Above: need arbitrary precision (BigFloat)
    pub fn needs_arbitrary_precision(zoom: f64) -> bool {
        zoom > 1e10
    }
}
```

#### Step 2.1.4: Run tests to verify implementation

```bash
cargo test precision
```

Expected: All tests PASS

#### Step 2.1.5: Add module to rendering

**File:** `src/rendering/mod.rs` (add after line 7)

```rust
pub mod precision;
```

#### Step 2.1.6: Export PrecisionCalculator

**File:** `src/rendering/mod.rs` (add to exports around line 26)

```rust
pub use precision::PrecisionCalculator;
```

#### Step 2.1.7: Run all tests

```bash
cargo test --workspace
```

Expected: All tests PASS

#### Step 2.1.8: Commit PrecisionCalculator

```bash
git add src/rendering/precision.rs src/rendering/mod.rs
git commit -m "feat: add PrecisionCalculator for zoom-based precision selection

- Calculate required bits: zoom.log10() × 3.322 + 128
- Threshold: 1e10 for f64/BigFloat switch
- Power-of-two rounding for efficient allocation
- Full test coverage"
```

---

### Task 2.2: Create DynamicRenderer and DynamicViewport

**Files:**
- Create: `src/rendering/renderer_factory.rs`
- Modify: `src/rendering/viewport.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.2.1: Add viewport conversion test

**File:** `src/rendering/viewport.rs` (add to test module at end)

```rust
#[test]
fn test_viewport_f64_to_bigfloat_conversion() {
    use crate::rendering::BigFloat;

    let vp_f64 = Viewport::new(Point::new(-0.5, 0.25), 1e15);

    // Should be able to create BigFloat viewport from f64 viewport
    let precision_bits = 256;
    let vp_big = Viewport::new(
        Point::new(
            BigFloat::with_precision(*vp_f64.center.x(), precision_bits),
            BigFloat::with_precision(*vp_f64.center.y(), precision_bits),
        ),
        vp_f64.zoom,
    );

    // Conversion should preserve values (within f64 precision)
    assert!((vp_big.center.x().to_f64() - vp_f64.center.x()).abs() < 1e-10);
    assert!((vp_big.center.y().to_f64() - vp_f64.center.y()).abs() < 1e-10);
    assert_eq!(vp_big.zoom, vp_f64.zoom);
}
```

#### Step 2.2.2: Run test

```bash
cargo test test_viewport_f64_to_bigfloat_conversion
```

Expected: PASS (no code changes needed - just verifying it works)

#### Step 2.2.3: Write DynamicRenderer skeleton with test

**File:** `src/rendering/renderer_factory.rs` (new file)

```rust
use crate::rendering::{
    points::{Point, Rect},
    viewport::Viewport,
    AppData, BigFloat, Colorizer, PrecisionCalculator, ToF64,
};

/// Dynamic renderer that holds either f64 or BigFloat renderer.
/// Automatically chosen based on zoom level via PrecisionCalculator.
pub enum DynamicRenderer {
    F64(Box<dyn crate::rendering::CanvasRenderer>),
    BigFloat(Box<dyn crate::rendering::CanvasRenderer>),
}

/// Dynamic viewport that matches DynamicRenderer precision.
pub enum DynamicViewport {
    F64(Viewport<f64>),
    BigFloat(Viewport<BigFloat>),
}

impl DynamicRenderer {
    /// Create Mandelbrot renderer with precision based on zoom level.
    pub fn create_mandelbrot(zoom: f64, colorizer: Colorizer<AppData>) -> Self {
        todo!("Implement in next step")
    }

    /// Check if currently using arbitrary precision.
    pub fn is_arbitrary_precision(&self) -> bool {
        matches!(self, DynamicRenderer::BigFloat(_))
    }
}

impl DynamicViewport {
    /// Create viewport with appropriate precision for zoom level.
    pub fn from_f64_viewport(vp: &Viewport<f64>) -> Self {
        if PrecisionCalculator::needs_arbitrary_precision(vp.zoom) {
            let precision_bits = PrecisionCalculator::calculate_precision_bits(vp.zoom);
            let vp_big = Viewport::new(
                Point::new(
                    BigFloat::with_precision(*vp.center.x(), precision_bits),
                    BigFloat::with_precision(*vp.center.y(), precision_bits),
                ),
                vp.zoom,
            );
            DynamicViewport::BigFloat(vp_big)
        } else {
            DynamicViewport::F64(vp.clone())
        }
    }

    /// Convert back to f64 viewport (for serialization).
    pub fn to_f64_viewport(&self) -> Viewport<f64> {
        match self {
            DynamicViewport::F64(vp) => vp.clone(),
            DynamicViewport::BigFloat(vp) => Viewport::new(
                Point::new(vp.center.x().to_f64(), vp.center.y().to_f64()),
                vp.zoom,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_viewport_f64_for_low_zoom() {
        let vp = Viewport::new(Point::new(0.0, 0.0), 1e5);
        let dynamic = DynamicViewport::from_f64_viewport(&vp);
        assert!(matches!(dynamic, DynamicViewport::F64(_)));
    }

    #[test]
    fn test_dynamic_viewport_bigfloat_for_high_zoom() {
        let vp = Viewport::new(Point::new(0.0, 0.0), 1e20);
        let dynamic = DynamicViewport::from_f64_viewport(&vp);
        assert!(matches!(dynamic, DynamicViewport::BigFloat(_)));
    }

    #[test]
    fn test_dynamic_viewport_roundtrip() {
        let vp_original = Viewport::new(Point::new(-0.5, 0.25), 1e15);
        let dynamic = DynamicViewport::from_f64_viewport(&vp_original);
        let vp_back = dynamic.to_f64_viewport();

        assert_eq!(vp_back.zoom, vp_original.zoom);
        assert!((vp_back.center.x() - vp_original.center.x()).abs() < 1e-10);
        assert!((vp_back.center.y() - vp_original.center.y()).abs() < 1e-10);
    }
}
```

#### Step 2.2.4: Run tests

```bash
cargo test renderer_factory
```

Expected: Tests PASS (todo!() not called yet)

#### Step 2.2.5: Add module exports

**File:** `src/rendering/mod.rs`

Add after other module declarations:
```rust
pub mod renderer_factory;
```

Add to exports:
```rust
pub use renderer_factory::{DynamicRenderer, DynamicViewport};
```

#### Step 2.2.6: Commit DynamicRenderer skeleton

```bash
git add src/rendering/renderer_factory.rs src/rendering/viewport.rs src/rendering/mod.rs
git commit -m "feat: add DynamicRenderer and DynamicViewport scaffolding

- Enum for f64/BigFloat renderer variants
- Auto-conversion based on PrecisionCalculator
- Viewport roundtrip conversions
- Tests for viewport precision switching"
```

---

## Phase 3: Complete DynamicRenderer Implementation

### Task 3.1: Implement DynamicRenderer::create_mandelbrot

**Files:**
- Modify: `src/rendering/renderer_factory.rs`

#### Step 3.1.1: Write failing test

**File:** `src/rendering/renderer_factory.rs` (add to test module)

```rust
#[test]
fn test_create_mandelbrot_uses_f64_for_low_zoom() {
    use crate::rendering::colorizers::mandelbrot_default_colorizer;

    let renderer = DynamicRenderer::create_mandelbrot(1e5, mandelbrot_default_colorizer);
    assert!(!renderer.is_arbitrary_precision());
}

#[test]
fn test_create_mandelbrot_uses_bigfloat_for_high_zoom() {
    use crate::rendering::colorizers::mandelbrot_default_colorizer;

    let renderer = DynamicRenderer::create_mandelbrot(1e20, mandelbrot_default_colorizer);
    assert!(renderer.is_arbitrary_precision());
}
```

#### Step 3.1.2: Run test to verify it fails

```bash
cargo test test_create_mandelbrot
```

Expected: FAIL - "not yet implemented: todo!()"

#### Step 3.1.3: Implement create_mandelbrot

**File:** `src/rendering/renderer_factory.rs`

Add imports at top:
```rust
use crate::rendering::{
    AppDataRenderer, MandelbrotComputer, PixelRenderer, TilingCanvasRenderer,
    points::{Point, Rect},
    viewport::Viewport,
    AppData, BigFloat, Colorizer, PrecisionCalculator, ToF64,
};
```

Replace `create_mandelbrot` todo!() with:
```rust
    /// Create Mandelbrot renderer with precision based on zoom level.
    pub fn create_mandelbrot(zoom: f64, colorizer: Colorizer<AppData>) -> Self {
        if PrecisionCalculator::needs_arbitrary_precision(zoom) {
            // High zoom: use BigFloat
            let computer = MandelbrotComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| {
                AppData::MandelbrotData(*d)
            });
            let canvas_renderer = TilingCanvasRenderer::new(app_renderer, colorizer, 128);
            DynamicRenderer::BigFloat(Box::new(canvas_renderer))
        } else {
            // Low zoom: use f64
            let computer = MandelbrotComputer::<f64>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| {
                AppData::MandelbrotData(*d)
            });
            let canvas_renderer = TilingCanvasRenderer::new(app_renderer, colorizer, 128);
            DynamicRenderer::F64(Box::new(canvas_renderer))
        }
    }
```

#### Step 3.1.4: Run tests

```bash
cargo test renderer_factory
```

Expected: All tests PASS

#### Step 3.1.5: Commit DynamicRenderer implementation

```bash
git add src/rendering/renderer_factory.rs
git commit -m "feat: implement DynamicRenderer::create_mandelbrot

- Auto-select f64 vs BigFloat based on zoom
- Use PrecisionCalculator for threshold
- Create full renderer pipeline for each variant"
```

---

## Phase 4: Update RenderConfig for Dynamic Precision

### Task 4.1: Update RenderConfig to use DynamicRenderer

**NOTE:** This task has a challenge - RenderConfig needs colorizer but it's selected separately. We'll need to refactor the API.

**Files:**
- Modify: `src/rendering/render_config.rs`
- Modify: `src/app.rs` (will need adjustments)

#### Step 4.1.1: Document current architecture issue

The current `RenderConfig` has:
```rust
pub create_renderer: fn() -> Box<dyn Renderer<Scalar = f64, Data = AppData>>,
```

But `DynamicRenderer::create_mandelbrot` needs:
- `zoom: f64` (for precision selection)
- `colorizer: Colorizer<AppData>` (for rendering)

**Solution:** Change signature to accept these parameters.

#### Step 4.1.2: Update RenderConfig signature

**File:** `src/rendering/render_config.rs` (line 22)

Replace:
```rust
pub create_renderer: fn() -> Box<dyn Renderer<Scalar = f64, Data = AppData>>,
```

With:
```rust
pub create_renderer: fn(zoom: f64, colorizer: Colorizer<AppData>) -> DynamicRenderer,
```

Also update line 23:
```rust
pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
```

(Leave this as-is for now - info provider doesn't need zoom)

#### Step 4.1.3: Update create_mandelbrot_renderer function

**File:** `src/rendering/render_config.rs` (lines 33-38)

Replace:
```rust
fn create_mandelbrot_renderer() -> Box<dyn Renderer<Scalar = f64, Data = AppData>> {
    let computer = MandelbrotComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    Box::new(app_renderer)
}
```

With:
```rust
fn create_mandelbrot_renderer(zoom: f64, colorizer: Colorizer<AppData>) -> DynamicRenderer {
    DynamicRenderer::create_mandelbrot(zoom, colorizer)
}
```

Add import at top:
```rust
use crate::rendering::DynamicRenderer;
```

#### Step 4.1.4: Update create_test_image_renderer function

**File:** `src/rendering/render_config.rs` (lines 26-31)

Replace:
```rust
fn create_test_image_renderer() -> Box<dyn Renderer<Scalar = f64, Data = AppData>> {
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    Box::new(app_renderer)
}
```

With:
```rust
fn create_test_image_renderer(zoom: f64, colorizer: Colorizer<AppData>) -> DynamicRenderer {
    // TestImage always uses f64 (doesn't need arbitrary precision)
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    let tiling_renderer = TilingCanvasRenderer::new(app_renderer, colorizer, 128);
    DynamicRenderer::F64(Box::new(tiling_renderer))
}
```

Add imports at top:
```rust
use crate::rendering::{DynamicRenderer, TilingCanvasRenderer};
```

#### Step 4.1.5: Try to compile

```bash
cargo check
```

Expected: FAIL - app.rs needs updates

This is expected. We've changed the API, now we need to update the call sites in app.rs.

#### Step 4.1.6: Commit RenderConfig changes

```bash
git add src/rendering/render_config.rs
git commit -m "refactor: update RenderConfig to use DynamicRenderer

- Change create_renderer signature to accept (zoom, colorizer)
- Use DynamicRenderer::create_mandelbrot for Mandelbrot
- TestImage always uses f64 variant
- Breaking change - app.rs needs updates (next task)"
```

---

## Phase 5: Update App Component

### Task 5.1: Update App to use DynamicRenderer

**WARNING:** This task will be complex. The app.rs file manages renderer state and needs careful updates.

**Files:**
- Modify: `src/app.rs`

#### Step 5.1.1: Update canvas_renderer type

**File:** `src/app.rs` (around line 40)

Find:
```rust
let canvas_renderer: RwSignal<Box<dyn CanvasRenderer>> = create_rw_signal(Box::new(
    TilingCanvasRenderer::new(initial_renderer, initial_colorizer, 128),
));
```

This needs to change, but we need to think about the architecture...

**STOP:** This task requires more architectural decisions. The current CanvasRenderer trait expects `Box<dyn Renderer<Scalar = f64>>` but we now have `DynamicRenderer` enum.

**Decision needed:**
1. Make CanvasRenderer generic over Scalar?
2. Make DynamicRenderer implement CanvasRenderer?
3. Refactor to use DynamicRenderer directly without CanvasRenderer trait?

**Recommendation:** Option 3 - use DynamicRenderer directly, it already has the methods we need.

But this is a significant refactor that needs careful planning.

#### Step 5.1.2: Create architectural decision note

**File:** `docs/plans/2025-01-30-app-integration-architecture.md` (new file)

```markdown
# App Integration Architecture Decision

## Problem

Current: App uses `CanvasRenderer` trait with `Box<dyn Renderer<Scalar = f64>>`
Target: Need to use `DynamicRenderer` enum which switches between f64/BigFloat

## Options

### Option 1: Make CanvasRenderer generic over Scalar
- Pro: Preserves trait abstraction
- Con: Trait objects can't be generic over associated types
- Con: Would need `Box<dyn CanvasRenderer<Scalar = f64>>` and `Box<dyn CanvasRenderer<Scalar = BigFloat>>` separately

### Option 2: Make DynamicRenderer implement CanvasRenderer
- Pro: Could fit into existing code structure
- Con: CanvasRenderer expects single Scalar type, DynamicRenderer has two variants
- Con: Methods would need to match on self

### Option 3: Refactor App to use DynamicRenderer directly
- Pro: Simpler, no trait object complications
- Pro: Clearer what's happening
- Con: Removes abstraction layer
- **Recommendation: Use this**

## Implementation Plan

Replace `RwSignal<Box<dyn CanvasRenderer>>` with `RwSignal<DynamicRenderer>` in app.rs

Methods needed on DynamicRenderer:
- `render(&self, viewport: DynamicViewport, canvas: &HtmlCanvasElement)`
- `natural_bounds(&self) -> DynamicBounds` (enum for Rect<f64>/Rect<BigFloat>)
- `set_colorizer(&mut self, colorizer: Colorizer<AppData>)`
- `cancel_render(&self)`

This matches current CanvasRenderer interface but uses Dynamic types.
```

#### Step 5.1.3: Pause for user decision

At this point, we need user confirmation on the architecture approach before proceeding with app.rs changes.

The plan needs to split into subtasks for implementing the chosen architecture.

---

## Execution Decision Point

We've completed:
- ✅ Task 2.0: Remove ImageFloat (use standard traits)
- ✅ Task 2.1: PrecisionCalculator
- ✅ Task 2.2: DynamicRenderer/DynamicViewport scaffolding
- ✅ Task 3.1: DynamicRenderer::create_mandelbrot
- ✅ Task 4.1: Update RenderConfig

**Next requires architectural decision:**

The integration with app.rs requires choosing how to handle DynamicRenderer in the UI layer. See `docs/plans/2025-01-30-app-integration-architecture.md` for options.

**Recommendation:** Implement Option 3 (use DynamicRenderer directly), but this needs:
1. Add render/natural_bounds/set_colorizer methods to DynamicRenderer
2. Update app.rs to work with DynamicRenderer instead of trait object
3. Handle viewport conversion to DynamicViewport

This is a medium-sized refactor that should be reviewed before proceeding.

---

## Testing Strategy

After implementation complete:

### Unit Tests
- All existing 89 tests must pass
- New tests for PrecisionCalculator
- New tests for DynamicRenderer
- New tests for DynamicViewport

### Integration Tests
Create `tests/integration/arbitrary_precision.rs`:
- Render same scene with f64 and BigFloat, compare outputs
- Test precision transition (zoom from 1e5 → 1e10 → 1e15)
- Verify BigFloat rendering correctness

### WASM Tests
```bash
wasm-pack test --headless --chrome
```

### Manual Browser Testing
1. `trunk serve`
2. Zoom from 1 to 1e10 (should stay f64)
3. Continue to 1e15 (should switch to BigFloat)
4. Verify UI shows precision mode
5. Check DevTools for memory stability

---

## Success Criteria

- [ ] All 89+ tests passing
- [ ] Clippy clean
- [ ] WASM builds successfully
- [ ] Can zoom to 1e50+ without precision loss
- [ ] Automatic f64/BigFloat switching works
- [ ] UI displays current precision mode
- [ ] No memory leaks in browser
- [ ] Performance acceptable (BigFloat within 10x of f64)
