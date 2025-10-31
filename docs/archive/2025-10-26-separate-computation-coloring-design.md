# Design: Separating Computation from Coloring

**Date:** 2025-10-26
**Status:** Approved for Implementation

## Overview

This design separates fractal computation from color generation in the rendering architecture, enabling instant recoloring without expensive recomputation. The architecture maintains clean abstractions while supporting progressive rendering, parallel processing, and data caching.

## Problem Summary

Current architecture tightly couples computation with coloring:
- `ImagePointComputer` returns RGBA directly
- Changing color schemes requires full recomputation (potentially minutes at deep zoom)
- No way to cache computed data separately from visualization
- Cannot export raw computation data

## Solution Architecture

### 1. Core Trait Changes

**Before:**
```rust
trait ImagePointComputer {
    type Coord;
    fn compute(&self, coord: Point<Self::Coord>) -> (u8, u8, u8, u8);
}

trait Renderer {
    type Coord;
    fn render(&self, viewport, pixel_rect, canvas_size) -> Vec<u8>;  // RGBA bytes
}
```

**After:**
```rust
trait ImagePointComputer {
    type Coord;
    type Data: Clone;  // Generic data type
    fn compute(&self, coord: Point<Self::Coord>) -> Self::Data;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
}

trait Renderer {
    type Coord;
    type Data: Clone;  // Generic data type
    fn render(&self, viewport, pixel_rect, canvas_size) -> Vec<Self::Data>;  // Data, not RGBA
    fn natural_bounds(&self) -> Rect<Self::Coord>;
}

// Colorizer: simple function type
type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);
```

**Key change:** Entire rendering pipeline (traits, implementations) works with generic `Data`. Colorization happens outside this abstraction.

### 2. CanvasRenderCoordinator

Central orchestrator that handles:
- Tiling (split canvas into tiles)
- Progressive rendering (display tiles as they complete)
- Data caching (store computed Data for recoloring)
- Recoloring (apply new colorizer without recomputation)
- Smart re-rendering (detect when to compute vs recolorize)

```rust
struct CanvasRenderCoordinator<T, C>
where C: ImagePointComputer<Coord = T>
{
    renderer: PixelRenderer<C>,
    colorizer: Colorizer<C::Data>,
    tile_size: u32,

    // Cached state
    cached_viewport: Option<Viewport<T>>,
    cached_canvas_size: Option<(u32, u32)>,
    cached_data: Vec<C::Data>,  // Full width × height grid
}

impl<T, C> CanvasRenderCoordinator<T, C> {
    // Main entry point - decides what to do
    pub fn render(&mut self, viewport: &Viewport<T>, canvas_size: (u32, u32), image_data: &mut ImageData) {
        if viewport_or_size_changed() {
            self.render_with_computation(viewport, canvas_size, image_data);
        } else {
            // Same viewport/size, already cached and displayed
        }
    }

    // Update colorizer and recolorize if data cached
    pub fn set_colorizer(&mut self, colorizer: Colorizer<C::Data>, image_data: &mut ImageData) {
        self.colorizer = colorizer;
        if self.has_cached_data() {
            self.recolorize_full(image_data);
        }
    }

    // Update computer (invalidates cache)
    pub fn set_computer(&mut self, renderer: PixelRenderer<C>) {
        self.renderer = renderer;
        self.cached_viewport = None;
        self.cached_data.clear();
    }
}
```

**Progressive rendering implementation:**
```rust
fn render_with_computation(&mut self, viewport, canvas_size, image_data) {
    // Prepare storage
    self.cached_data.resize(width * height);

    // Compute tiles progressively
    for tile_rect in compute_tiles(canvas_size) {
        // TODO: Parallelize this loop in future

        // Compute tile data
        let tile_data = self.renderer.render(viewport, tile_rect, canvas_size);

        // Store in cache
        store_tile(tile_rect, &tile_data);

        // Colorize tile
        let rgba = colorize_tile(&tile_data, self.colorizer);

        // Display immediately (progressive!)
        put_tile_to_image_data(image_data, tile_rect, &rgba);
    }

    self.cached_viewport = Some(viewport.clone());
}
```

**Recoloring implementation:**
```rust
fn recolorize_full(&self, image_data) {
    // Read cached data, colorize, display
    let rgba = colorize_tile(&self.cached_data, self.colorizer);
    put_tile_to_image_data(image_data, full_rect, &rgba);
}
```

### 3. Component Architecture

**Ownership hierarchy:**
```
App (owns coordinator, computer, colorizer, viewport)
└── InteractiveCanvas (generic, handles UI only)
    ├── Canvas element (owns ImageData)
    └── Interaction handlers (zoom, pan)
```

**InteractiveCanvas: Generic and dumb**
```rust
#[component]
pub fn InteractiveCanvas<T>(
    viewport: ReadSignal<Viewport<T>>,
    set_viewport: WriteSignal<Viewport<T>>,
    natural_bounds: Rect<T>,
    render_trigger: ReadSignal<u32>,  // Increment to force re-render
    on_render: impl Fn(&Viewport<T>, (u32, u32), &mut ImageData) + 'static,
) -> impl IntoView {
    // Effect: Render when viewport OR render_trigger changes
    create_effect(move |_| {
        viewport.track();
        render_trigger.track();

        let mut image_data = get_image_data(&canvas);
        on_render(&viewport.get_untracked(), canvas_size, &mut image_data);
        put_image_data(&canvas, &image_data);
    });

    // Interaction handlers (zoom, pan) modify viewport signal
    let on_wheel = move |e: WheelEvent| {
        let new_viewport = zoom_viewport_at_point(/* ... */);
        set_viewport.set(new_viewport);
    };

    view! { <canvas on:wheel=on_wheel /> }
}
```

**App: Owns all rendering state**
```rust
#[component]
fn App() -> impl IntoView {
    let (computer, set_computer) = create_signal(MandelbrotComputer::new());
    let (colorizer, set_colorizer) = create_signal(classic_colorizer);
    let (viewport, set_viewport) = create_signal(initial_viewport());
    let (render_trigger, set_render_trigger) = create_signal(0u32);

    let coordinator = create_rw_signal(
        CanvasRenderCoordinator::new(
            PixelRenderer::new(computer.get_untracked()),
            colorizer.get_untracked(),
        )
    );

    // Effect: Computer changed → update coordinator, trigger render
    create_effect(move |_| {
        coordinator.update(|c| c.set_computer(PixelRenderer::new(computer.get())));
        set_render_trigger.update(|n| *n += 1);
    });

    // Effect: Colorizer changed → update coordinator, trigger render
    create_effect(move |_| {
        coordinator.update(|c| c.colorizer = colorizer.get());
        set_render_trigger.update(|n| *n += 1);
    });

    // Render callback
    let on_render = move |vp, size, image_data| {
        coordinator.update(|c| c.render(vp, size, image_data));
    };

    view! {
        <InteractiveCanvas
            viewport=viewport
            set_viewport=set_viewport
            natural_bounds=computer.get_untracked().natural_bounds()
            render_trigger=render_trigger
            on_render=on_render
        />
    }
}
```

**Key design decisions:**
- **InteractiveCanvas is fully generic** - no knowledge of computers, colorizers, coordinators
- **App owns coordinator** - centralized rendering state
- **render_trigger signal** - explicit control over when canvas re-renders (needed for colorizer changes)
- **on_render callback** - App defines rendering behavior, canvas just calls it

## Example Implementations

### TestImageRenderer (adapted)

```rust
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

impl ImagePointComputer for TestImageRenderer {
    type Coord = f64;
    type Data = TestImageData;

    fn compute(&self, coord: Point<f64>) -> Self::Data {
        // Compute pattern data (not colors)
        TestImageData { /* ... */ }
    }
}

fn test_image_colorizer(data: &TestImageData) -> (u8, u8, u8, u8) {
    // Convert data to RGBA
}
```

### MandelbrotComputer (future)

```rust
#[derive(Clone, Copy, Debug)]
pub struct MandelbrotData {
    pub escape_time: u32,
    pub z_max: f64,
    pub distance_estimate: f64,
    pub escaped: bool,
}

impl ImagePointComputer for MandelbrotComputer {
    type Coord = f64;  // Or rug::Float for arbitrary precision
    type Data = MandelbrotData;

    fn compute(&self, coord: Point<f64>) -> Self::Data {
        // Mandelbrot iteration, return rich data
    }
}

// Multiple colorizers for same data!
fn classic_colorizer(data: &MandelbrotData) -> (u8, u8, u8, u8) { /* ... */ }
fn distance_estimate_colorizer(data: &MandelbrotData) -> (u8, u8, u8, u8) { /* ... */ }
fn z_max_colorizer(data: &MandelbrotData) -> (u8, u8, u8, u8) { /* ... */ }
```

## Requirements Satisfaction

✅ **Separate computation from coloring** - `Data` vs `Colorizer`
✅ **Progressive rendering** - Tiles displayed as they complete
✅ **Data storage** - `cached_data` stores full grid
✅ **Instant recoloring** - `recolorize_full()` reads cache
✅ **Parallel processing ready** - TODO in tile loop (future work)
✅ **Clean abstractions** - Generic traits, composable renderers
✅ **Display-agnostic** - Coordinator works with any ImageData source

## Future Optimizations

1. **Parallelism** - Parallelize tile computation loop (web workers, rayon-wasm)
2. **Smart re-rendering** - Only recompute changed tiles on pan
3. **RGBA caching** - Store colorized data to avoid recolorization
4. **IndexedDB persistence** - Persist computed data across page refreshes
5. **Distributed rendering** - Send tiles to server/workers for computation

## Migration Path

1. Add `type Data` to `ImagePointComputer` and `Renderer` traits
2. Update `PixelRenderer` to return `Vec<Data>` instead of `Vec<u8>`
3. Update `TestImageRenderer` to return `TestImageData` instead of RGBA
4. Create `CanvasRenderCoordinator` implementation
5. Update `InteractiveCanvas` to use coordinator via callback
6. Remove `TiledRenderer` (functionality absorbed by coordinator)
7. Test with existing test image
8. Implement `MandelbrotComputer` with new architecture

## Open Questions

1. **Default value for Data:** How to initialize `cached_data.resize()`? Require `Data: Default`?
2. **Colorizer identity:** Currently just overwrites - should we track colorizer changes more explicitly?
3. **Export use cases:** How to handle PNG export without canvas? Separate utility functions?

## Success Metrics

- [ ] TestImageRenderer compiles and renders with new architecture
- [ ] Colorizer changes recolorize instantly (< 50ms)
- [ ] Progressive rendering displays tiles as they complete
- [ ] Viewport changes trigger recomputation
- [ ] Cache invalidation works correctly
- [ ] No regression in render performance
