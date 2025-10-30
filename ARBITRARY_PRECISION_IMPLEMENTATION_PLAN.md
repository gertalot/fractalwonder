# Implementation Plan: Arbitrary Precision Mandelbrot Set Rendering

## Objective
Enable the Mandelbrot set explorer to use arbitrary precision decimals for deep zoom rendering (up to 10^100 and beyond) while maintaining full WASM compatibility.

## Requirements
1. Use a WASM-compatible arbitrary precision library
2. Support existing f64 rendering (backward compatibility)
3. Automatically switch precision based on zoom level
4. All image-space computations must use generic types
5. All tests must pass
6. Full WASM compilation support

## Executive Summary

**What's Been Done (Phase 1 - Foundation):**
- ✅ Added `dashu` library for arbitrary precision (pure Rust, WASM-compatible)
- ✅ Created `ImageFloat` trait ⚠️ **NEEDS REMOVAL - redundant with Point<T> constraints**
- ✅ Implemented `BigFloat` wrapper with precision management
- ✅ Made `MandelbrotComputer` generic over numeric type `T`
- ✅ Renamed associated type from `Coord` to `Scalar` (clearer semantics)
- ✅ All 89 tests passing, clippy clean, WASM builds successfully

**What's Next (Phases 2-6):**
- **Phase 2 Task 2.0** ⚠️ **START HERE**: Remove ImageFloat, use standard Rust traits + simple ToF64
- **Phase 2 Rest**: Make rendering pipeline support dynamic precision switching
- **Phase 3**: Integrate into App component with automatic f64↔BigFloat transitions
- **Phase 4**: Add optimization for smooth precision transitions
- **Phase 5**: Comprehensive testing (unit, integration, WASM, browser)
- **Phase 6**: Documentation and cleanup

**Current Architecture:**
```
Point<Scalar>          Scalar = f64 or BigFloat
    └─ Scalar          Currently: f64 only in pipeline
                       Target: Dynamic based on zoom level
```

**Key Innovation:**
Auto-switching renderer that uses f64 for zoom < 1e10, BigFloat for deeper zoom, with precision calculated as `precision_bits = zoom.log10() × 3.322 + 128`.

**Status:** 8/26 tasks complete. Foundation is solid. Ready to implement pipeline integration.

---

## Phase 1: Foundation (COMPLETED ✓)

### Task 1.1: Add Dependencies ✓
**File**: `Cargo.toml`
**What**: Added dashu crates for arbitrary precision
**Changes**:
```toml
dashu = "0.4"
dashu-float = "0.4"
```
**Why dashu**: Pure Rust, full WASM support, no C dependencies, no_std compatible

### Task 1.2: Create ImageFloat Trait ✓ (TO BE REMOVED)
**File**: `src/rendering/numeric.rs` (NEW FILE)
**What**: Created trait abstraction for numeric types in image space
**Implementation**:
- `ImageFloat` trait with operations: from_f64, from_i32, to_f64, mul, add, sub, div, gt
- Implemented for f64 (standard precision)
- Full test coverage

**IMPORTANT NOTE**: This trait is **REDUNDANT** and should be removed in Phase 2.

**Why it's redundant:**
- `Point<T>` already defines required operations via trait bounds on its methods
- Standard Rust traits (`Add`, `Sub`, `Mul`, `Div`, `From<f64>`, `PartialOrd`) already exist
- Only missing piece: conversion to f64 for display (can add simple `ToF64` trait)

**The correct approach:**
- `BigFloat` should implement standard Rust operators (`Add`, `Sub`, `Mul`, `Div`)
- `BigFloat` should implement `From<f64>` for conversions from literals
- Add a simple `ToF64` trait for display conversions
- Use the constraints that `Point<T>` methods already define

**Why we implemented it anyway:**
- Built it before understanding existing architecture
- Phase 1 complete with it, so leaving for now
- Will refactor in Phase 2

### Task 1.3: Create BigFloat Type ✓
**File**: `src/rendering/numeric.rs`
**What**: Wrapper around dashu's FBig with precision management
**Implementation**:
- BigFloat struct with value and precision_bits
- All arithmetic operators (Add, Sub, Mul, Div)
- Scalar operations (Mul<f64>, Div<f64>)
- PartialEq, PartialOrd, From<f64>
- ImageFloat trait implementation
- Comprehensive test coverage

**Verification**: All tests pass (89 total tests)

### Task 1.4: Make MandelbrotComputer Generic ✓
**File**: `src/rendering/computers/mandelbrot.rs`
**What**: Made MandelbrotComputer generic over numeric type
**Implementation**:
```rust
pub struct MandelbrotComputer<T = f64> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: ImageFloat + From<f64>,
{
    type Coord = T;
    // ...
}
```
**Changes**:
- All f64 literals converted to T::from_f64()
- All arithmetic uses ImageFloat trait methods
- Default type parameter maintains backward compatibility
- Added tests for both f64 and BigFloat

**Verification**: All tests pass including BigFloat-specific tests

### Task 1.5: Update Point Scalar Operations ✓
**File**: `src/rendering/points.rs`
**What**: Changed mul_scalar and div_scalar to use generic &T
**Before**: `pub fn mul_scalar(&self, scalar: f64)`
**After**: `pub fn mul_scalar(&self, scalar: &T)`
**Impact**: Allows arbitrary precision scalars in Point operations

### Task 1.6: Export Public API ✓
**File**: `src/rendering/mod.rs`
**What**: Added BigFloat to public exports
```rust
pub use numeric::{BigFloat, ImageFloat};
```

### Task 1.7: Rename Associated Type from Coord to Scalar ✓
**Files**: 13 files across entire rendering pipeline
**What**: Renamed `type Coord` to `type Scalar` for better semantic clarity
**Why**: Following Rust naming conventions - associated types should be descriptive (like `Item`, `Output`) not abbreviations. "Scalar" clearly indicates the scalar numeric type used inside `Point<T>`, whereas "Coord" was ambiguous (coordinate structure vs numeric type).

**Files modified**:
- All trait definitions: `ImagePointComputer`, `Renderer`, `RendererInfo`, `CanvasRenderer`
- All implementations: `MandelbrotComputer`, `TestImageComputer`, `PixelRenderer`, `AppDataRenderer`, `TiledRenderer`, `TilingCanvasRenderer`
- Trait objects in `RenderConfig`, `canvas_renderer.rs`

**Verification**: All 89 tests passing, clippy clean

### Task 1.8: Final Phase 1 Verification ✓
**Commands run**:
- `cargo fmt --all` ✓
- `cargo test --workspace --all-targets --all-features` ✓ (89 tests passing)
- `cargo clippy --all-targets --all-features -- -D warnings` ✓
- `cargo check --workspace --all-targets --all-features` ✓
- `cargo build --target wasm32-unknown-unknown --lib` ✓

---

## Phase 2: Rendering Pipeline Integration (TO DO)

### Background
Currently, the entire rendering pipeline hardcodes `Scalar = f64`:
- RenderConfig::create_renderer returns `Box<dyn Renderer<Scalar = f64>>`
- App component uses f64 throughout
- All renderer implementations are f64-only

To enable arbitrary precision rendering, we need to make the pipeline generic.

### Task 2.0: Remove ImageFloat and Use Standard Traits (NEW - FIRST TASK)
**Files**:
- `src/rendering/numeric.rs` - Simplify or remove
- `src/rendering/computers/mandelbrot.rs` - Update to use standard operators
- Any other files using ImageFloat trait methods

**What**: Refactor to use standard Rust traits instead of ImageFloat

**Current (wrong):**
```rust
impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: ImageFloat + From<f64>,
{
    let zx = T::from_f64(0.0);
    let zx_sq = ImageFloat::mul(&zx, &zx);
}
```

**Target (correct):**
```rust
pub trait ToF64 {
    fn to_f64(&self) -> f64;
}

impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: Clone + From<f64> + ToF64
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Mul<f64, Output = T>  // For scalar operations in transforms
        + Div<f64, Output = T>
        + PartialOrd,
{
    let zx = T::from(0.0);               // Standard From trait
    let zx_sq = zx.clone() * zx.clone(); // Standard Mul operator

    // For display:
    let x_display = zx.to_f64();  // Our simple ToF64 trait
}
```

**Why**:
- `Point<T>` methods already define these trait bounds
- Standard Rust operators are idiomatic
- Removes entire layer of unnecessary abstraction
- `ToF64` is the ONLY non-standard trait we need

**Implementation steps:**
1. Create simple `ToF64` trait in numeric.rs
2. Implement `ToF64` for `f64` and `BigFloat`
3. Update `MandelbrotComputer` to use standard operators
4. Remove `ImageFloat` trait (or keep as type alias for convenience)
5. Update all tests

**Verification:**
```bash
cargo test rendering::computers::mandelbrot
cargo test rendering::numeric
```

### Task 2.1: Make Renderer Trait Implementations Generic
**Files**:
- `src/rendering/app_data_renderer.rs`
- `src/rendering/pixel_renderer.rs`
- `src/rendering/tiled_renderer.rs`
- `src/rendering/tiling_canvas_renderer.rs`

**What**: Update all Renderer trait implementations to be generic over Scalar type

**Current state**:
```rust
impl Renderer for AppDataRenderer<R>
where
    R: Renderer<Scalar = f64, Data = D>,
```

**Target state**:
```rust
impl<T> Renderer for AppDataRenderer<R, T>
where
    R: Renderer<Scalar = T, Data = D>,
    T: ImageFloat + Serialize + Deserialize,
```

**Why**: Allows these renderers to work with any ImageFloat type, not just f64

**Verification**:
```bash
cargo test --lib rendering::app_data_renderer
cargo test --lib rendering::pixel_renderer
cargo test --lib rendering::tiled_renderer
```

### Task 2.2: Update CanvasRenderer Trait
**File**: `src/rendering/canvas_renderer.rs`

**What**: Make CanvasRenderer generic over Coord type

**Current**:
```rust
pub trait CanvasRenderer {
    fn natural_bounds(&self) -> Rect<f64>;
    fn render(&mut, ...);
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Coord = f64, Data = AppData>>);
}
```

**Target**:
```rust
pub trait CanvasRenderer {
    type Scalar: ImageFloat;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
    fn render(&mut, ...);
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Coord = Self::Coord, Data = AppData>>);
}
```

**Why**: Removes f64 coupling from the canvas rendering abstraction

**Impact**: TilingCanvasRenderer implementation must be updated

### Task 2.3: Implement Precision Calculator
**File**: `src/rendering/precision.rs` (NEW FILE)

**What**: Create precision calculation logic based on zoom level

**Implementation**:
```rust
pub struct PrecisionCalculator;

impl PrecisionCalculator {
    /// Calculate required precision bits for given zoom level
    /// Formula: precision_bits = max(64, zoom.log10() * 3.322 + 128)
    pub fn calculate_precision_bits(zoom: f64) -> usize {
        if zoom <= 1e10 {
            // For moderate zoom, use f64
            64
        } else {
            // Each decimal digit requires ~3.322 bits
            let zoom_digits = zoom.log10();
            let required_bits = (zoom_digits * 3.322 + 128.0) as usize;
            required_bits.max(128).next_power_of_two()
        }
    }

    /// Determine if arbitrary precision is needed for given zoom
    pub fn needs_arbitrary_precision(zoom: f64) -> bool {
        zoom > 1e10
    }
}
```

**Why**: Automatically determines when to switch from f64 to BigFloat

**Tests**:
- Test zoom < 1e10 returns 64 bits
- Test zoom = 1e15 returns appropriate precision
- Test zoom = 1e50 returns appropriate precision
- Test zoom = 1e100 returns appropriate precision

**Verification**:
```bash
cargo test rendering::precision
```

### Task 2.4: Create Dual-Mode Renderer Factory
**File**: `src/rendering/renderer_factory.rs` (NEW FILE)

**What**: Factory that creates either f64 or BigFloat renderers based on zoom

**Implementation**:
```rust
pub enum DynamicRenderer {
    F64(Box<dyn CanvasRenderer<Coord = f64>>),
    BigFloat(Box<dyn CanvasRenderer<Coord = BigFloat>>),
}

impl DynamicRenderer {
    pub fn create_mandelbrot(zoom: f64, colorizer: Colorizer<AppData>) -> Self {
        if PrecisionCalculator::needs_arbitrary_precision(zoom) {
            let precision = PrecisionCalculator::calculate_precision_bits(zoom);
            let computer = MandelbrotComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
            let canvas_renderer = TilingCanvasRenderer::new(app_renderer, colorizer, 128);
            DynamicRenderer::BigFloat(Box::new(canvas_renderer))
        } else {
            let computer = MandelbrotComputer::<f64>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
            let canvas_renderer = TilingCanvasRenderer::new(app_renderer, colorizer, 128);
            DynamicRenderer::F64(Box::new(canvas_renderer))
        }
    }

    pub fn render(&mut self, viewport: DynamicViewport, ...) -> Vec<u32> {
        match (self, viewport) {
            (DynamicRenderer::F64(r), DynamicViewport::F64(v)) => r.render(v, ...),
            (DynamicRenderer::BigFloat(r), DynamicViewport::BigFloat(v)) => r.render(v, ...),
            _ => panic!("Viewport type mismatch"),
        }
    }
}

pub enum DynamicViewport {
    F64(Viewport<f64>),
    BigFloat(Viewport<BigFloat>),
}
```

**Why**: Abstracts away the complexity of choosing precision at runtime

**Verification**:
```bash
cargo test rendering::renderer_factory
```

### Task 2.5: Update Viewport for Serialization
**File**: `src/rendering/viewport.rs`

**Current**: `Viewport<T: Clone>` with Serialize/Deserialize

**Challenge**: BigFloat cannot implement Serialize/Deserialize directly

**Solution**: Add conversion methods
```rust
impl Viewport<BigFloat> {
    pub fn to_f64_viewport(&self) -> Viewport<f64> {
        Viewport::new(
            Point::new(self.center.x().to_f64(), self.center.y().to_f64()),
            self.zoom
        )
    }
}

impl Viewport<f64> {
    pub fn to_bigfloat_viewport(&self, precision_bits: usize) -> Viewport<BigFloat> {
        Viewport::new(
            Point::new(
                BigFloat::with_precision(*self.center.x(), precision_bits),
                BigFloat::with_precision(*self.center.y(), precision_bits)
            ),
            self.zoom
        )
    }
}
```

**Why**: Allows serialization for localStorage while supporting arbitrary precision internally

**Verification**:
```bash
cargo test rendering::viewport
```

### Task 2.6: Update RenderConfig
**File**: `src/rendering/render_config.rs`

**What**: Update RenderConfig to support dynamic precision

**Before**:
```rust
pub struct RenderConfig {
    pub create_renderer: fn() -> Box<dyn Renderer<Coord = f64, Data = AppData>>,
}
```

**After**:
```rust
pub struct RenderConfig {
    pub create_renderer: fn(zoom: f64) -> DynamicRenderer,
}

fn create_mandelbrot_renderer(zoom: f64) -> DynamicRenderer {
    // Get color scheme from context (requires refactor)
    DynamicRenderer::create_mandelbrot(zoom, colorizer)
}
```

**Challenge**: Colorizer needs to be passed separately

**Revised approach**:
```rust
pub create_renderer: fn(zoom: f64, colorizer: Colorizer<AppData>) -> DynamicRenderer,
```

**Verification**:
```bash
cargo test rendering::render_config
```

---

## Phase 3: Application Integration (TO DO)

### Task 3.1: Update App Component State
**File**: `src/app.rs`

**What**: Update App component to use DynamicRenderer

**Changes**:
1. Change canvas_renderer type from `Box<dyn CanvasRenderer>` to `DynamicRenderer`
2. Update renderer creation in effects to pass zoom level
3. Add precision transition logic when zoom crosses threshold

**Implementation details**:
```rust
// Create effect that monitors zoom and switches precision
create_effect(move |_| {
    let vp = viewport.get();
    let needs_bigfloat = PrecisionCalculator::needs_arbitrary_precision(vp.zoom);
    let current_is_bigfloat = canvas_renderer.with(|r| matches!(r, DynamicRenderer::BigFloat(_)));

    if needs_bigfloat != current_is_bigfloat {
        // Precision threshold crossed - recreate renderer
        let renderer_id = selected_renderer_id.get_untracked();
        let config = get_config(&renderer_id).unwrap();
        let states = renderer_states.get_untracked();
        let state = states.get(&renderer_id).unwrap();
        let colorizer = get_color_scheme(config, &state.color_scheme_id).unwrap().colorizer;

        let new_renderer = (config.create_renderer)(vp.zoom, colorizer);
        canvas_renderer.set(new_renderer);
    }
});
```

**Why**: Automatically switches between f64 and BigFloat as user zooms

**Verification**:
- Manual test: zoom to 1e10, verify no precision loss
- Manual test: zoom to 1e20, verify BigFloat activated
- Check browser console for precision transition messages

### Task 3.2: Update State Serialization
**File**: `src/state/app_state.rs`

**What**: Ensure viewport serialization always uses f64

**Current**: `viewport: Viewport<f64>` in RendererState

**Action**: No changes needed - viewport is already stored as f64

**Note**: Internal BigFloat viewport is ephemeral, always converted from/to f64 for storage

**Why**: localStorage can't handle arbitrary precision; f64 is sufficient for restore

### Task 3.3: Add Precision Display to UI
**File**: `src/rendering/renderer_info.rs`

**What**: Add precision information to renderer info

**Changes in MandelbrotComputer::info()**:
```rust
fn info(&self, viewport: &Viewport<T>) -> RendererInfoData {
    let max_iterations = calculate_max_iterations(viewport.zoom);
    let precision_info = if std::any::type_name::<T>().contains("BigFloat") {
        let precision_bits = PrecisionCalculator::calculate_precision_bits(viewport.zoom);
        let precision_digits = (precision_bits as f64 / 3.322) as usize;
        format!("Arbitrary ({} digits)", precision_digits)
    } else {
        "Standard (f64)".to_string()
    };

    RendererInfoData {
        name: "Mandelbrot".to_string(),
        custom_params: vec![
            ("Max Iterations".to_string(), max_iterations.to_string()),
            ("Precision".to_string(), precision_info),
        ],
        // ...
    }
}
```

**Why**: Users should see when arbitrary precision is active

**Verification**: UI shows "Precision: Arbitrary (77 digits)" when zoomed past threshold

---

## Phase 4: Optimization (TO DO)

### Task 4.1: Implement Precision Transition Smoothing
**File**: `src/rendering/precision_transition.rs` (NEW FILE)

**What**: Smooth viewport conversion when switching precision

**Implementation**:
```rust
pub struct PrecisionTransition;

impl PrecisionTransition {
    /// Convert viewport from one precision to another with error correction
    pub fn convert_viewport<From, To>(
        viewport: &Viewport<From>,
        to_precision_bits: usize
    ) -> Viewport<To>
    where
        From: ImageFloat,
        To: ImageFloat,
    {
        // Convert via f64 as intermediate
        let f64_vp = Viewport::new(
            Point::new(viewport.center.x().to_f64(), viewport.center.y().to_f64()),
            viewport.zoom
        );

        // Convert to target precision
        Viewport::new(
            Point::new(
                To::from_f64(*f64_vp.center.x()),
                To::from_f64(*f64_vp.center.y())
            ),
            viewport.zoom
        )
    }
}
```

**Why**: Prevents jarring jumps when crossing precision threshold

**Verification**:
```bash
cargo test rendering::precision_transition
```

### Task 4.2: Add Perturbation Theory Support (FUTURE)
**File**: `src/rendering/perturbation.rs` (NEW FILE)

**What**: Implement perturbation theory for extreme zoom performance

**Why**: At extreme zoom (>1e50), naive iteration becomes slow even with SIMD
Perturbation theory allows computing most pixels using f64 deltas from a reference point

**Status**: Documented for future implementation
**Complexity**: High - requires series approximation and glitch detection

**Reference**: https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Perturbation_theory

---

## Phase 5: Testing & Validation (TO DO)

### Task 5.1: Unit Tests
**Files**: Throughout codebase

**Test coverage required**:
- [ ] ImageFloat trait for f64 ✓ (already done)
- [ ] ImageFloat trait for BigFloat ✓ (already done)
- [ ] MandelbrotComputer with f64 ✓ (already done)
- [ ] MandelbrotComputer with BigFloat ✓ (already done)
- [ ] PrecisionCalculator logic
- [ ] DynamicRenderer creation
- [ ] Viewport conversion between precisions
- [ ] Renderer factory switching

**Verification**:
```bash
cargo test --workspace --all-targets --all-features
```

**Success criteria**: All tests pass, >90% code coverage for new code

### Task 5.2: Integration Tests
**File**: `tests/integration/arbitrary_precision.rs` (NEW FILE)

**Test scenarios**:
1. Create f64 renderer, render a tile
2. Create BigFloat renderer, render same tile
3. Verify pixel outputs are equivalent (within tolerance)
4. Test precision transition (f64 -> BigFloat -> f64)
5. Test extreme zoom (1e100)

**Verification**:
```bash
cargo test --test arbitrary_precision
```

### Task 5.3: WASM Tests
**What**: Verify arbitrary precision works in WASM environment

**Commands**:
```bash
wasm-pack test --headless --chrome
```

**Test cases**:
- BigFloat arithmetic in WASM
- Mandelbrot computation with BigFloat in WASM
- Renderer creation and rendering in WASM

**Success criteria**: All WASM tests pass

### Task 5.4: Browser Manual Testing
**What**: End-to-end testing in actual browser

**Test procedure**:
1. `trunk serve`
2. Open localhost:8080
3. Zoom to 1e5 - verify smooth rendering
4. Continue to 1e10 - verify precision switch (check console)
5. Zoom to 1e15 - verify no precision loss, details still sharp
6. Zoom to 1e20 - verify arbitrary precision active (check UI)
7. Zoom to 1e50 - verify rendering still works (may be slow)
8. Pan around at extreme zoom - verify stability
9. Zoom back out - verify switch back to f64
10. Check for memory leaks (DevTools memory profiler)

**Success criteria**:
- No crashes
- Smooth transitions
- Details remain sharp at all zoom levels
- Memory usage stable

### Task 5.5: Performance Benchmarks
**File**: `benches/arbitrary_precision.rs` (NEW FILE)

**What**: Benchmark rendering performance at various zoom levels

**Benchmarks**:
- f64 rendering at zoom 1e5
- BigFloat rendering at zoom 1e15 (128-bit precision)
- BigFloat rendering at zoom 1e30 (256-bit precision)
- BigFloat rendering at zoom 1e50 (512-bit precision)

**Success criteria**:
- f64 performance unchanged
- BigFloat at 128-bit within 2x of f64
- BigFloat at 256-bit within 5x of f64
- BigFloat at 512-bit within 10x of f64

**Run with**:
```bash
cargo bench --bench arbitrary_precision
```

---

## Phase 6: Documentation & Cleanup (TO DO)

### Task 6.1: Update CLAUDE.md
**File**: `/workspace/CLAUDE.md`

**What**: Document arbitrary precision architecture

**Additions**:
```markdown
## ARBITRARY PRECISION

The Mandelbrot renderer supports arbitrary precision for deep zoom:

- **Standard zoom (1 - 1e10)**: Uses f64 (fast, hardware-accelerated)
- **Deep zoom (>1e10)**: Automatically switches to dashu BigFloat
- **Precision calculation**: `precision_bits = zoom.log10() × 3.322 + 128`

The renderer automatically switches between f64 and BigFloat based on zoom level,
with no user intervention required.

Key types:
- `ImageFloat` trait: Abstraction for numeric types in image space
- `BigFloat`: Wrapper around dashu FBig with precision management
- `DynamicRenderer`: Enum that holds either f64 or BigFloat renderer

All image-space coordinates use generic `T: ImageFloat` types.
Pixel-space coordinates always use `f64`.
```

### Task 6.2: Add Code Comments
**Files**: All modified files

**What**: Ensure all new code has clear comments

**Focus areas**:
- Why we use dashu instead of rug
- How precision is calculated
- When precision switching occurs
- Performance implications of BigFloat

**Verification**: `cargo doc --open` generates clear documentation

### Task 6.3: Update Tests Documentation
**File**: `/workspace/TESTING.md` (NEW FILE)

**What**: Document how to test arbitrary precision

**Contents**:
```markdown
# Testing Arbitrary Precision

## Unit Tests
cargo test rendering::numeric
cargo test rendering::computers::mandelbrot

## Integration Tests
cargo test --test arbitrary_precision

## WASM Tests
wasm-pack test --headless --chrome

## Browser Manual Tests
See CLAUDE.md "Browser Manual Testing" section

## Benchmarks
cargo bench --bench arbitrary_precision
```

### Task 6.4: Clean Up Temporary Files
**What**: Remove this planning document after implementation

**Files to remove**:
- `/workspace/ARBITRARY_PRECISION_IMPLEMENTATION_PLAN.md`

### Task 6.5: Final Code Review
**What**: Review all changes against coding standards

**Checklist**:
- [ ] Line length ≤ 120 characters
- [ ] 4-space indentation
- [ ] Strong types, explicit error handling
- [ ] No clippy warnings
- [ ] Formatted with rustfmt
- [ ] No "new", "legacy", "updated" in names/comments
- [ ] Comments explain "why" not "what"
- [ ] No placeholders or TODOs in code
- [ ] All tests pass
- [ ] WASM compilation successful

**Verification**:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo build --target wasm32-unknown-unknown --lib
wasm-pack test --headless --chrome
```

---

## Summary

**Total phases**: 6
**Total tasks**: 27 (added Task 2.0 to fix ImageFloat)
**Completed**: 8 tasks (Phase 1) ✓
**Remaining**: 19 tasks (Phases 2-6)
**CRITICAL FIRST STEP**: Task 2.0 - Remove ImageFloat, use standard Rust traits

**Estimated complexity**:
- Phase 1 (Foundation): ✓ COMPLETED
- Phase 2 (Rendering Pipeline): MEDIUM - requires careful refactoring
- Phase 3 (App Integration): MEDIUM - state management complexity
- Phase 4 (Optimization): LOW - mostly nice-to-have
- Phase 5 (Testing): MEDIUM - comprehensive coverage needed
- Phase 6 (Documentation): LOW - cleanup and docs

**Critical path**:
1. Phase 2.1-2.2: Make rendering pipeline generic
2. Phase 2.3-2.4: Add precision calculation and factory
3. Phase 3.1: Update App component
4. Phase 5: Validate everything works

**Risk areas**:
- Trait object complications with generic associated types
- Potential performance regression from dynamic dispatch
- WASM binary size increase from dashu dependency
- State serialization with mixed precision types

**Success criteria**:
- All tests pass (unit, integration, WASM)
- No clippy warnings
- Successful WASM build
- Smooth rendering at zoom levels 1 to 1e50+
- No memory leaks or crashes
- Clear precision mode indication in UI
