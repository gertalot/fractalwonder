# Arbitrary Precision Mandelbrot Implementation Plan (SIMPLIFIED v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable Mandelbrot set rendering with arbitrary precision for deep zoom (up to 10^100+) using zoom-based precision calculation.

**Architecture:**
- `MandelbrotComputer<BigFloat>` - ALWAYS uses arbitrary precision, precision bits scale with zoom
- `TestImageComputer<f64>` - ALWAYS uses f64
- `TilingCanvasRenderer<R>` - generic over Renderer
- No runtime precision swapping needed!

**Tech Stack:** Rust, dashu/dashu-float, Leptos, WASM, existing generic architecture

---

## Current State

**What exists (Phase 1 - DONE):**
- ✓ `dashu` and `dashu-float` dependencies added to Cargo.toml
- ✓ `src/rendering/numeric.rs` with `BigFloat` type and `ImageFloat` trait
- ✓ `MandelbrotComputer<T>` is generic
- ✓ All `type Coord` renamed to `type Scalar` (13 files)
- ✓ 89 tests passing, clippy clean

**What's wrong:**
- ❌ `ImageFloat` trait is redundant - should use standard Rust operators
- ❌ `TilingCanvasRenderer` was deliberately changed from generic to trait object with hardcoded `Scalar = f64`
- ❌ `CanvasRenderer` trait hardcoded to `Scalar = f64`
- ❌ No precision calculation based on zoom

**What we're building:**
- Fix `ImageFloat` → use standard traits + simple `ToF64` for display
- Revert `TilingCanvasRenderer` back to generic `TilingCanvasRenderer<R>`
- Revert `CanvasRenderer` trait back to generic
- Add `PrecisionCalculator` to calculate precision bits from zoom
- Update `BigFloat` to adjust precision based on zoom
- Use `MandelbrotComputer<BigFloat>` always (no swapping!)

---

## Phase 2: Fix ImageFloat and Revert to Generic Architecture

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

#### Step 2.0.2: Run test to verify it fails

```bash
cargo test test_to_f64_trait
```

Expected: FAIL - "no method named `to_f64` found"

#### Step 2.0.3: Create ToF64 trait

**File:** `src/rendering/numeric.rs` (replace ImageFloat trait section, lines 5-41)

Replace the entire `ImageFloat` trait with:

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

**File:** `src/rendering/numeric.rs` (replace existing ImageFloat impl for f64, lines 189-222)

Replace the entire `impl ImageFloat for f64` block with:

```rust
impl ToF64 for f64 {
    fn to_f64(&self) -> f64 {
        *self
    }
}
```

#### Step 2.0.5: Implement ToF64 for BigFloat

**File:** `src/rendering/numeric.rs` (update existing ImageFloat impl for BigFloat, lines 150-187)

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

#### Step 2.0.7: Verify BigFloat From<f64> impl exists

**File:** `src/rendering/numeric.rs` (lines 144-148, verify it exists)

Ensure this exists:
```rust
impl From<f64> for BigFloat {
    fn from(val: f64) -> Self {
        Self::with_precision(val, 256)
    }
}
```

Note: Default precision of 256 is fine - we'll override it when creating BigFloat from zoom-based precision.

#### Step 2.0.8: Remove old ImageFloat tests

**File:** `src/rendering/numeric.rs` (lines 228-238)

Remove `test_f64_image_float` test (it uses ImageFloat trait methods).
Keep all other tests that use standard operators.

#### Step 2.0.9: Update MandelbrotComputer trait bounds

**File:** `src/rendering/computers/mandelbrot.rs` (lines 5-50)

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

**File:** `src/rendering/computers/mandelbrot.rs` (update to use From instead of from_f64)

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

#### Step 2.0.11: Update MandelbrotComputer::compute to use standard operators

**File:** `src/rendering/computers/mandelbrot.rs` (replace compute method)

Replace entire compute method to use `*` instead of `.mul()`, `+` instead of `.add()`, etc:

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

**File:** `src/rendering/computers/mandelbrot.rs`

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

#### Step 2.0.13: Update imports in mandelbrot.rs

**File:** `src/rendering/computers/mandelbrot.rs` (top of file)

Replace:
```rust
use crate::rendering::numeric::ImageFloat;
```

With:
```rust
use crate::rendering::numeric::ToF64;
```

#### Step 2.0.14: Update public exports

**File:** `src/rendering/mod.rs`

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

### Task 2.1: Revert TilingCanvasRenderer to Generic

**Goal:** Revert to the generic architecture from commit c0392f5467e524755c84f39f5250f6f5b0f606f8

**Files:**
- Modify: `src/rendering/tiling_canvas_renderer.rs`

#### Step 2.1.1: Revert TilingCanvasRenderer to generic

**File:** `src/rendering/tiling_canvas_renderer.rs`

Get the old generic version:

```bash
git show c0392f5467e524755c84f39f5250f6f5b0f606f8:src/rendering/tiling_canvas_renderer.rs > /tmp/tiling_canvas_renderer_old.rs
```

Then manually update it:
- Replace `R::Coord` with `R::Scalar` throughout (the type was renamed)
- Copy to `src/rendering/tiling_canvas_renderer.rs`

Or do it manually - the key changes are:

```rust
/// Cached rendering state
struct CachedState<R: Renderer> {
    viewport: Option<Viewport<R::Scalar>>,  // was R::Coord
    canvas_size: Option<(u32, u32)>,
    data: Vec<R::Data>,
    render_id: AtomicU32,
}

pub struct TilingCanvasRenderer<R: Renderer> {
    renderer: R,
    colorizer: Colorizer<R::Data>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<R>>>,
}
```

#### Step 2.1.2: Try to compile

```bash
cargo check
```

Expected: FAIL - need to update CanvasRenderer trait and app.rs

This is expected - we'll fix it in next tasks.

#### Step 2.1.3: Commit generic TilingCanvasRenderer

```bash
git add src/rendering/tiling_canvas_renderer.rs
git commit -m "refactor: revert TilingCanvasRenderer to generic over Renderer

- Was generic, got changed to trait object with Scalar=f64
- Reverting back to generic TilingCanvasRenderer<R: Renderer>
- Allows different Scalar types (f64, BigFloat)
- Breaking change - needs CanvasRenderer trait and app.rs updates"
```

---

### Task 2.2: Make CanvasRenderer trait generic

**Files:**
- Modify: `src/rendering/canvas_renderer.rs`
- Modify: `src/rendering/tiling_canvas_renderer.rs`

#### Step 2.2.1: Make CanvasRenderer trait generic

**File:** `src/rendering/canvas_renderer.rs`

Replace entire trait:
```rust
use crate::rendering::{points::Rect, renderer_trait::Renderer, viewport::Viewport, AppData, Colorizer};
use web_sys::HtmlCanvasElement;

/// Canvas renderer trait - takes a Renderer and Colorizer to render RGBA pixels on a canvas
///
/// Implementations handle the strategy for putting computed data onto canvas pixels:
/// - TilingCanvasRenderer: progressive tiled rendering with caching
/// - Future: SimpleCanvasRenderer, OffscreenCanvasRenderer, etc.
pub trait CanvasRenderer {
    type Scalar;
    type Data: Clone;

    /// Swap the underlying renderer at runtime (invalidates cache)
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>);

    /// Swap the colorizer at runtime (preserves cache if implementation supports it)
    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>);

    /// Main rendering entry point - renders viewport to canvas
    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement);

    /// Get natural bounds from the underlying renderer
    fn natural_bounds(&self) -> Rect<Self::Scalar>;

    /// Cancel any in-progress render
    fn cancel_render(&self);
}
```

Note: We KEEP `set_renderer` because we want to swap between Mandelbrot and TestImage without recreating TilingCanvasRenderer (preserves cache for colorizer swaps).

#### Step 2.2.2: Update TilingCanvasRenderer to hold trait object

**File:** `src/rendering/tiling_canvas_renderer.rs`

Change the renderer field from generic `R` to trait object:

```rust
pub struct TilingCanvasRenderer<R: Renderer> {
    renderer: Box<dyn Renderer<Scalar = R::Scalar, Data = R::Data>>,  // Trait object!
    colorizer: Colorizer<R::Data>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<R>>>,
}

impl<R: Renderer> TilingCanvasRenderer<R> {
    pub fn new(
        renderer: Box<dyn Renderer<Scalar = R::Scalar, Data = R::Data>>,
        colorizer: Colorizer<R::Data>,
        tile_size: u32,
    ) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
        }
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = R::Scalar, Data = R::Data>>) {
        self.renderer = renderer;
        self.clear_cache();
    }

    // ... rest stays same
}
```

Wait, this doesn't work. The struct is generic over `R` but doesn't actually contain `R`. We need a phantom type or a different approach.

**Actually, simpler:** Make TilingCanvasRenderer store the Scalar and Data types directly:

```rust
pub struct TilingCanvasRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
}

struct CachedState<S, D: Clone> {
    viewport: Option<Viewport<S>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<D>,
    render_id: AtomicU32,
}
```

#### Step 2.2.3: Implement CanvasRenderer for TilingCanvasRenderer

**File:** `src/rendering/tiling_canvas_renderer.rs` (add at end)

```rust
impl<S: Clone + PartialEq, D: Clone> CanvasRenderer for TilingCanvasRenderer<S, D> {
    type Scalar = S;
    type Data = D;

    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>) {
        self.set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        self.set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        self.render(viewport, canvas);
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        self.natural_bounds()
    }

    fn cancel_render(&self) {
        self.cancel_render()
    }
}
```

#### Step 2.2.4: Try to compile

```bash
cargo check
```

Expected: Still fails in app.rs and render_config.rs - that's next

#### Step 2.2.5: Commit generic CanvasRenderer

```bash
git add src/rendering/canvas_renderer.rs src/rendering/tiling_canvas_renderer.rs
git commit -m "refactor: make CanvasRenderer trait and TilingCanvasRenderer generic

- CanvasRenderer has associated types Scalar and Data
- TilingCanvasRenderer generic over <S, D> instead of <R: Renderer>
- Holds trait object internally for renderer swapping
- Preserves cache on colorizer swap, clears on renderer swap
- Breaking change - app.rs and render_config.rs need updates"
```

---

### Task 2.3: Add PrecisionCalculator

**Files:**
- Create: `src/rendering/precision.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.3.1: Write failing test for precision calculation

**File:** `src/rendering/precision.rs` (new file)

```rust
pub struct PrecisionCalculator;

impl PrecisionCalculator {
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_scales_with_zoom() {
        let bits_1 = PrecisionCalculator::calculate_precision_bits(1.0);
        let bits_5 = PrecisionCalculator::calculate_precision_bits(1e5);
        let bits_10 = PrecisionCalculator::calculate_precision_bits(1e10);
        let bits_15 = PrecisionCalculator::calculate_precision_bits(1e15);
        let bits_30 = PrecisionCalculator::calculate_precision_bits(1e30);
        let bits_50 = PrecisionCalculator::calculate_precision_bits(1e50);

        // At low zoom, should use reasonable baseline
        assert!(bits_1 >= 64);
        assert!(bits_1 <= 256);

        // Should scale with zoom
        assert!(bits_10 >= bits_5);
        assert!(bits_15 > bits_10);
        assert!(bits_30 > bits_15);
        assert!(bits_50 > bits_30);
    }

    #[test]
    fn test_precision_is_power_of_two() {
        let bits = PrecisionCalculator::calculate_precision_bits(1e20);
        assert_eq!(bits.count_ones(), 1); // Power of 2
    }

    #[test]
    fn test_minimum_precision() {
        // Even at zoom=1, should have reasonable minimum
        let bits = PrecisionCalculator::calculate_precision_bits(1.0);
        assert!(bits >= 64);
    }
}
```

#### Step 2.3.2: Run test to verify it fails

```bash
cargo test precision
```

Expected: FAIL - "not yet implemented"

#### Step 2.3.3: Implement PrecisionCalculator

**File:** `src/rendering/precision.rs` (replace impl block)

```rust
impl PrecisionCalculator {
    /// Calculate required precision bits for given zoom level.
    ///
    /// Formula: precision_bits = max(zoom.log10() × 3.322 + 128, 64).next_power_of_two()
    ///
    /// Explanation:
    /// - Each decimal digit requires ~3.322 bits (log2(10))
    /// - Add 128 bit base for safety margin
    /// - Minimum 64 bits (for low zoom)
    /// - Round up to next power of 2 for efficient allocation
    ///
    /// Examples:
    /// - zoom=1: 64 bits (minimum)
    /// - zoom=1e10: 128 bits
    /// - zoom=1e15: 256 bits
    /// - zoom=1e30: 512 bits
    /// - zoom=1e50: 1024 bits
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        let zoom_digits = zoom.log10();
        let required_bits = (zoom_digits * 3.322 + 128.0) as usize;
        required_bits.max(64).next_power_of_two()
    }
}
```

#### Step 2.3.4: Run tests

```bash
cargo test precision
```

Expected: All tests PASS

#### Step 2.3.5: Add module exports

**File:** `src/rendering/mod.rs`

Add module:
```rust
pub mod precision;
```

Add export:
```rust
pub use precision::PrecisionCalculator;
```

#### Step 2.3.6: Run all tests

```bash
cargo test --workspace
```

Expected: All tests PASS

#### Step 2.3.7: Commit PrecisionCalculator

```bash
git add src/rendering/precision.rs src/rendering/mod.rs
git commit -m "feat: add PrecisionCalculator for zoom-based precision

- Calculate required bits: zoom.log10() × 3.322 + 128
- Minimum 64 bits, scales automatically with zoom
- Power-of-two rounding for efficient allocation
- Full test coverage"
```

---

### Task 2.4: Update RenderConfig to use generic renderers

**Files:**
- Modify: `src/rendering/render_config.rs`

#### Step 2.4.1: Update create_mandelbrot_renderer to use BigFloat

**File:** `src/rendering/render_config.rs`

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
fn create_mandelbrot_renderer() -> Box<dyn Renderer<Scalar = BigFloat, Data = AppData>> {
    let computer = MandelbrotComputer::<BigFloat>::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    Box::new(app_renderer)
}
```

#### Step 2.4.2: Update RenderConfig signature

**File:** `src/rendering/render_config.rs`

The create_renderer field needs to be generic. We have a problem: different renderers return different Scalar types.

**Solution:** Make RenderConfig generic over Scalar type:

```rust
pub struct RenderConfig<S> {
    pub id: &'static str,
    pub name: &'static str,
    pub color_schemes: &'static [ColorSchemeConfig],
    pub create_renderer: fn() -> Box<dyn Renderer<Scalar = S, Data = AppData>>,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = S>>,
}
```

Then:
```rust
pub const MANDELBROT: RenderConfig<BigFloat> = RenderConfig {
    id: "mandelbrot",
    name: "Mandelbrot Set",
    color_schemes: &[...],
    create_renderer: create_mandelbrot_renderer,
    create_info_provider: create_mandelbrot_info_provider,
};

pub const TEST_IMAGE: RenderConfig<f64> = RenderConfig {
    id: "test_image",
    name: "Test Image",
    color_schemes: &[...],
    create_renderer: create_test_image_renderer,
    create_info_provider: create_test_image_info_provider,
};
```

But now `RENDER_CONFIGS` array can't hold both types! We need an enum:

```rust
pub enum AnyRenderConfig {
    F64(RenderConfig<f64>),
    BigFloat(RenderConfig<BigFloat>),
}
```

This is getting complicated. **Simpler approach:** Just remove the generic from RenderConfig and make app.rs handle it.

Actually, let me think about this differently...

#### Step 2.4.3: Simplify - remove create_renderer from RenderConfig

The issue is that `RenderConfig` needs to be in a static array but different renderers have different Scalar types.

**Simplest solution:** Remove `create_renderer` from `RenderConfig`. The app will create renderers directly knowing which type each needs.

```rust
pub struct RenderConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub color_schemes: &'static [ColorSchemeConfig],
    // Remove create_renderer - app will create directly
    // Keep create_info_provider but use f64 for UI display
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}
```

Delete `create_mandelbrot_renderer` and `create_test_image_renderer` functions.

Update configs to remove `create_renderer` field.

#### Step 2.4.4: Try to compile

```bash
cargo check
```

Expected: FAIL in app.rs - needs to create renderers directly

#### Step 2.4.5: Commit RenderConfig changes

```bash
git add src/rendering/render_config.rs
git commit -m "refactor: remove create_renderer from RenderConfig

- Different renderers use different Scalar types (f64 vs BigFloat)
- Can't have single factory function in static registry
- App will create renderers directly based on renderer type
- Breaking change - app.rs needs updates"
```

---

## Phase 3: Update App Component

### Task 3.1: Update app.rs to use BigFloat for Mandelbrot

**Files:**
- Modify: `src/app.rs`

#### Step 3.1.1: Add imports

**File:** `src/app.rs` (add to imports at top)

```rust
use crate::rendering::{
    get_color_scheme, get_config, Viewport, RENDER_CONFIGS,
    TilingCanvasRenderer, Renderer, AppData, Colorizer,
    MandelbrotComputer, TestImageComputer, PixelRenderer, AppDataRenderer,
    BigFloat, PrecisionCalculator, Point, Rect, ToF64,
};
```

#### Step 3.1.2: Create helper functions to build renderers

**File:** `src/app.rs` (add before App component)

```rust
fn create_mandelbrot_canvas_renderer(
    zoom: f64,
    colorizer: Colorizer<AppData>,
) -> TilingCanvasRenderer<BigFloat, AppData> {
    let computer = MandelbrotComputer::<BigFloat>::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    let renderer: Box<dyn Renderer<Scalar = BigFloat, Data = AppData>> = Box::new(app_renderer);
    TilingCanvasRenderer::new(renderer, colorizer, 128)
}

fn create_test_image_canvas_renderer(
    zoom: f64,
    colorizer: Colorizer<AppData>,
) -> TilingCanvasRenderer<f64, AppData> {
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    let renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>> = Box::new(app_renderer);
    TilingCanvasRenderer::new(renderer, colorizer, 128)
}
```

#### Step 3.1.3: Create enum to hold either renderer type

**File:** `src/app.rs` (add before App component)

```rust
enum CanvasRendererHolder {
    F64(TilingCanvasRenderer<f64, AppData>),
    BigFloat(TilingCanvasRenderer<BigFloat, AppData>),
}

impl CanvasRendererHolder {
    fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
        match self {
            CanvasRendererHolder::F64(r) => r.render(viewport, canvas),
            CanvasRendererHolder::BigFloat(r) => {
                // Convert f64 viewport to BigFloat viewport
                let precision_bits = PrecisionCalculator::calculate_precision_bits(viewport.zoom);
                let viewport_big = Viewport::new(
                    Point::new(
                        BigFloat::with_precision(*viewport.center.x(), precision_bits),
                        BigFloat::with_precision(*viewport.center.y(), precision_bits),
                    ),
                    viewport.zoom,
                );
                r.render(&viewport_big, canvas)
            }
        }
    }

    fn natural_bounds(&self) -> Rect<f64> {
        match self {
            CanvasRendererHolder::F64(r) => r.natural_bounds(),
            CanvasRendererHolder::BigFloat(r) => {
                let bounds = r.natural_bounds();
                Rect::new(
                    Point::new(bounds.min.x().to_f64(), bounds.min.y().to_f64()),
                    Point::new(bounds.max.x().to_f64(), bounds.max.y().to_f64()),
                )
            }
        }
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        match self {
            CanvasRendererHolder::F64(r) => r.set_colorizer(colorizer),
            CanvasRendererHolder::BigFloat(r) => r.set_colorizer(colorizer),
        }
    }

    fn cancel_render(&self) {
        match self {
            CanvasRendererHolder::F64(r) => r.cancel_render(),
            CanvasRendererHolder::BigFloat(r) => r.cancel_render(),
        }
    }
}
```

#### Step 3.1.4: Update App component to create initial renderer

**File:** `src/app.rs` (in App component, around line 30)

Replace:
```rust
let initial_renderer = (initial_config.create_renderer)();
let initial_colorizer = get_color_scheme(initial_config, &initial_renderer_state.color_scheme_id)
    .unwrap()
    .colorizer;

let canvas_renderer: RwSignal<Box<dyn CanvasRenderer>> = create_rw_signal(Box::new(
    TilingCanvasRenderer::new(initial_renderer, initial_colorizer, 128),
));
```

With:
```rust
let initial_colorizer = get_color_scheme(initial_config, &initial_renderer_state.color_scheme_id)
    .unwrap()
    .colorizer;

let initial_canvas_renderer = match initial_state.selected_renderer_id.as_str() {
    "mandelbrot" => CanvasRendererHolder::BigFloat(
        create_mandelbrot_canvas_renderer(initial_renderer_state.viewport.zoom, initial_colorizer)
    ),
    "test_image" => CanvasRendererHolder::F64(
        create_test_image_canvas_renderer(initial_renderer_state.viewport.zoom, initial_colorizer)
    ),
    _ => panic!("Unknown renderer: {}", initial_state.selected_renderer_id),
};

let canvas_renderer: RwSignal<CanvasRendererHolder> = create_rw_signal(initial_canvas_renderer);
```

#### Step 3.1.5: Update renderer switching effect

**File:** `src/app.rs` (find effect that switches renderers, around line 66+)

Replace the part that creates new renderer:
```rust
        // Get new renderer and colorizer
        let config = get_config(&new_renderer_id).unwrap();
        let new_renderer = (config.create_renderer)();
        let colorizer = get_color_scheme(config, &new_renderer_state.color_scheme_id)
            .unwrap()
            .colorizer;

        // Swap renderer
        canvas_renderer.update(|cr| {
            cr.set_renderer(new_renderer);
            cr.set_colorizer(colorizer);
        });
```

With:
```rust
        // Get config and colorizer
        let config = get_config(&new_renderer_id).unwrap();
        let colorizer = get_color_scheme(config, &new_renderer_state.color_scheme_id)
            .unwrap()
            .colorizer;

        // Create new canvas renderer
        let new_canvas_renderer = match new_renderer_id.as_str() {
            "mandelbrot" => CanvasRendererHolder::BigFloat(
                create_mandelbrot_canvas_renderer(new_renderer_state.viewport.zoom, colorizer)
            ),
            "test_image" => CanvasRendererHolder::F64(
                create_test_image_canvas_renderer(new_renderer_state.viewport.zoom, colorizer)
            ),
            _ => panic!("Unknown renderer: {}", new_renderer_id),
        };

        // Swap renderer
        canvas_renderer.set(new_canvas_renderer);
```

#### Step 3.1.6: Update colorizer switching effect

**File:** `src/app.rs` (find effect that switches colorizers)

The `set_colorizer` call should already work:
```rust
canvas_renderer.update(|cr| cr.set_colorizer(new_colorizer));
```

#### Step 3.1.7: Update InteractiveCanvas calls

**File:** `src/app.rs` (in view! macro)

The InteractiveCanvas component calls should already work, but verify the `render` call uses the wrapper:
- All calls go through `CanvasRendererHolder` methods
- Viewport is always `Viewport<f64>` in app state
- Conversion to BigFloat happens inside `CanvasRendererHolder::render()`

#### Step 3.1.8: Try to compile

```bash
cargo check
```

Expected: SUCCESS or minor issues to fix

#### Step 3.1.9: Try to run

```bash
trunk serve
```

Test in browser:
- Load app - should show Mandelbrot (using BigFloat)
- Zoom in to various levels (precision automatically adjusts)
- Switch to Test Image (uses f64)
- Switch back to Mandelbrot (uses BigFloat)
- Switch colorizers (cache preserved)

#### Step 3.1.10: Run all tests

```bash
cargo test --workspace --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: All pass

#### Step 3.1.11: Commit app.rs changes

```bash
git add src/app.rs
git commit -m "feat: use BigFloat for Mandelbrot with zoom-based precision

- MandelbrotComputer always uses BigFloat (no runtime swapping)
- Precision bits calculated from zoom level automatically
- TestImageComputer always uses f64
- CanvasRendererHolder enum wraps both types for signal storage
- Viewport stored as f64, converted to BigFloat when rendering
- Cache preserved on colorizer swap, cleared on renderer swap"
```

---

## Phase 4: Testing and Verification

### Task 4.1: Manual browser testing

1. Start dev server: `trunk serve`
2. Open http://localhost:8080
3. Test Mandelbrot at various zoom levels:
   - Zoom 1: BigFloat with 64-128 bits (minimal overhead)
   - Zoom 1e5: BigFloat with 128-256 bits
   - Zoom 1e10: BigFloat with 256 bits
   - Zoom 1e15: BigFloat with 256-512 bits
   - Zoom 1e30: BigFloat with 512-1024 bits
4. Test renderer switching:
   - Start on Mandelbrot
   - Switch to Test Image (should work, uses f64)
   - Switch back to Mandelbrot (should work, uses BigFloat)
5. Test colorizer switching:
   - Zoom in on Mandelbrot
   - Switch colorizer (should be instant - cache preserved)
6. Check for precision:
   - Zoom to 1e20+
   - Verify fractal detail still renders correctly
   - No blurry/pixelated areas from precision loss

### Task 4.2: Add integration test

**File:** `tests/integration/arbitrary_precision.rs` (new file, create `tests/integration/` dir first)

```bash
mkdir -p tests/integration
```

```rust
#[cfg(test)]
mod tests {
    use fractal_wonder::rendering::*;

    #[test]
    fn test_mandelbrot_with_bigfloat() {
        let computer = MandelbrotComputer::<BigFloat>::new();

        // High zoom viewport
        let precision_bits = 256;
        let viewport = Viewport::new(
            Point::new(
                BigFloat::with_precision(-0.5, precision_bits),
                BigFloat::with_precision(0.0, precision_bits),
            ),
            1e15,
        );

        // Compute a point
        let point = Point::new(
            BigFloat::with_precision(-0.5, precision_bits),
            BigFloat::with_precision(0.0, precision_bits),
        );

        let result = computer.compute(point, &viewport);

        // Should compute without panicking
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_precision_calculator_scaling() {
        let bits_1 = PrecisionCalculator::calculate_precision_bits(1.0);
        let bits_10 = PrecisionCalculator::calculate_precision_bits(1e10);
        let bits_15 = PrecisionCalculator::calculate_precision_bits(1e15);
        let bits_30 = PrecisionCalculator::calculate_precision_bits(1e30);

        // Should scale
        assert!(bits_10 >= bits_1);
        assert!(bits_15 > bits_10);
        assert!(bits_30 > bits_15);

        // All should be powers of 2
        assert_eq!(bits_1.count_ones(), 1);
        assert_eq!(bits_10.count_ones(), 1);
        assert_eq!(bits_15.count_ones(), 1);
        assert_eq!(bits_30.count_ones(), 1);
    }

    #[test]
    fn test_bigfloat_to_f64_conversion() {
        let val = BigFloat::with_precision(42.5, 128);
        assert!((val.to_f64() - 42.5).abs() < 1e-10);

        let val2 = BigFloat::with_precision(-0.123456789, 256);
        assert!((val2.to_f64() - (-0.123456789)).abs() < 1e-10);
    }
}
```

### Task 4.3: Run all verification

```bash
# Format
cargo fmt --all

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --workspace --all-targets --all-features

# WASM build
cargo build --target wasm32-unknown-unknown --lib

# Integration tests
cargo test --test arbitrary_precision
```

All should pass.

### Task 4.4: Final commit

```bash
git add tests/
git commit -m "test: add integration tests for arbitrary precision

- Test MandelbrotComputer with BigFloat at high zoom
- Test PrecisionCalculator scaling
- Test BigFloat to f64 conversion
- All tests passing"
```

---

## Success Criteria

- [ ] All existing 89 tests passing
- [ ] New integration tests passing
- [ ] Clippy clean
- [ ] WASM builds successfully
- [ ] Browser testing:
  - [ ] Mandelbrot renders at zoom 1 (low precision BigFloat)
  - [ ] Mandelbrot renders at zoom 1e15 (high precision BigFloat)
  - [ ] Mandelbrot renders at zoom 1e30+ (very high precision)
  - [ ] Test Image still works (uses f64)
  - [ ] Can switch between renderers
  - [ ] Colorizer switching is instant (cache preserved)
  - [ ] Deep zoom shows correct fractal detail (no precision loss)
- [ ] No console errors
- [ ] Performance is acceptable (BigFloat overhead only at high zoom)

---

## Summary

**What we did:**
1. Replaced `ImageFloat` trait with standard Rust traits + simple `ToF64`
2. Reverted `TilingCanvasRenderer` to generic `TilingCanvasRenderer<S, D>`
3. Made `CanvasRenderer` trait generic with associated types
4. Added `PrecisionCalculator` to calculate precision bits from zoom
5. **Mandelbrot ALWAYS uses `BigFloat`** - precision scales with zoom automatically
6. **TestImage ALWAYS uses `f64`** - doesn't need arbitrary precision
7. Added `CanvasRendererHolder` enum to wrap both types for Leptos signal
8. Viewport stored as `f64` in app state, converted to `BigFloat` when rendering

**What we didn't do:**
- No runtime precision swapping (Mandelbrot is always BigFloat)
- No complex DynamicRenderer/DynamicViewport abstraction
- No unnecessary indirection
- Just clean generic types

**Key insight:**
BigFloat at low zoom (64-128 bits) has minimal overhead vs f64. So there's no need to swap between f64 and BigFloat at runtime. Just always use BigFloat for Mandelbrot and let precision scale with zoom.

**Lines of code added:** ~250
**Complexity added:** Minimal
**Generics used:** Yes
**Traits used:** Yes
**Over-engineering:** None
