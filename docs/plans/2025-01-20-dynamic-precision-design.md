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

## Required Changes

### Phase 1: Move precision logic to CORE

1. **Create** `fractalwonder-core/src/precision.rs`
   - Move `calculate_max_iterations` from `mandelbrot.rs`
   - Create `required_precision_bits` function

2. **Update** `fractalwonder-core/src/lib.rs`
   - Export precision functions

3. **Update** `fractalwonder-compute/src/computers/mandelbrot.rs`
   - Import `calculate_max_iterations` from core
   - Remove local implementation

4. **Update** `fractalwonder-compute/src/precision.rs`
   - Import `calculate_max_iterations` from core
   - Update `PrecisionCalculator` to use it

### Phase 2: Extend Viewport with context fields

5. **Update** `fractalwonder-core/src/viewport.rs`
   - Add `natural_bounds`, `canvas_width`, `canvas_height` fields
   - Update constructor

6. **Update all Viewport creation sites** throughout codebase
   - Pass natural_bounds and canvas_size to constructor

### Phase 3: Change Viewport<f64> → Viewport<BigFloat>

7. **Update** `fractalwonder-ui/src/state/app_state.rs`
   - Change `viewport: Viewport<f64>` to `viewport: Viewport<BigFloat>`

8. **Update** `fractalwonder-ui/src/app.rs`
   - Change all viewport signals and functions to use `Viewport<BigFloat>`
   - Update `CanvasRenderer` type signature

9. **Update** `fractalwonder-ui/src/components/interactive_canvas.rs`
   - Change component signature to accept `Viewport<BigFloat>`
   - Simplify interaction callback (viewport already has everything)
   - Update render effect

10. **Update** `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`
    - Remove hardcoded f64 → BigFloat conversion (lines 156-162)
    - Viewport is already BigFloat from app state

### Phase 4: Fix BigFloat to use precision properly

11. **Update** `fractalwonder-core/src/numeric.rs`
    - Fix BigFloat arithmetic to actually use FBig Context with stored precision
    - Currently precision is stored but not used in operations

12. **Update worker protocol** `fractalwonder-compute/src/messages.rs`
    - Add `precision_bits: usize` to `MainToWorker::RenderTile`

13. **Update** `fractalwonder-compute/src/worker.rs`
    - Use precision from message for rendering

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
