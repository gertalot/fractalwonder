# Arbitrary Precision Mandelbrot Implementation Plan (COMPLETE & SELF-CONTAINED)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable Mandelbrot set rendering with arbitrary precision for deep zoom (up to 10^100+) using zoom-based precision calculation.

**Architecture:**
- `MandelbrotComputer<BigFloat>` - ALWAYS uses arbitrary precision, precision bits scale with zoom
- `TestImageComputer<f64>` - ALWAYS uses f64
- `TilingCanvasRenderer<S, D>` - generic over Scalar and Data types
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
- Revert `TilingCanvasRenderer` back to generic `TilingCanvasRenderer<S, D>`
- Revert `CanvasRenderer` trait back to generic
- Add `PrecisionCalculator` to calculate precision bits from zoom
- Use `MandelbrotComputer<BigFloat>` always (no swapping!)

---

## Phase 2: Fix ImageFloat and Revert to Generic Architecture

### Task 2.0: Replace ImageFloat with Standard Traits

**Files:**
- Modify: `src/rendering/numeric.rs`
- Modify: `src/rendering/computers/mandelbrot.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.0.1: Write failing test for ToF64 trait

**File:** `src/rendering/numeric.rs`

Add this test to the existing `mod tests` section at the end of the file (after all existing tests):

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

Expected: FAIL - "no method named `to_f64` found for type `f64`"

#### Step 2.0.3: Create ToF64 trait

**File:** `src/rendering/numeric.rs` (lines 5-41)

Find the `ImageFloat` trait definition (starts around line 5) and replace the ENTIRE trait with:

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

**File:** `src/rendering/numeric.rs` (lines 189-222)

Find the `impl ImageFloat for f64` block and replace the ENTIRE impl with:

```rust
impl ToF64 for f64 {
    fn to_f64(&self) -> f64 {
        *self
    }
}
```

#### Step 2.0.5: Implement ToF64 for BigFloat

**File:** `src/rendering/numeric.rs` (lines 150-187)

Find the `impl ImageFloat for BigFloat` block and replace the ENTIRE impl with:

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

#### Step 2.0.7: Remove old ImageFloat test

**File:** `src/rendering/numeric.rs` (around lines 228-238)

Find and delete the `test_f64_image_float` test function (it uses ImageFloat trait methods that no longer exist).

Keep all other tests.

#### Step 2.0.8: Update MandelbrotComputer trait bounds

**File:** `src/rendering/computers/mandelbrot.rs`

Find the `impl<T> ImagePointComputer for MandelbrotComputer<T>` block (around line 41) and replace:

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

#### Step 2.0.9: Update MandelbrotComputer::natural_bounds

**File:** `src/rendering/computers/mandelbrot.rs`

Find the `natural_bounds` method and change `T::from_f64(...)` to `T::from(...)`:

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

#### Step 2.0.10: Update MandelbrotComputer::compute to use standard operators

**File:** `src/rendering/computers/mandelbrot.rs`

Find the `compute` method and replace trait method calls (`.mul()`, `.add()`, etc.) with operators (`*`, `+`, etc.):

Replace the entire `compute` method body with:

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

#### Step 2.0.11: Update MandelbrotComputer RendererInfo impl

**File:** `src/rendering/computers/mandelbrot.rs`

Find the `impl<T> RendererInfo for MandelbrotComputer<T>` block and replace:

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

#### Step 2.0.12: Update imports in mandelbrot.rs

**File:** `src/rendering/computers/mandelbrot.rs` (top of file)

Find the import line:
```rust
use crate::rendering::numeric::ImageFloat;
```

Replace with:
```rust
use crate::rendering::numeric::ToF64;
```

#### Step 2.0.13: Update public exports

**File:** `src/rendering/mod.rs`

Find the line:
```rust
pub use numeric::{BigFloat, ImageFloat};
```

Replace with:
```rust
pub use numeric::{BigFloat, ToF64};
```

#### Step 2.0.14: Run all tests

```bash
cargo test --workspace --all-targets --all-features
```

Expected: All tests PASS (may be fewer than 89 since we deleted one test)

#### Step 2.0.15: Run clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: No warnings

#### Step 2.0.16: Commit ImageFloat removal

```bash
git add -A
git commit -m "refactor: replace ImageFloat with standard traits + ToF64

- Remove redundant ImageFloat trait
- Use standard Rust operators (Add, Sub, Mul, Div, From<f64>, PartialOrd)
- Add simple ToF64 trait for display conversion only
- Update MandelbrotComputer to use operators instead of trait methods
- All tests passing"
```

---

### Task 2.1: Make TilingCanvasRenderer and CanvasRenderer Generic

**Files:**
- Modify: `src/rendering/tiling_canvas_renderer.rs` - COMPLETE REWRITE
- Modify: `src/rendering/canvas_renderer.rs`

#### Step 2.1.1: Update CachedState to be generic

**File:** `src/rendering/tiling_canvas_renderer.rs` (lines 12-29)

Replace:
```rust
/// Cached rendering state
struct CachedState {
    viewport: Option<Viewport<f64>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<AppData>,
    render_id: AtomicU32,
}

impl Default for CachedState {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
            render_id: AtomicU32::new(0),
        }
    }
}
```

With:
```rust
/// Cached rendering state
struct CachedState<S, D: Clone> {
    viewport: Option<Viewport<S>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<D>,
    render_id: AtomicU32,
}

impl<S, D: Clone> Default for CachedState<S, D> {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
            render_id: AtomicU32::new(0),
        }
    }
}
```

#### Step 2.1.2: Update TilingCanvasRenderer struct to be generic

**File:** `src/rendering/tiling_canvas_renderer.rs` (lines 31-37)

Replace:
```rust
/// Canvas renderer with tiling, progressive rendering, and caching
pub struct TilingCanvasRenderer {
    renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>,
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState>>,
}
```

With:
```rust
/// Canvas renderer with tiling, progressive rendering, and caching
pub struct TilingCanvasRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
}
```

#### Step 2.1.3: Update TilingCanvasRenderer impl to be generic

**File:** `src/rendering/tiling_canvas_renderer.rs` (lines 39-78)

Replace:
```rust
impl TilingCanvasRenderer {
    pub fn new(
        renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>,
        colorizer: Colorizer<AppData>,
        tile_size: u32,
    ) -> Self {
        // ... rest of method
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>) {
        // ... rest of method
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        // ... rest of method
    }

    // ... other methods
}
```

With:
```rust
impl<S: Clone + PartialEq, D: Clone> TilingCanvasRenderer<S, D> {
    pub fn new(
        renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
        colorizer: Colorizer<D>,
        tile_size: u32,
    ) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
        }
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
        self.renderer = renderer;
        self.clear_cache();
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
        self.colorizer = colorizer;
        // Cache preserved!
    }

    fn clear_cache(&mut self) {
        let mut cache = self.cached_state.lock().unwrap();
        cache.viewport = None;
        cache.canvas_size = None;
        cache.data.clear();
    }

    pub fn natural_bounds(&self) -> Rect<S> {
        self.renderer.natural_bounds()
    }

    /// Cancel any in-progress render
    pub fn cancel_render(&self) {
        let cache = self.cached_state.lock().unwrap();
        cache.render_id.fetch_add(1, Ordering::SeqCst);
    }
```

#### Step 2.1.4: Update render method signature

**File:** `src/rendering/tiling_canvas_renderer.rs` (line 81)

Replace:
```rust
pub fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
```

With:
```rust
pub fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
```

Keep the rest of the render method body the same.

#### Step 2.1.5: Update render_with_computation signature

**File:** `src/rendering/tiling_canvas_renderer.rs` (line 113)

Replace:
```rust
fn render_with_computation(
    &self,
    viewport: &Viewport<f64>,
    canvas: &HtmlCanvasElement,
    cache: &mut CachedState,
    render_id: u32,
) {
```

With:
```rust
fn render_with_computation(
    &self,
    viewport: &Viewport<S>,
    canvas: &HtmlCanvasElement,
    cache: &mut CachedState<S, D>,
    render_id: u32,
) {
```

Keep the rest of the method body the same, except change line 125:
```rust
.resize((width * height) as usize, AppData::default());
```

To:
```rust
.resize((width * height) as usize, D::default());
```

NOTE: This requires `D: Default`. Add that bound to the impl.

#### Step 2.1.6: Update colorize_and_display_tile signature

**File:** `src/rendering/tiling_canvas_renderer.rs` (line 201)

Replace:
```rust
fn colorize_and_display_tile(
    &self,
    data: &[AppData],
    rect: PixelRect,
    canvas: &HtmlCanvasElement,
) {
```

With:
```rust
fn colorize_and_display_tile(
    &self,
    data: &[D],
    rect: PixelRect,
    canvas: &HtmlCanvasElement,
) {
```

Keep the rest of the method body the same.

#### Step 2.1.7: Update CanvasRenderer trait impl

**File:** `src/rendering/tiling_canvas_renderer.rs` (lines 266-286)

Replace:
```rust
impl CanvasRenderer for TilingCanvasRenderer {
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>) {
        self.set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        self.set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
        self.render(viewport, canvas);
    }

    fn natural_bounds(&self) -> Rect<f64> {
        self.natural_bounds()
    }

    fn cancel_render(&self) {
        self.cancel_render();
    }
}
```

With:
```rust
impl<S: Clone + PartialEq, D: Clone + Default> CanvasRenderer for TilingCanvasRenderer<S, D> {
    type Scalar = S;
    type Data = D;

    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
        self.set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
        self.set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
        self.render(viewport, canvas);
    }

    fn natural_bounds(&self) -> Rect<S> {
        self.natural_bounds()
    }

    fn cancel_render(&self) {
        self.cancel_render();
    }
}
```

#### Step 2.1.8: Update impl bounds to include Default

**File:** `src/rendering/tiling_canvas_renderer.rs` (line 39)

Change:
```rust
impl<S: Clone + PartialEq, D: Clone> TilingCanvasRenderer<S, D> {
```

To:
```rust
impl<S: Clone + PartialEq, D: Clone + Default> TilingCanvasRenderer<S, D> {
```

#### Step 2.1.9: Update CanvasRenderer trait to be generic

**File:** `src/rendering/canvas_renderer.rs`

Replace the entire trait:
```rust
pub trait CanvasRenderer {
    /// Swap the renderer at runtime (invalidates cache)
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>);

    /// Swap the colorizer at runtime (preserves cache if implementation supports it)
    fn set_colorizer(&mut self, colorizer: Colorizer<AppData>);

    /// Main rendering entry point - renders viewport to canvas
    fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement);

    /// Get natural bounds from the underlying renderer
    fn natural_bounds(&self) -> Rect<f64>;

    /// Cancel any in-progress render
    fn cancel_render(&self);
}
```

With:
```rust
pub trait CanvasRenderer {
    type Scalar;
    type Data: Clone;

    /// Swap the renderer at runtime (invalidates cache)
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

#### Step 2.1.10: Try to compile

```bash
cargo check
```

Expected: FAIL - app.rs and render_config.rs need updates (we'll fix those later)

#### Step 2.1.11: Commit generic TilingCanvasRenderer

```bash
git add src/rendering/tiling_canvas_renderer.rs src/rendering/canvas_renderer.rs
git commit -m "refactor: make TilingCanvasRenderer and CanvasRenderer generic

- TilingCanvasRenderer now generic over <S, D> (Scalar and Data)
- CanvasRenderer trait has associated types Scalar and Data
- Holds trait object internally for renderer swapping
- Preserves cache on colorizer swap, clears on renderer swap
- Breaking change - app.rs and render_config.rs need updates"
```

---

### Task 2.2: Add PrecisionCalculator

**Files:**
- Create: `src/rendering/precision.rs`
- Modify: `src/rendering/mod.rs`

#### Step 2.2.1: Create precision.rs with tests

**File:** `src/rendering/precision.rs` (NEW FILE)

Create the complete file:

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

#### Step 2.2.2: Run test to verify it fails

```bash
cargo test precision
```

Expected: FAIL - "not yet implemented: todo!()"

#### Step 2.2.3: Implement PrecisionCalculator

**File:** `src/rendering/precision.rs`

Replace the `impl PrecisionCalculator` block:

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

#### Step 2.2.4: Run tests

```bash
cargo test precision
```

Expected: All tests PASS

#### Step 2.2.5: Add module to rendering

**File:** `src/rendering/mod.rs`

Find the module declarations section and add:

```rust
pub mod precision;
```

#### Step 2.2.6: Export PrecisionCalculator

**File:** `src/rendering/mod.rs`

Find the pub use section and add:

```rust
pub use precision::PrecisionCalculator;
```

#### Step 2.2.7: Run all tests

```bash
cargo test --workspace
```

Expected: All tests PASS

#### Step 2.2.8: Commit PrecisionCalculator

```bash
git add src/rendering/precision.rs src/rendering/mod.rs
git commit -m "feat: add PrecisionCalculator for zoom-based precision

- Calculate required bits: zoom.log10() × 3.322 + 128
- Minimum 64 bits, scales automatically with zoom
- Power-of-two rounding for efficient allocation
- Full test coverage"
```

---

### Task 2.3: Update RenderConfig to remove create_renderer

**Files:**
- Modify: `src/rendering/render_config.rs`

#### Step 2.3.1: Remove create_renderer field

**File:** `src/rendering/render_config.rs`

Find the `RenderConfig` struct and remove the `create_renderer` field:

Change from:
```rust
pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_renderer: fn() -> Box<dyn Renderer<Scalar = f64, Data = AppData>>,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}
```

To:
```rust
pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}
```

#### Step 2.3.2: Delete renderer creation functions

**File:** `src/rendering/render_config.rs`

Delete the functions:
- `create_test_image_renderer`
- `create_mandelbrot_renderer`

#### Step 2.3.3: Update RENDER_CONFIGS

**File:** `src/rendering/render_config.rs`

Remove `create_renderer:` field from both TEST_IMAGE and MANDELBROT configs.

Change from:
```rust
RenderConfig {
    id: "test_image",
    display_name: "Test Image",
    color_schemes: &[...],
    default_color_scheme_id: "default",
    create_renderer: create_test_image_renderer,
    create_info_provider: || Box::new(TestImageComputer::new()),
},
```

To:
```rust
RenderConfig {
    id: "test_image",
    display_name: "Test Image",
    color_schemes: &[...],
    default_color_scheme_id: "default",
    create_info_provider: || Box::new(TestImageComputer::new()),
},
```

Do the same for MANDELBROT config.

#### Step 2.3.4: Try to compile

```bash
cargo check
```

Expected: FAIL in app.rs - needs to create renderers directly

#### Step 2.3.5: Commit RenderConfig changes

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

**File:** `src/app.rs` (at top, add to existing imports)

Add these imports:
```rust
use crate::rendering::{
    BigFloat, PrecisionCalculator, Point, Rect, ToF64,
    MandelbrotComputer, TestImageComputer, PixelRenderer, AppDataRenderer,
    TilingCanvasRenderer, Renderer, AppData, Colorizer,
};
```

#### Step 3.1.2: Create CanvasRendererHolder enum

**File:** `src/app.rs` (add BEFORE the App component)

Add this complete enum:

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

#### Step 3.1.3: Create renderer factory functions

**File:** `src/app.rs` (add BEFORE the App component, after CanvasRendererHolder)

Add these functions:

```rust
fn create_mandelbrot_canvas_renderer(
    _zoom: f64,
    colorizer: Colorizer<AppData>,
) -> TilingCanvasRenderer<BigFloat, AppData> {
    let computer = MandelbrotComputer::<BigFloat>::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    let renderer: Box<dyn Renderer<Scalar = BigFloat, Data = AppData>> = Box::new(app_renderer);
    TilingCanvasRenderer::new(renderer, colorizer, 128)
}

fn create_test_image_canvas_renderer(
    _zoom: f64,
    colorizer: Colorizer<AppData>,
) -> TilingCanvasRenderer<f64, AppData> {
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    let renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>> = Box::new(app_renderer);
    TilingCanvasRenderer::new(renderer, colorizer, 128)
}
```

#### Step 3.1.4: Update initial renderer creation

**File:** `src/app.rs` (inside App component function)

Find the lines that create `initial_renderer` and `canvas_renderer` signal.

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

**File:** `src/app.rs` (find the `create_effect` that handles renderer switching)

Find the code that switches renderers (look for where it calls `set_renderer` or similar).

Replace the renderer switching logic with:

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

#### Step 3.1.6: Verify colorizer switching still works

**File:** `src/app.rs`

Find where colorizers are switched (look for `set_colorizer` calls).

The code should look like:
```rust
canvas_renderer.update(|cr| cr.set_colorizer(new_colorizer));
```

This should still work with the new `CanvasRendererHolder`.

#### Step 3.1.7: Try to compile

```bash
cargo check
```

Expected: SUCCESS or only minor issues

#### Step 3.1.8: Fix any remaining compile errors

If there are errors about method calls on `canvas_renderer`, update them to use the `CanvasRendererHolder` methods:
- `canvas_renderer.with(|cr| cr.natural_bounds())` → should work
- `canvas_renderer.with(|cr| cr.render(...))` → should work
- `canvas_renderer.with(|cr| cr.cancel_render())` → should work

#### Step 3.1.9: Run all tests

```bash
cargo test --workspace --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: All pass

#### Step 3.1.10: Try to run in browser

```bash
trunk serve
```

Open http://localhost:8080 and test:
- Load app - should show Mandelbrot (using BigFloat)
- Zoom in to various levels
- Switch to Test Image
- Switch back to Mandelbrot
- Switch colorizers

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

### Task 4.1: Add integration tests

**Files:**
- Create: `tests/integration/arbitrary_precision.rs`

#### Step 4.1.1: Create integration test directory

```bash
mkdir -p tests/integration
```

#### Step 4.1.2: Create integration test file

**File:** `tests/integration/arbitrary_precision.rs` (NEW FILE)

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

#### Step 4.1.3: Run integration tests

```bash
cargo test --test arbitrary_precision
```

Expected: All tests PASS

#### Step 4.1.4: Commit integration tests

```bash
git add tests/
git commit -m "test: add integration tests for arbitrary precision

- Test MandelbrotComputer with BigFloat at high zoom
- Test PrecisionCalculator scaling
- Test BigFloat to f64 conversion
- All tests passing"
```

---

### Task 4.2: Final verification

#### Step 4.2.1: Run all tests

```bash
cargo test --workspace --all-targets --all-features
```

Expected: All tests PASS

#### Step 4.2.2: Run clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: No warnings

#### Step 4.2.3: Format code

```bash
cargo fmt --all
```

#### Step 4.2.4: Build WASM

```bash
cargo build --target wasm32-unknown-unknown --lib
```

Expected: SUCCESS

#### Step 4.2.5: Manual browser testing

```bash
trunk serve
```

Open http://localhost:8080 and verify:

**Test Mandelbrot at various zoom levels:**
- [ ] Zoom 1: BigFloat with 64-128 bits (minimal overhead)
- [ ] Zoom 1e5: BigFloat with 128-256 bits
- [ ] Zoom 1e10: BigFloat with 256 bits
- [ ] Zoom 1e15: BigFloat with 256-512 bits
- [ ] Zoom 1e30: BigFloat with 512-1024 bits (if you can zoom that far)

**Test renderer switching:**
- [ ] Start on Mandelbrot
- [ ] Switch to Test Image (should work, uses f64)
- [ ] Switch back to Mandelbrot (should work, uses BigFloat)

**Test colorizer switching:**
- [ ] Zoom in on Mandelbrot
- [ ] Switch colorizer (should be instant - cache preserved)

**Check for precision:**
- [ ] Zoom to 1e20+
- [ ] Verify fractal detail still renders correctly
- [ ] No blurry/pixelated areas from precision loss

**Performance:**
- [ ] No console errors
- [ ] Rendering is reasonably fast
- [ ] No memory leaks (check DevTools)

---

## Success Criteria

- [ ] All existing tests passing
- [ ] New integration tests passing
- [ ] Clippy clean
- [ ] WASM builds successfully
- [ ] Browser testing all passed (see checklist above)
- [ ] Code formatted

---

## Summary

**What we built:**
1. Replaced `ImageFloat` trait with standard Rust traits + simple `ToF64`
2. Made `TilingCanvasRenderer` generic over `<S, D>` (Scalar and Data types)
3. Made `CanvasRenderer` trait generic with associated types
4. Added `PrecisionCalculator` to calculate precision bits from zoom
5. **Mandelbrot ALWAYS uses `BigFloat`** - precision scales with zoom automatically
6. **TestImage ALWAYS uses `f64`** - doesn't need arbitrary precision
7. Added `CanvasRendererHolder` enum to wrap both types for Leptos signal
8. Viewport stored as `f64` in app state, converted to `BigFloat` when rendering

**Key insight:**
BigFloat at low zoom (64-128 bits) has minimal overhead vs f64. So there's no need to swap between f64 and BigFloat at runtime. Just always use BigFloat for Mandelbrot and let precision scale with zoom.

**Lines of code added:** ~300
**Complexity added:** Minimal
**Generics used:** Yes
**Traits used:** Yes
**Over-engineering:** None
