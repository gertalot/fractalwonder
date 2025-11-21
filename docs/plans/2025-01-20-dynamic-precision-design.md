# Dynamic Precision for Extreme Zoom - Design Document

**Date:** 2025-01-20
**Status:** Design Complete

## Problem Statement

The current architecture stores viewport as `Viewport<f64>` throughout the application. This is fundamentally broken for extreme zoom levels because:

1. **f64 precision limit**: ~15-17 decimal digits (53 bits mantissa)
2. **At zoom 1e10**: Viewport width = 3.5e-10, pixel spacing = 1.8e-13
   - f64 spacing at center (-0.5) = 5.5e-17
   - Ratio: f64 is 3000x more precise than needed ✅
3. **At zoom 1e15**: Viewport width = 3.5e-15, pixel spacing = 1.8e-18
   - f64 spacing at center = 5.5e-17
   - Ratio: f64 is 30x LESS precise than needed ❌

**The core issue**: Converting f64 → BigFloat at render time doesn't recover lost precision. The viewport center coordinates themselves must be BigFloat from the start.

## Architecture Principles

### Layer Structure

- **CORE**: Foundational types and utilities. Cannot import from anywhere
- **UI**: Leptos web app, user interaction, worker coordination, pushing pixels onto a canvas. Imports from CORE only
- **COMPUTE**: Pure computational engine. Imports from CORE only. No DOM, no UI code
- **UI and COMPUTE are siblings** - they don't directly import from each other
- Communication between UI and COMPUTE happens via web workers (message passing)

### Separation of Concerns

**Pixel Space vs Fractal Space:**
- **Pixel space**: Mouse coords, canvas dimensions, drag offsets - f64 is fine
- **Fractal space**: Renderer and Compute (e.g. test image and mandelbrot) coordinates - needs BigFloat with calculate
  precision
- Transform functions convert pixel → fractal using precision parameter

---

## Solution Design

### 1. Precision Calculation (in CORE)

**Location:** `fractalwonder-core/src/precision.rs` (new file)

```rust
/// Calculate maximum iterations based on zoom level
/// CURRENTLY IN MANDELBROT.RS - move to core.
pub fn calculate_max_iterations(zoom: f64) -> u32 {
    let base = 50.0;
    let k = 100.0;
    let power = 1.5;
    let iterations = base + k * zoom.log10().powf(power);
    iterations.clamp(50.0, 10000.0) as u32
}

/// Calculate required precision bits for viewport at given zoom
pub fn required_precision_bits(
    viewport_width: f64,
    canvas_width: usize,
    max_iter: u32
) -> usize {
    let delta = viewport_width / (canvas_width as f64);
    let bits_for_spacing = (-delta.log2()).ceil() as usize;
    let bits_for_iterations = (max_iter as f64).log2().ceil() as usize;
    let safety_margin = 32;
    bits_for_spacing + bits_for_iterations + safety_margin
}
```

**Key insight**: Precision depends only on zoom level and canvas dimensions:
- `viewport_width` is derivable from zoom + natural_bounds
- `max_iter` is derivable from zoom
- Therefore: `precision = f(zoom, canvas_size, natural_bounds)`

### 2. Viewport with Context (in CORE)

**Pattern**: Following codebase convention of free functions rather than methods.

**Location:** `fractalwonder-core/src/viewport.rs` (update)

```rust
pub struct Viewport<T> {
    pub center: Point<T>,
    pub zoom: f64,
    pub natural_bounds: Rect<T>,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

impl<T: Clone> Viewport<T> {
    pub fn new(
        center: Point<T>,
        zoom: f64,
        natural_bounds: Rect<T>,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Self {
        Self { center, zoom, natural_bounds, canvas_width, canvas_height }
    }
}
```

**Location:** `fractalwonder-core/src/precision.rs` (new file)

```rust
/// Calculate precision for a viewport
pub fn calculate_viewport_precision_bits(viewport: &Viewport<BigFloat>) -> usize {
    let viewport_width = viewport.natural_bounds.width() / viewport.zoom;
    let max_iter = calculate_max_iterations(viewport.zoom);
    required_precision_bits(viewport_width, viewport.canvas_width as usize, max_iter)
}
```

**Location:** `fractalwonder-core/src/transforms.rs` (update)

```rust
/// Apply pixel transform with automatic precision management
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport<BigFloat>,
    transform: &TransformResult,
) -> Viewport<BigFloat> {
    let new_zoom = viewport.zoom * transform.zoom_factor;

    // Calculate precision for new zoom
    let precision = calculate_viewport_precision_bits_for_zoom(viewport, new_zoom);

    // Create natural bounds with appropriate precision
    let natural_bounds_bf = /* ... */;

    // Perform transformation
    // ... existing transform logic ...
}
```

### 3. Integration Point (in interactive_canvas.rs)

**Current code** (lines 19-35):
```rust
let interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
    if let Some(canvas_el) = canvas_ref.get_untracked() {
        let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
        let width = canvas.width();
        let height = canvas.height();

        let current_vp = viewport.get_untracked();
        let new_vp = crate::rendering::apply_pixel_transform_to_viewport(
            &current_vp,
            &natural_bounds.get_untracked(),
            &transform_result,
            width,
            height,
        );
        set_viewport(new_vp);
    }
});
```

**New code**:
```rust
let interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
    let current_vp = viewport.get_untracked();
    let new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform_result);
    set_viewport(new_vp);
});
```

## Implementation Plan

Each task delivers working, tested, incrementally improved functionality.

### Task 1: Add Precision Calculation to CORE

**Goal:** Foundational infrastructure - precision calculation functions

**Changes:**
- Create `fractalwonder-core/src/precision.rs`
  - Move `calculate_max_iterations` from `mandelbrot.rs`
  - Add `required_precision_bits(viewport_width, canvas_width, max_iter)` function
- Update `fractalwonder-core/src/lib.rs` to export precision module
- Update `fractalwonder-compute/src/computers/mandelbrot.rs` to import from core
- Update `fractalwonder-compute/src/precision.rs` to import from core

**Verification:**
```bash
cargo test --workspace
cargo clippy --workspace
```

**Browser test:** Works exactly the same (no functional change)

---

### Task 2: Extend Viewport with Context Fields

**Goal:** Viewport carries all data needed for precision calculation

**Changes:**
- Update `fractalwonder-core/src/viewport.rs`
  - Add `natural_bounds: Rect<T>`, `canvas_width: u32`, `canvas_height: u32` fields
  - Update `Viewport::new()` constructor signature
- Update all Viewport creation sites:
  - `fractalwonder-ui/src/state/app_state.rs`
  - `fractalwonder-ui/src/app.rs`
  - `fractalwonder-ui/src/components/interactive_canvas.rs`
  - Test files

**Verification:**
```bash
cargo test --workspace
cargo clippy --workspace
```

**Browser test:** Works exactly the same (just carrying extra data)

---

### Task 3: Fix BigFloat to Use Precision Properly

**Goal:** BigFloat operations respect stored precision

**Changes:**
- Update `fractalwonder-core/src/numeric.rs`
  - Modify BigFloat arithmetic operations (`add`, `sub`, `mul`, `div`) to create and use `FBig::Context` with stored precision
  - Currently precision is stored but operations don't use it

**Verification:**
```bash
cargo test --workspace -- BigFloat
```

**Browser test:** Works the same (slightly more accurate internally)

---

### Task 4: Calculate and Use Dynamic Precision in Rendering

**Goal:** 🎯 **ENABLES EXTREME ZOOM** - precision calculated based on zoom level

**Changes:**
- Add to `fractalwonder-core/src/precision.rs`:
  - `calculate_viewport_precision_bits(viewport: &Viewport<f64>) -> usize`
- Update `fractalwonder-compute/src/messages.rs`:
  - Add `precision_bits: usize` field to `MainToWorker::RenderTile`
- Update `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`:
  - Calculate precision from viewport using new function
  - Use calculated precision when converting `f64` → `BigFloat` (replace hardcoded 256)
  - Pass `precision_bits` in worker message
- Update `fractalwonder-compute/src/worker.rs`:
  - Extract `precision_bits` from message
  - Use it when working with BigFloat values

**Verification:**
```bash
cargo test --workspace
wasm-pack test --headless --chrome
```

**Browser test:** 🚀 **Can now zoom beyond 1e15!** Try zooming to 1e20 and verify fractal details remain crisp

---

### Task 5: Change Viewport to BigFloat Throughout UI

**Goal:** Cleaner architecture - eliminate conversion boundary

**Changes:**
- Update `fractalwonder-ui/src/state/app_state.rs`:
  - Change `viewport: RwSignal<Viewport<f64>>` → `viewport: RwSignal<Viewport<BigFloat>>`
- Update `fractalwonder-ui/src/app.rs`:
  - Update all viewport signal types
  - Update `CanvasRenderer` type signature
- Update `fractalwonder-ui/src/components/interactive_canvas.rs`:
  - Change component to accept `Viewport<BigFloat>`
  - Simplify interaction callback (no canvas dimensions needed)
- Update `fractalwonder-core/src/transforms.rs`:
  - Update `apply_pixel_transform_to_viewport` to work with `Viewport<BigFloat>`
  - Calculate precision for new zoom level inside transform
- Update `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`:
  - Remove `f64` → `BigFloat` conversion (lines 156-162)
  - Viewport is already `BigFloat` from app state

**Verification:**
```bash
cargo test --workspace
cargo clippy --workspace
wasm-pack test --headless --chrome
```

**Browser test:** Works exactly the same as Task 4, but cleaner code architecture

## Open Questions

1. **BigFloat serialization**: Currently deserializes with hardcoded 256 bits. Need to serialize precision metadata?

2. **Initial viewport creation**: What precision for zoom=1.0? Could start with f64, upgrade to BigFloat at zoom threshold.

3. **Performance**: Creating ViewportContext on every interaction - should it be cached?

## Success Criteria

- [ ] Can zoom beyond 1e15 without coordinate precision loss
- [ ] Precision automatically adjusts based on zoom level
- [ ] No hardcoded precision values remain
- [ ] Clean layered architecture maintained (no circular dependencies)
- [ ] Tests pass for all zoom levels
