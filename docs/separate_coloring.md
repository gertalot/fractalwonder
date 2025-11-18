# Rendering Architecture Requirements: Separating Computation from Coloring

## Problem Statement

The current rendering architecture tightly couples fractal computation with color generation. `ImagePointComputer` returns RGBA values directly:

```rust
trait ImagePointComputer {
    type Coord;
    fn compute(&self, coord: Point<Self::Coord>) -> (u8, u8, u8, u8);
}
```

**Why this is limiting:**
- Computing Mandelbrot data (escape time, z_max, distance estimate, etc.) is **expensive** (potentially minutes at deep zoom)
- Choosing colors from that data is **cheap** (microseconds)
- Trying a different color scheme requires **full recomputation** of the entire fractal
- No way to cache computed data separately from visualization

## Key Requirements

Any new architecture must satisfy these essential requirements:

### 1. Separate Computation from Coloring

**Requirement:** Fractal/image data computation must be completely decoupled from color assignment.

**Benefits:**
- Swap color schemes instantly without recomputation
- Store computed data for multiple visualizations
- Apply different colorizers to the same dataset
- Export raw computation data separately from RGBA images

**Example:**
```rust
// Compute once (expensive)
let data = compute_mandelbrot(viewport);

// Recolor many times (cheap)
let image1 = colorize(data, ClassicColorizer);
let image2 = colorize(data, DistanceEstimateColorizer);
let image3 = colorize(data, HistogramColorizer);
```

### 2. Progressive Rendering Visible to User

**Requirement:** User must see tiles appear progressively as they are computed, not wait for entire image to complete.

**Why this matters:**
- At extreme zoom levels, full-canvas render can take minutes
- Instant visual feedback shows computation is happening
- User can see interesting regions and cancel/navigate before completion
- Essential for good UX with expensive computations

**Current behavior:**
- `TiledRenderer` computes tiles progressively
- all tiles need to complete before TiledRenderer returns
- User still only sees image appear at once after computation is complete

**Must change:** into fully progressive display behavior.

### 3. Parallel Processing

**Requirement:** Tiles must be computable in parallel (web workers, thread pool, etc.)

**Why this matters:**
- Mandelbrot computation is embarrassingly parallel
- Each tile is independent - perfect for concurrent computation
- Multi-core systems should see near-linear speedup
- Essential for performance at high resolutions

**Implementation consideration:**
- Tiles may complete out of order
- Need async/streaming API to handle completion events
- Cache must be thread-safe (or each tile caches independently)

### 4. Data Storage for Recoloring

**Requirement:** Store the complete computed dataset (raw Data, not RGBA) for the entire image rect, enabling instant recoloring.

**What gets stored:**
- Full width × height grid of computed Data values
- One Data value per pixel in the rendered rect
- Raw computation results (e.g., MandelbrotData for each pixel)
- NOT a cache with invalidation - this is the **source of truth** for the current viewport

**Storage behavior:**
```rust
// First render: compute all pixels → store Data grid → colorize → display
render(viewport, colorizer_a)
// Produces: Vec<Data> with width×height elements
// Stores: Complete data grid
// Displays: RGBA from colorizing the data

// Recolor: retrieve stored Data grid → colorize with new colorizer → display
render(viewport, colorizer_b)
// Reads: Existing Vec<Data>
// Computes: Nothing! Just recolorizes
// Displays: New RGBA from same data

// Pan/zoom: discard old data, compute new data grid
render(new_viewport, colorizer_b)
// Old data no longer relevant - compute fresh Data grid for new viewport
```

**Future extension:** Persist this data grid to IndexedDB so expensive computations survive page refreshes.

### 5. Abstract Renderer Trait

**Requirement:** Maintain clean, composable renderer abstraction that works with arbitrary data types.

**Principles to preserve:**
- Generic over coordinate type (`Coord`: f64, rug::Float, etc.)
- Generic over data type (`Data`: arbitrary per implementation)
- Composable wrappers (TiledRenderer, CachedRenderer, etc.)
- Type-safe separation of pixel space vs. image space
- Pure computation trait: `Point<T> → Data` (no side effects, no loops)

**Example composition:**
```rust
let computer = MandelbrotComputer::new();      // Point<T> → MandelbrotData
let pixel_renderer = PixelRenderer::new(computer);  // Adds pixel iteration
let tiled = TiledRenderer::new(pixel_renderer);     // Adds tiling
let colorized = Colorizer::new(tiled, colorizer);   // Adds coloring
```

### 6. Decoupled from Display Medium

**Requirement:** Rendering to RGBA must not be tied to browser canvas display.

**Why:**
- Same rendering should work for canvas display, image file export, server-side rendering
- Colorization happens before display medium is chosen
- Should be able to render to PNG file without a canvas

**Anti-pattern to avoid:**
```rust
// BAD: Colorizer lives in InteractiveCanvas
impl InteractiveCanvas {
    fn render(&self) {
        let data = renderer.compute();
        let rgba = self.colorizer.colorize(data);  // Tied to canvas!
        self.canvas.display(rgba);
    }
}
```

**Better:**
```rust
// GOOD: Colorizer produces RGBA independent of display
let rgba_renderer = ColorizedRenderer::new(data_renderer, colorizer);
let rgba_data = rgba_renderer.render();

// Use RGBA wherever needed
canvas.display(rgba_data);
// or
save_to_png(rgba_data);
// or
send_to_server(rgba_data);
```

## Current Architecture Baseline

From `RENDERER-ARCHITECTURE.md`:

```rust
trait ImagePointComputer {
    type Coord;
    fn compute(&self, coord: Point<Self::Coord>) -> (u8, u8, u8, u8);
}

trait Renderer {
    type Coord;
    fn render(&self, viewport, pixel_rect, canvas_size) -> Vec<u8>;
}

struct PixelRenderer<C: ImagePointComputer> {
    computer: C,
}

struct TiledRenderer<R: Renderer> {
    inner: R,
    tile_size: u32,
}
```

**Current flow:**
```
TiledRenderer
  └─ Splits canvas into tiles
     └─ PixelRenderer (for each tile)
        └─ Loops pixels in tile
           └─ Transforms pixel → Point<T>
              └─ ImagePointComputer::compute(Point<T>) → (r,g,b,a)
```

**Progressive rendering:** `TiledRenderer` yields each completed tile to canvas immediately.

## Constraints and Design Principles

The solution must preserve these architectural qualities:

1. **Generic-first design** - Support arbitrary precision types (f64, rug::Float)
2. **Type safety** - Compiler prevents mixing pixel/image coordinates
3. **Composability** - Renderers wrap renderers (like Rust iterators)
4. **Pure computation core** - `ImagePointComputer` has no side effects, just math
5. **Separation of concerns** - Transform logic separate from computation logic
6. **Zero-cost abstractions** - Generic composition optimizes to direct calls

## Success Criteria

A successful solution will:

1. ✅ Allow `MandelbrotComputer` to return `MandelbrotData { escape_time, z_max, distance_estimate }` instead of RGBA
2. ✅ Support multiple `Colorizer` implementations operating on the same data
3. ✅ Store computed data per tile
4. ✅ Recolor stored data without calling compute functions
5. ✅ Display tiles progressively as they complete (not all at once at end)
6. ✅ Enable parallel tile computation (future: web workers)
7. ✅ Work with `InteractiveCanvas` with minimal changes
8. ✅ Support non-canvas use cases (PNG export, server-side rendering)
9. ✅ Maintain clean trait hierarchy and composability
10. ✅ Compile without breaking existing test image renderer

## Open Design Questions

These questions need answers during design phase:

1. **API shape for progressive rendering**
   - **Decision: Async Stream** - `render_tiles() -> impl Stream<Item = (PixelRect, Vec<u8>)>`
   - Rationale: Parallel computation means tiles complete out-of-order, ruling out Iterator. Stream is Rust-idiomatic for async sequences and Leptos is already async-native.

2. **Raw Data Storage ownership**
   - Does raw data live in TiledRenderer? In a separate Renderer wrapper? In the colorizer layer?
   - How does storage get accessed for recoloring?

3. **Trait hierarchy**
   - Single `Renderer` trait with generic `Output` type?
   - Separate `DataRenderer` and `RGBARenderer` traits?
   - Keep existing `Renderer`, add new `DataRenderer` alongside?

4. **Progressive rendering + parallel processing**
   - How do we handle out-of-order tile completion?
   - Should cache be thread-safe or per-worker?
   - Async runtime in WASM?

5. **Colorizer composition**
   - Is colorizer a trait or a function type?
   - How does `TiledRGBARenderer` combine tiling + caching + colorizing?
   - Can we keep tiling generic (works for both Data and RGBA)?

## Example Use Case: Mandelbrot Explorer

**Phase 1: Initial render**
```rust
let computer = MandelbrotComputer { max_iterations: 1000 };
let renderer = /* some composition of computer + tiling + colorizer */;

// User sees tiles appear progressively
renderer.render(viewport, canvas_size);
// Internally: computes tiles in parallel, caches data, colorizes, displays
```

**Phase 2: Recolor (instant)**
```rust
// User clicks "Distance Estimate" colorizer button
renderer.set_colorizer(DistanceEstimateColorizer);
// Internally: uses cached data, recolorizes all tiles, displays
// No computation happens!
```

**Phase 3: Pan/Zoom (recompute)**
```rust
// User zooms in 2x
renderer.render(new_viewport, canvas_size);
// Cache miss → recomputes all tiles with new viewport
```

**Phase 4: Export (same renderer, different output)**
```rust
// User clicks "Export PNG"
let rgba_data = renderer.render_full(viewport, (3840, 2160));
save_png("mandelbrot.png", rgba_data);
// Uses same rendering pipeline, no canvas involved
```

## Next Steps

1. Design trait hierarchy that satisfies all requirements
2. Sketch API for progressive + parallel rendering
3. Determine cache ownership and access pattern
4. Validate design with concrete examples (Mandelbrot + TestImage)
5. Create implementation plan
6. Consider migration path from current architecture
