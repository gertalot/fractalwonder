# Async Progressive Renderer Design

## Overview

Iteration 8 introduces tiled progressive rendering that keeps the UI responsive during long renders. Instead of blocking the main thread until all pixels are computed, we render tile-by-tile and yield to the browser between tiles.

## Problem

The current `InteractiveCanvas` computes all pixels synchronously in a `create_effect`, blocking the main thread. At deep zoom levels, a single render can take many minutes. During this time:
- UI is frozen (no pan/zoom preview)
- No progress feedback
- Cannot cancel mid-render

## Solution

**AsyncProgressiveRenderer** - an async tile-by-tile renderer that:
1. Divides canvas into tiles (center-out ordering)
2. Computes one tile at a time
3. Draws each tile immediately to canvas
4. Yields to browser via `requestAnimationFrame` between tiles
5. Supports cancellation via signal check before each tile

This is a stepping stone to Iteration 9 (workers). The same tiling/progress infrastructure will be reused, but computation moves from main thread async to parallel workers.

## Architecture

### File Structure

```
fractalwonder-ui/src/rendering/
├── mod.rs
├── async_progressive_renderer.rs   # Main renderer implementation
├── canvas_renderer.rs              # CanvasRenderer trait
├── render_progress.rs              # RenderProgress struct
├── tiles.rs                        # Tile generation utilities
├── canvas_utils.rs                 # Canvas drawing helpers
└── colorizers/                     # Existing colorizers
```

### CanvasRenderer Trait

```rust
pub trait CanvasRenderer {
    /// Start rendering viewport to canvas (async, returns immediately)
    fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement);

    /// Cancel in-progress render
    fn cancel(&self);

    /// Progress signal for UI binding
    fn progress(&self) -> RwSignal<RenderProgress>;

    /// Change colorizer (re-colorizes from cache if available)
    fn set_colorizer(&self, colorizer: Colorizer);
}
```

### RenderProgress

```rust
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 { 0.0 }
        else { (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0 }
    }
}
```

### AsyncProgressiveRenderer

```rust
pub struct AsyncProgressiveRenderer {
    config: &'static FractalConfig,
    colorizer: Rc<RefCell<Colorizer>>,
    progress: RwSignal<RenderProgress>,
    render_id: Rc<Cell<u32>>,
    cache: Rc<RefCell<TileCache>>,
}
```

### Core Render Flow

```rust
fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
    let render_id = self.render_id.get() + 1;
    self.render_id.set(render_id);

    let tiles = generate_tiles(canvas.width(), canvas.height(), tile_size);
    self.progress.set(RenderProgress::new(tiles.len() as u32));

    // Clone refs for async block
    let cancel_id = self.render_id.clone();
    let progress = self.progress;
    let config = self.config;
    let colorizer = self.colorizer.clone();
    let cache = self.cache.clone();
    let ctx = get_2d_context(canvas);
    let vp = viewport.clone();
    let canvas_size = (canvas.width(), canvas.height());

    spawn_local(async move {
        let start_time = performance_now();

        for (i, tile) in tiles.iter().enumerate() {
            // Check cancellation
            if cancel_id.get() != render_id {
                break;
            }

            // Compute tile
            let region = pixel_rect_to_viewport(&tile, &vp, canvas_size);
            let data = match config.id {
                "mandelbrot" => compute_mandelbrot(&region, tile.size()),
                "test_image" => compute_test_image(&region, tile.size()),
                _ => continue,
            };

            // Colorize and draw
            draw_tile_to_canvas(&ctx, &data, &tile, &colorizer.borrow());

            // Cache for re-colorization
            cache_tile(&mut cache.borrow_mut(), &data, &tile);

            // Update progress
            progress.update(|p| {
                p.completed_tiles = (i + 1) as u32;
                p.elapsed_ms = performance_now() - start_time;
            });

            // Yield to browser
            yield_to_browser().await;
        }

        progress.update(|p| p.is_complete = true);
    });
}
```

### yield_to_browser Implementation

```rust
async fn yield_to_browser() {
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();

    let closure = Closure::once(move || {
        let _ = sender.send(());
    });

    web_sys::window()
        .unwrap()
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap();

    closure.forget();
    let _ = receiver.await;
}
```

### Tile Generation

Reused from archive with center-out ordering:

```rust
pub fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y in (0..height).step_by(tile_size as usize) {
        for x in (0..width).step_by(tile_size as usize) {
            tiles.push(PixelRect::new(
                x, y,
                tile_size.min(width - x),
                tile_size.min(height - y),
            ));
        }
    }

    // Sort by distance from center
    let center = (width as f64 / 2.0, height as f64 / 2.0);
    tiles.sort_by(|a, b| {
        let a_dist = distance_to_center(a, center);
        let b_dist = distance_to_center(b, center);
        a_dist.partial_cmp(&b_dist).unwrap()
    });

    tiles
}

pub fn calculate_tile_size(zoom: f64) -> u32 {
    if zoom >= 1e10 { 64 } else { 128 }
}
```

### Cancellation

```rust
fn cancel(&self) {
    // Increment render_id - the async loop checks this before each tile
    self.render_id.set(self.render_id.get() + 1);
}
```

The async loop checks `cancel_id.get() != render_id` before each tile. If they don't match, a new render was started (or cancel was called), so we break out.

## Hook Changes

### use_canvas_interaction

Add `on_interaction_start` callback:

```rust
pub fn use_canvas_interaction<F, G>(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    on_interaction_start: G,
    on_interaction_end: F,
) -> InteractionHandle
where
    F: Fn(PixelTransform) + 'static,
    G: Fn() + 'static,
```

The callback fires when:
- `pointerdown` (drag starts)
- First `wheel` event (zoom starts)
- `dblclick` (discrete zoom)
- Window resize starts

This allows `InteractiveCanvas` to cancel renders immediately when the user starts interacting.

## InteractiveCanvas Integration

```rust
#[component]
pub fn InteractiveCanvas(
    viewport: Signal<Viewport>,
    on_viewport_change: Callback<Viewport>,
    config: Signal<&'static FractalConfig>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Create renderer
    let renderer = AsyncProgressiveRenderer::new(config.get_untracked());

    // Wire up interaction with cancel on start
    let renderer_for_cancel = renderer.clone();
    let _interaction = use_canvas_interaction(
        canvas_ref,
        move || renderer_for_cancel.cancel(),
        move |transform| {
            let new_vp = apply_pixel_transform_to_viewport(
                &viewport.get_untracked(),
                &transform,
                canvas_size,
            );
            on_viewport_change.call(new_vp);
        },
    );

    // Render on viewport change
    create_effect(move |_| {
        let vp = viewport.get();
        if let Some(canvas) = canvas_ref.get() {
            renderer.render(&vp, &canvas);
        }
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
```

## Cache Strategy

The renderer caches computed data (not pixels) per tile:

```rust
struct TileCache {
    viewport: Option<Viewport>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<ComputeData>,  // Full canvas worth of data
}
```

**Cache hit (same viewport, different colorizer):**
- Skip computation
- Re-colorize from cached data
- Instant color scheme switching

**Cache miss (viewport changed or resize):**
- Clear cache
- Recompute all tiles

## Testing Strategy

**Unit tests:**
- `generate_tiles` covers canvas exactly (no gaps/overlaps)
- Center-out ordering correct
- `RenderProgress` percentage calculation
- `yield_to_browser` resolves after rAF

**Browser tests (wasm-pack):**
- Renderer creates valid progress signal
- Cancel stops tile iteration
- Tiles draw to correct canvas positions

**Manual tests:**
- Tiles appear progressively from center
- UI stays responsive during render
- Pan/zoom cancels current render
- Progress updates in UI panel
