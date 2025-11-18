# Design: Unified Rendering Architecture with Runtime Polymorphism

**Date:** 2025-10-28
**Status:** Approved for Implementation

## Overview

This design establishes a clean component ownership hierarchy and separates computation from coloring, enabling instant recoloring without expensive recomputation. The architecture uses runtime polymorphism (trait objects) to allow swapping renderers dynamically while maintaining cached computation data.

## Problem Summary

Current architecture has three major issues:

1. **Backward ownership hierarchy:** `TestImageRenderer` component owns both canvas AND rendering logic, violating separation of concerns
2. **Tight coupling:** Computation coupled with coloring - changing colors requires full recomputation
3. **Inflexible abstractions:** Hard to swap renderer implementations at runtime

## Solution Architecture

### Core Principles

1. **Separation of Concerns:** UI ↔ Rendering ↔ Computation ↔ Visualization are independent
2. **Runtime Polymorphism:** Use trait objects (`Box<dyn Trait>`) for swappable renderers
3. **Cache Preservation:** Shared ownership via `Arc` preserves expensive computation when only colorizer changes
4. **Clean Ownership:** App owns domain logic, InteractiveCanvas owns presentation, UI owns its own presentation logic

### Architectural Patterns

- **Strategy Pattern:** Colorizer is swappable visualization strategy
- **Dependency Injection:** Components receive dependencies rather than creating them
- **Observer Pattern:** Reactive signals propagate changes automatically
- **Memoization:** Cache expensive computation keyed by viewport/size
- **Unidirectional Data Flow:** State flows down, events flow up

---

## Architecture Layers

### Layer 1: Trait Hierarchy

Three clean abstraction layers:

```rust
// Layer 1: Image-space computation (generic coordinates)
trait ImagePointComputer {
    type Coord;          // f64, rug::Float, or other numeric type
    type Data: Clone;    // Computation output (NOT colors)

    fn compute(&self, coord: Point<Self::Coord>) -> Self::Data;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
}

// Layer 2: Pixel-space rendering (converts viewport → pixel data)
trait Renderer {
    type Coord;
    type Data: Clone;

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: Rect<u32>,
        canvas_size: (u32, u32)
    ) -> Vec<Self::Data>;

    fn natural_bounds(&self) -> Rect<Self::Coord>;
}

// Layer 3: Canvas rendering (handles tiling, caching, progressive display)
trait CanvasRenderer {
    type Coord;
    type Data: Clone;

    fn render(&self, viewport: &Viewport<Self::Coord>, image_data: &mut ImageData);
    fn with_colorizer(&self, colorizer: Colorizer<Self::Data>) -> Self;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
}

// Colorizer: simple function type
type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);
```

**Key change from old architecture:** Traits work with generic `Data` type, RGBA is only the final output.

**Critical insight:** RGBA generation happens **outside** the computation pipeline. Computation returns rich data, colorizer converts to colors.

---

### Layer 2: Implementations

**PixelRenderer:** Iterates pixels, converts to image coordinates, calls computer

```rust
struct PixelRenderer<C: ImagePointComputer> {
    computer: C,
}

impl<C: ImagePointComputer> Renderer for PixelRenderer<C> {
    type Coord = C::Coord;
    type Data = AppData;  // Wrapped in unified enum

    fn render(&self, viewport, pixel_rect, canvas_size) -> Vec<AppData> {
        // Iterate pixels in pixel_rect
        // Convert pixel coords to image coords via viewport
        // Call computer.compute(image_coord)
        // Wrap result in AppData enum
    }
}
```

**TilingCanvasRenderer:** Orchestrates tiling, progressive rendering, caching

```rust
struct TilingCanvasRenderer<R: Renderer> {
    renderer: R,
    colorizer: Colorizer<R::Data>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<R>>>,  // Shared ownership!
}

struct CachedState<R: Renderer> {
    viewport: Option<Viewport<R::Coord>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<R::Data>,  // Expensive computed data
}

impl<R: Renderer> TilingCanvasRenderer<R> {
    pub fn render(&self, viewport: &Viewport<R::Coord>, image_data: &mut ImageData) {
        let mut cache = self.cached_state.lock().unwrap();

        // Decision: compute vs recolorize
        if cache.viewport.as_ref() == Some(viewport) &&
           cache.canvas_size == Some(image_data.size()) {
            // Same viewport/size → recolorize from cache (FAST!)
            self.recolorize_from_cache(&cache, image_data);
        } else {
            // Viewport/size changed → recompute
            self.render_with_computation(viewport, image_data, &mut cache);
        }
    }

    pub fn with_colorizer(&self, colorizer: Colorizer<R::Data>) -> Self {
        Self {
            renderer: self.renderer.clone(),
            colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state),  // SHARED cache!
        }
    }

    fn render_with_computation(
        &self,
        viewport: &Viewport<R::Coord>,
        image_data: &mut ImageData,
        cache: &mut CachedState<R>
    ) {
        cache.data.clear();
        cache.data.reserve(width * height);

        // Progressive tiled rendering
        for tile_rect in compute_tiles(width, height, self.tile_size) {
            let tile_data = self.renderer.render(viewport, tile_rect, (width, height));
            cache.data.extend(tile_data.iter().cloned());
            self.colorize_and_display_tile(&tile_data, tile_rect, image_data);
        }

        cache.viewport = Some(viewport.clone());
        cache.canvas_size = Some((width, height));
    }

    fn recolorize_from_cache(&self, cache: &CachedState<R>, image_data: &mut ImageData) {
        let (width, height) = image_data.size();
        self.colorize_and_display_tile(
            &cache.data,
            Rect { x: 0, y: 0, width, height },
            image_data
        );
    }

    fn colorize_and_display_tile(
        &self,
        data: &[R::Data],
        rect: Rect<u32>,
        image_data: &mut ImageData
    ) {
        let rgba_bytes: Vec<u8> = data
            .iter()
            .flat_map(|d| {
                let (r, g, b, a) = (self.colorizer)(d);
                [r, g, b, a]
            })
            .collect();

        image_data.put_tile_data(rect, &rgba_bytes);
    }
}
```

---

### Layer 3: Type System

**Unified data type using enum:**

```rust
// All renderer data types unified
#[derive(Clone, Debug)]
pub enum AppData {
    TestImage(TestImageData),
    Mandelbrot(MandelbrotData),
    // Future: TileMap, YouTubeFrame, etc.
}

#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

// Colorizers pattern match
fn test_image_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImage(d) => {
            // Colorize based on checkerboard, circle_distance
        }
        _ => (0, 0, 0, 255),  // Black for wrong type
    }
}
```

**Why enum instead of generics everywhere:**
- Allows runtime polymorphism via trait objects
- Single unified type for all App-level signals
- Pattern matching in colorizers is type-safe
- Easy to add new renderer types

---

### Layer 4: Component Architecture

**App: Owns domain/rendering state**

```rust
type AppRenderer = Box<dyn Renderer<Coord=f64, Data=AppData>>;

#[component]
fn App() -> impl IntoView {
    // ========== Domain state ==========
    let (renderer, set_renderer) = create_signal(
        Box::new(PixelRenderer::new(TestImageComputer::new())) as AppRenderer
    );

    let (colorizer, set_colorizer) = create_signal(
        test_image_colorizer as Colorizer<AppData>
    );

    let (viewport, set_viewport) = create_signal(
        Viewport::new(renderer.get_untracked().natural_bounds())
    );

    // ========== Canvas renderer with cache preservation ==========
    let canvas_renderer = create_rw_signal(
        TilingCanvasRenderer::new(
            renderer.get_untracked(),
            colorizer.get_untracked(),
            128
        )
    );

    // Effect: Colorizer changed → preserve cache
    create_effect(move |_| {
        let new_colorizer = colorizer.get();
        canvas_renderer.update(|cr| {
            *cr = cr.with_colorizer(new_colorizer);  // Arc::clone preserves cache!
        });
    });

    // Effect: Renderer changed → rebuild cache
    create_effect(move |_| {
        let new_renderer = renderer.get();
        canvas_renderer.update(|cr| {
            *cr = TilingCanvasRenderer::new(new_renderer, cr.colorizer, cr.tile_size);
        });
        set_viewport.set(Viewport::new(
            renderer.get_untracked().natural_bounds()
        ));
    });

    // ========== RendererInfo for UI display ==========
    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

    let renderer_info = create_memo(move |_| {
        let vp = viewport.get();
        RendererInfoData {
            center_display: format!("{:.6}, {:.6}", vp.center.x, vp.center.y),
            zoom_display: format!("{:.2e}", vp.zoom),
            render_time_ms: render_time_ms.get(),
            name: "Test Image".to_string(),
            custom_params: vec![],
        }
    });

    view! {
        <div class="app-container">
            <InteractiveCanvas
                canvas_renderer=canvas_renderer
                viewport=viewport
                set_viewport=set_viewport
                set_render_time_ms=set_render_time_ms
            />
            <UI
                info=renderer_info
                viewport=viewport
                set_viewport=set_viewport
                natural_bounds=renderer.get_untracked().natural_bounds()
                set_renderer=set_renderer
                set_colorizer=set_colorizer
            />
        </div>
    }
}
```

**InteractiveCanvas: Owns canvas element and interaction**

```rust
#[component]
fn InteractiveCanvas(
    canvas_renderer: RwSignal<TilingCanvasRenderer<AppRenderer>>,
    viewport: ReadSignal<Viewport<f64>>,
    set_viewport: WriteSignal<Viewport<f64>>,
    set_render_time_ms: WriteSignal<Option<f64>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<Canvas>();

    // Effect: Render when canvas_renderer OR viewport changes
    create_effect(move |_| {
        let vp = viewport.get();
        canvas_renderer.track();

        if let Some(canvas) = canvas_ref.get() {
            let start = window().performance().unwrap().now();

            let mut image_data = get_image_data(&canvas);
            canvas_renderer.with(|cr| cr.render(&vp, &mut image_data));
            put_image_data(&canvas, &image_data);

            let elapsed = window().performance().unwrap().now() - start;
            set_render_time_ms.set(Some(elapsed));
        }
    });

    // use_canvas_interaction hook for zoom/pan
    let interaction = use_canvas_interaction(
        canvas_ref,
        viewport,
        set_viewport,
        canvas_renderer.get_untracked().natural_bounds()
    );

    view! {
        <canvas
            node_ref=canvas_ref
            on:wheel=interaction.on_wheel
            on:mousedown=interaction.on_mousedown
        />
    }
}
```

**UI: Owns presentation logic**

```rust
#[component]
pub fn UI(
    info: ReadSignal<RendererInfoData>,
    viewport: ReadSignal<Viewport<f64>>,
    set_viewport: WriteSignal<Viewport<f64>>,
    natural_bounds: Rect<f64>,
    set_renderer: WriteSignal<AppRenderer>,
    set_colorizer: WriteSignal<Colorizer<AppData>>,
) -> impl IntoView {
    // UI manages its own presentation state
    let (is_popover_open, set_is_popover_open) = create_signal(false);
    let (is_visible, set_is_hovering) = use_ui_visibility();

    // Keep UI visible when popover is open
    create_effect(move |_| {
        if is_popover_open.get() {
            set_is_hovering.set(true);
        }
    });

    // UI callbacks (presentation logic, not domain logic)
    let on_home_click = move || {
        set_viewport.set(Viewport::new(natural_bounds));
    };

    let (toggle_fullscreen, _) = use_fullscreen();
    let on_fullscreen_click = move || toggle_fullscreen();

    // ... UI rendering
}
```

---

## Data Flow Examples

### Scenario 1: User changes colorizer

```
1. UI dropdown → set_colorizer(blue_colorizer)
2. App effect runs: canvas_renderer.update(|cr| cr.with_colorizer(...))
3. with_colorizer() creates new TilingCanvasRenderer, Arc::clone shares cache
4. canvas_renderer signal changes
5. InteractiveCanvas effect triggers
6. renderer.render() called
7. Cache check: viewport unchanged → recolorize_from_cache()
8. Result: Instant recolor (< 50ms), no expensive computation
```

### Scenario 2: User pans viewport

```
1. use_canvas_interaction → set_viewport
2. viewport signal changes
3. InteractiveCanvas effect triggers
4. renderer.render() called
5. Cache check: viewport changed → render_with_computation()
6. Tiles computed progressively, cache updated
7. Result: Smooth progressive render
```

### Scenario 3: User selects new renderer

```
1. UI dropdown → set_renderer(Box::new(PixelRenderer::new(MandelbrotComputer::new())))
2. App effect runs: canvas_renderer.update(|cr| TilingCanvasRenderer::new(...))
3. New TilingCanvasRenderer created with fresh cache
4. viewport reset to natural_bounds
5. InteractiveCanvas effect triggers
6. Full recompute with new renderer
```

---

## Component Responsibilities

| Component | Responsibilities | Does NOT handle |
|-----------|-----------------|-----------------|
| **App** | Domain state (renderer, colorizer, viewport), cache management | UI visibility, fullscreen, presentation |
| **InteractiveCanvas** | Canvas element, zoom/pan interaction, render timing | What to render, how to cache, UI presentation |
| **UI** | Presentation (visibility, fullscreen, popover), renderer/colorizer selection | Canvas interaction, caching |
| **TilingCanvasRenderer** | Tiling, progressive rendering, cache optimization | UI, viewport management |
| **PixelRenderer** | Pixel iteration, coordinate conversion | Tiling, caching, coloring |
| **ImagePointComputer** | Pure computation | Coordinates, rendering, colors |

---

## Migration Path

### Components/Modules Being Removed

1. **TestImageRenderer component** - Violates SoC, owns canvas + rendering + UI
2. **TiledRenderer** (if exists) - Functionality absorbed by TilingCanvasRenderer

### Components Being Refactored

1. **ImagePointComputer trait** - Add `type Data`, remove RGBA return
2. **Renderer trait** - Add `type Data`, return `Vec<Data>` not `Vec<u8>`
3. **PixelRenderer** - Work with generic Data, wrap in AppData
4. **UI component** - Add `set_renderer`/`set_colorizer` props

### New Implementations

1. **AppData enum** - Unified data type
2. **TilingCanvasRenderer** - Tiling/caching orchestrator
3. **Colorizer functions** - `test_image_colorizer`, etc.
4. **InteractiveCanvas component** - Generic canvas with interaction

### Implementation Steps

1. Add `type Data` to ImagePointComputer/Renderer traits
2. Create AppData enum with TestImage variant
3. Update TestImageComputer to return TestImageData (not RGBA)
4. Create test_image_colorizer function
5. Implement TilingCanvasRenderer
6. Create InteractiveCanvas component
7. Refactor App to new architecture
8. Update UI component for renderer/colorizer selection
9. Remove TestImageRenderer component
10. Test with existing test image

---

## Success Criteria

- [ ] TestImageRenderer component removed
- [ ] Test image renders correctly via new architecture
- [ ] Colorizer changes recolorize instantly (< 50ms)
- [ ] Viewport changes trigger recomputation with progressive rendering
- [ ] Cache preserved when only colorizer changes
- [ ] Cache invalidated when renderer or viewport changes
- [ ] UI shows correct info and allows renderer/colorizer selection
- [ ] No regression in render performance
- [ ] Clean separation: App (domain) / InteractiveCanvas (interaction) / UI (presentation)

---

## Future Optimizations

1. **Parallelism:** Parallelize tile computation (web workers, rayon-wasm)
2. **Smart re-rendering:** Only recompute changed tiles on pan
3. **RGBA caching:** Store colorized data to avoid recolorization
4. **Multiple colorizers:** Allow multiple colorizer selection in UI
5. **Renderer plugins:** Dynamic renderer loading

---

## Design Comparison with 2025-10-26 Design

**Key differences from previous design:**

| Aspect | 2025-10-26 Design | This Design (2025-10-28) |
|--------|-------------------|--------------------------|
| Polymorphism | Generic-based (compile time) | Trait objects (runtime) |
| Renderer swapping | Complex, type changes | Simple, just swap Box |
| App component | Owns coordinator in signal, uses callback | Owns renderer/colorizer separately, effects manage coordination |
| Cache preservation | with_colorizer method + memo | with_colorizer method + effects |
| InteractiveCanvas | Receives callback | Receives RwSignal of renderer |
| UI ownership | Mixed (some in App) | Fully owns presentation logic |

**Why this design is better:**
- Runtime polymorphism matches OOP mental model
- Cleaner separation of concerns (App vs UI vs Canvas)
- Effects make cache preservation explicit
- UI component is fully self-contained
