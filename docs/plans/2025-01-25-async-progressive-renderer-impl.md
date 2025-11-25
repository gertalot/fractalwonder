# Async Progressive Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement tiled progressive rendering that keeps UI responsive during multi-minute renders.

**Architecture:** `AsyncProgressiveRenderer` divides canvas into tiles, renders them one-by-one using `spawn_local`, yields to browser via `requestAnimationFrame` between tiles. Cancellation via render_id comparison.

**Tech Stack:** Rust, Leptos, wasm-bindgen-futures, web-sys

---

## Task 1: Add `on_interaction_start` Callback to Hook

**Files:**
- Modify: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs`

**Step 1: Update function signature**

Change the function signature to accept two callbacks:

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

**Step 2: Store the callback**

After line 227 (`let on_interaction_end = store_value(on_interaction_end);`), add:

```rust
let on_interaction_start = store_value(on_interaction_start);
```

**Step 3: Call callback in `start_interaction`**

Modify the `start_interaction` closure (around line 211) to call the callback at the end:

```rust
let start_interaction = move || {
    let canvas_ref = canvas_ref_stored.get_value();
    if let Some(canvas) = canvas_ref.get_untracked() {
        if let Ok(image_data) = capture_canvas_image_data(&canvas) {
            initial_image_data.set_value(Some(image_data));
            initial_canvas_size.set_value(Some((canvas.width(), canvas.height())));
            base_offset.set_value((0.0, 0.0));
            current_drag_offset.set_value((0.0, 0.0));
            accumulated_zoom.set_value(1.0);
            zoom_center.set_value(None);
            transform_sequence.set_value(Vec::new());

            // Fire interaction start callback
            on_interaction_start.with_value(|cb| cb());
        }
    }
};
```

**Step 4: Update InteractiveCanvas call site**

In `fractalwonder-ui/src/components/interactive_canvas.rs`, update the hook call (around line 35):

```rust
let _interaction = use_canvas_interaction(
    canvas_ref,
    move || {
        // Cancel render on interaction start (placeholder for now)
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Interaction started - would cancel render"));
    },
    move |transform| {
        let current_vp = viewport.get_untracked();
        let size = canvas_size.get_untracked();

        if size.0 > 0 && size.1 > 0 {
            let new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform, size);
            on_viewport_change.call(new_vp);
        }
    },
);
```

**Step 5: Run tests**

```bash
cargo test --package fractalwonder-ui -- --nocapture
cargo clippy --package fractalwonder-ui -- -D warnings
```

**Step 6: Manual browser test**

Open app, drag canvas, check browser console shows "Interaction started - would cancel render".

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/hooks/use_canvas_interaction.rs fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(hooks): add on_interaction_start callback to use_canvas_interaction"
```

---

## Task 2: Create RenderProgress Struct

**Files:**
- Create: `fractalwonder-ui/src/rendering/render_progress.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create render_progress.rs**

```rust
/// Progress information for ongoing renders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    /// Create new progress tracker.
    pub fn new(total_tiles: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    /// Calculate completion percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progress_starts_at_zero() {
        let progress = RenderProgress::new(100);
        assert_eq!(progress.completed_tiles, 0);
        assert_eq!(progress.total_tiles, 100);
        assert!(!progress.is_complete);
    }

    #[test]
    fn percentage_calculation() {
        let mut progress = RenderProgress::new(100);
        progress.completed_tiles = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);
    }

    #[test]
    fn percentage_zero_tiles() {
        let progress = RenderProgress::new(0);
        assert!((progress.percentage() - 0.0).abs() < 0.001);
    }

    #[test]
    fn percentage_complete() {
        let mut progress = RenderProgress::new(64);
        progress.completed_tiles = 64;
        assert!((progress.percentage() - 100.0).abs() < 0.001);
    }
}
```

**Step 2: Update mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod render_progress;

pub use render_progress::RenderProgress;
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-ui render_progress -- --nocapture
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/render_progress.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(rendering): add RenderProgress struct for tile tracking"
```

---

## Task 3: Create Tile Generation Utilities

**Files:**
- Create: `fractalwonder-ui/src/rendering/tiles.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create tiles.rs**

```rust
use fractalwonder_core::PixelRect;

/// Calculate tile size based on zoom level.
///
/// Uses smaller tiles at deep zoom for more frequent progress updates.
pub fn calculate_tile_size(zoom_factor: f64) -> u32 {
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;
    const NORMAL_TILE_SIZE: u32 = 128;
    const DEEP_ZOOM_TILE_SIZE: u32 = 64;

    if zoom_factor >= DEEP_ZOOM_THRESHOLD {
        DEEP_ZOOM_TILE_SIZE
    } else {
        NORMAL_TILE_SIZE
    }
}

/// Generate tiles covering the canvas, sorted by distance from center.
///
/// Center-out ordering provides better UX - users see the most important
/// part of the image first.
pub fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    // Generate grid of tiles
    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);
            tiles.push(PixelRect::new(x_start, y_start, w, h));
        }
    }

    // Sort by distance from canvas center
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let a_center_x = a.x as f64 + a.width as f64 / 2.0;
        let a_center_y = a.y as f64 + a.height as f64 / 2.0;
        let a_dist_sq = (a_center_x - center_x).powi(2) + (a_center_y - center_y).powi(2);

        let b_center_x = b.x as f64 + b.width as f64 / 2.0;
        let b_center_y = b.y as f64 + b.height as f64 / 2.0;
        let b_dist_sq = (b_center_x - center_x).powi(2) + (b_center_y - center_y).powi(2);

        a_dist_sq.partial_cmp(&b_dist_sq).unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_zoom_uses_128px_tiles() {
        assert_eq!(calculate_tile_size(1.0), 128);
        assert_eq!(calculate_tile_size(1e9), 128);
    }

    #[test]
    fn deep_zoom_uses_64px_tiles() {
        assert_eq!(calculate_tile_size(1e10), 64);
        assert_eq!(calculate_tile_size(1e50), 64);
    }

    #[test]
    fn generate_tiles_covers_canvas_exactly() {
        let tiles = generate_tiles(256, 256, 64);

        // Should be 4x4 = 16 tiles
        assert_eq!(tiles.len(), 16);

        // Total area should equal canvas area
        let total_area: u32 = tiles.iter().map(|t| t.area()).sum();
        assert_eq!(total_area, 256 * 256);
    }

    #[test]
    fn generate_tiles_handles_non_divisible_sizes() {
        let tiles = generate_tiles(100, 100, 64);

        // 100/64 = 1.56, so 2x2 = 4 tiles
        assert_eq!(tiles.len(), 4);

        // Edge tiles should be smaller
        let has_partial_width = tiles.iter().any(|t| t.width == 36);
        let has_partial_height = tiles.iter().any(|t| t.height == 36);
        assert!(has_partial_width);
        assert!(has_partial_height);
    }

    #[test]
    fn generate_tiles_center_out_ordering() {
        let tiles = generate_tiles(256, 256, 64);

        // First tile should be one of the center tiles
        let first = &tiles[0];
        let first_center_x = first.x as f64 + first.width as f64 / 2.0;
        let first_center_y = first.y as f64 + first.height as f64 / 2.0;

        // Should be close to canvas center (128, 128)
        let dist_to_center = ((first_center_x - 128.0).powi(2) + (first_center_y - 128.0).powi(2)).sqrt();
        assert!(dist_to_center < 64.0, "First tile should be near center");

        // Last tile should be a corner
        let last = &tiles[tiles.len() - 1];
        let last_center_x = last.x as f64 + last.width as f64 / 2.0;
        let last_center_y = last.y as f64 + last.height as f64 / 2.0;
        let last_dist = ((last_center_x - 128.0).powi(2) + (last_center_y - 128.0).powi(2)).sqrt();
        assert!(last_dist > dist_to_center, "Last tile should be farther from center");
    }

    #[test]
    fn generate_tiles_no_overlap() {
        let tiles = generate_tiles(256, 256, 64);

        for (i, a) in tiles.iter().enumerate() {
            for (j, b) in tiles.iter().enumerate() {
                if i == j {
                    continue;
                }
                // Check no overlap: rectangles overlap if they intersect in both x and y
                let x_overlap = a.x < b.x + b.width && a.x + a.width > b.x;
                let y_overlap = a.y < b.y + b.height && a.y + a.height > b.y;
                assert!(!(x_overlap && y_overlap), "Tiles {} and {} overlap", i, j);
            }
        }
    }
}
```

**Step 2: Update mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod tiles;

pub use tiles::{calculate_tile_size, generate_tiles};
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-ui tiles -- --nocapture
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/tiles.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(rendering): add tile generation with center-out ordering"
```

---

## Task 4: Create Canvas Utilities

**Files:**
- Create: `fractalwonder-ui/src/rendering/canvas_utils.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`
- Modify: `fractalwonder-ui/Cargo.toml` (add futures dependency)

**Step 1: Add futures dependency**

In `fractalwonder-ui/Cargo.toml`, add to `[dependencies]`:

```toml
futures = "0.3"
```

**Step 2: Create canvas_utils.rs**

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Yield to browser event loop via requestAnimationFrame.
///
/// Returns a Future that resolves on the next animation frame,
/// allowing the browser to handle events and paint between tiles.
pub async fn yield_to_browser() {
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();

    let closure = Closure::once(move || {
        let _ = sender.send(());
    });

    web_sys::window()
        .expect("should have window")
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("should register rAF");

    closure.forget();
    let _ = receiver.await;
}

/// Get the current time in milliseconds (for elapsed time tracking).
pub fn performance_now() -> f64 {
    web_sys::window()
        .expect("should have window")
        .performance()
        .expect("should have performance")
        .now()
}

/// Get 2D rendering context from canvas.
pub fn get_2d_context(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, JsValue> {
    canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("No 2d context"))?
        .dyn_into::<CanvasRenderingContext2d>()
}

/// Draw RGBA pixel data to canvas at specified position.
pub fn draw_pixels_to_canvas(
    ctx: &CanvasRenderingContext2d,
    pixels: &[u8],
    width: u32,
    x: f64,
    y: f64,
) -> Result<(), JsValue> {
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(pixels), width, pixels.len() as u32 / width / 4)?;
    ctx.put_image_data(&image_data, x, y)
}

#[cfg(test)]
mod tests {
    // Note: These are browser-only functions, so unit tests are limited.
    // Real testing happens in wasm-pack browser tests.
}
```

**Step 3: Update mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod canvas_utils;

pub use canvas_utils::{draw_pixels_to_canvas, get_2d_context, performance_now, yield_to_browser};
```

**Step 4: Check it compiles**

```bash
cargo check --package fractalwonder-ui
cargo clippy --package fractalwonder-ui -- -D warnings
```

**Step 5: Commit**

```bash
git add fractalwonder-ui/Cargo.toml fractalwonder-ui/src/rendering/canvas_utils.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(rendering): add canvas utilities with yield_to_browser"
```

---

## Task 5: Create Colorizer Type and Dispatch

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Add Colorizer type alias and dispatch function**

Update `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
pub mod mandelbrot;
pub mod test_image;

use fractalwonder_core::ComputeData;

pub use mandelbrot::colorize as colorize_mandelbrot;
pub use test_image::colorize as colorize_test_image;

/// Colorizer function type - converts compute data to RGBA pixels.
pub type Colorizer = fn(&ComputeData) -> [u8; 4];

/// Dispatch colorization based on ComputeData variant.
pub fn colorize(data: &ComputeData) -> [u8; 4] {
    match data {
        ComputeData::TestImage(d) => colorize_test_image(d),
        ComputeData::Mandelbrot(d) => colorize_mandelbrot(d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{MandelbrotData, TestImageData};

    #[test]
    fn colorize_dispatches_test_image() {
        let data = ComputeData::TestImage(TestImageData {
            checkerboard_white: true,
            is_on_origin: false,
            circle_distance: 1.0,
        });
        let color = colorize(&data);
        // Should be white (checkerboard_white = true, not on origin)
        assert_eq!(color, [255, 255, 255, 255]);
    }

    #[test]
    fn colorize_dispatches_mandelbrot() {
        let data = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 0,
            max_iterations: 1000,
            escaped: false,
        });
        let color = colorize(&data);
        // Should be black (in set)
        assert_eq!(color, [0, 0, 0, 255]);
    }
}
```

**Step 2: Run tests**

```bash
cargo test --package fractalwonder-ui colorizers -- --nocapture
```

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add Colorizer type and dispatch function"
```

---

## Task 6: Create AsyncProgressiveRenderer

**Files:**
- Create: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create the struct and basic methods**

Create `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`:

```rust
use crate::config::FractalConfig;
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::canvas_utils::{draw_pixels_to_canvas, get_2d_context, performance_now, yield_to_browser};
use crate::rendering::RenderProgress;
use fractalwonder_compute::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{calculate_max_iterations, fit_viewport_to_canvas, ComputeData, PixelRect, Viewport};
use leptos::*;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlCanvasElement;

/// Async progressive renderer that yields to browser between tiles.
///
/// Renders tiles one-by-one, drawing each to canvas immediately and yielding
/// via requestAnimationFrame to keep UI responsive during long renders.
#[derive(Clone)]
pub struct AsyncProgressiveRenderer {
    config: &'static FractalConfig,
    progress: RwSignal<RenderProgress>,
    render_id: Rc<Cell<u32>>,
}

impl AsyncProgressiveRenderer {
    /// Create a new renderer for the given fractal config.
    pub fn new(config: &'static FractalConfig) -> Self {
        Self {
            config,
            progress: create_rw_signal(RenderProgress::default()),
            render_id: Rc::new(Cell::new(0)),
        }
    }

    /// Get progress signal for UI binding.
    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }

    /// Cancel any in-progress render.
    pub fn cancel(&self) {
        // Increment render_id - the async loop checks this before each tile
        self.render_id.set(self.render_id.get().wrapping_add(1));
    }

    /// Start rendering viewport to canvas.
    ///
    /// Returns immediately - rendering happens asynchronously.
    /// Previous render is automatically cancelled.
    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Cancel any existing render and start new one
        let render_id = self.render_id.get().wrapping_add(1);
        self.render_id.set(render_id);

        // Calculate tile size (smaller at deep zoom for progress feedback)
        let reference_width = self.config.default_viewport(viewport.precision_bits()).width;
        let zoom = reference_width.to_f64() / viewport.width.to_f64();
        let tile_size = calculate_tile_size(zoom);

        // Generate tiles in center-out order
        let tiles = generate_tiles(width, height, tile_size);
        let total_tiles = tiles.len() as u32;

        // Reset progress
        self.progress.set(RenderProgress::new(total_tiles));

        // Clone what we need for async block
        let render_id_cell = self.render_id.clone();
        let progress = self.progress;
        let config_id = self.config.id;
        let vp = viewport.clone();
        let ctx = match get_2d_context(canvas) {
            Ok(ctx) => ctx,
            Err(_) => return,
        };
        let canvas_size = (width, height);
        let max_iters = calculate_max_iterations(&viewport.width, &reference_width);

        spawn_local(async move {
            let start_time = performance_now();

            for (i, tile) in tiles.iter().enumerate() {
                // Check cancellation before each tile
                if render_id_cell.get() != render_id {
                    return;
                }

                // Compute tile
                let tile_viewport = tile_to_viewport(&tile, &vp, canvas_size);
                let tile_size = (tile.width, tile.height);

                let computed_data: Vec<ComputeData> = match config_id {
                    "test_image" => {
                        let renderer = TestImageRenderer;
                        renderer.render(&tile_viewport, tile_size)
                            .into_iter()
                            .map(ComputeData::TestImage)
                            .collect()
                    }
                    "mandelbrot" => {
                        let renderer = MandelbrotRenderer::new(max_iters);
                        renderer.render(&tile_viewport, tile_size)
                            .into_iter()
                            .map(ComputeData::Mandelbrot)
                            .collect()
                    }
                    _ => continue,
                };

                // Colorize
                let pixels: Vec<u8> = computed_data
                    .iter()
                    .flat_map(|data| colorize(data))
                    .collect();

                // Draw to canvas
                let _ = draw_pixels_to_canvas(&ctx, &pixels, tile.width, tile.x as f64, tile.y as f64);

                // Update progress
                progress.update(|p| {
                    p.completed_tiles = (i + 1) as u32;
                    p.elapsed_ms = performance_now() - start_time;
                });

                // Yield to browser
                yield_to_browser().await;
            }

            // Mark complete
            progress.update(|p| {
                p.is_complete = true;
                p.elapsed_ms = performance_now() - start_time;
            });
        });
    }
}

/// Convert a pixel-space tile to its corresponding fractal-space viewport.
fn tile_to_viewport(tile: &PixelRect, viewport: &Viewport, canvas_size: (u32, u32)) -> Viewport {
    let (canvas_width, canvas_height) = canvas_size;
    let precision = viewport.precision_bits();

    // Calculate fractal-space dimensions per pixel
    let pixel_width = viewport.width.div(&fractalwonder_core::BigFloat::from_u32(canvas_width, precision));
    let pixel_height = viewport.height.div(&fractalwonder_core::BigFloat::from_u32(canvas_height, precision));

    // Calculate tile center in fractal space
    // Tile pixel center relative to canvas center
    let canvas_center_x = canvas_width as f64 / 2.0;
    let canvas_center_y = canvas_height as f64 / 2.0;
    let tile_center_x = tile.x as f64 + tile.width as f64 / 2.0;
    let tile_center_y = tile.y as f64 + tile.height as f64 / 2.0;

    let offset_x = tile_center_x - canvas_center_x;
    let offset_y = tile_center_y - canvas_center_y;

    // Convert pixel offset to fractal offset
    let offset_x_bf = pixel_width.mul(&fractalwonder_core::BigFloat::with_precision(offset_x, precision));
    let offset_y_bf = pixel_height.mul(&fractalwonder_core::BigFloat::with_precision(offset_y, precision));

    // Note: In fractal space, Y increases upward, but in pixel space Y increases downward
    // So we negate the Y offset
    let center_x = viewport.center.0.add(&offset_x_bf);
    let center_y = viewport.center.1.sub(&offset_y_bf);

    // Tile dimensions in fractal space
    let tile_width = pixel_width.mul(&fractalwonder_core::BigFloat::from_u32(tile.width, precision));
    let tile_height = pixel_height.mul(&fractalwonder_core::BigFloat::from_u32(tile.height, precision));

    Viewport::with_bigfloat(center_x, center_y, tile_width, tile_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_to_viewport_center_tile() {
        // Viewport centered at origin
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        // Center tile of a 200x200 canvas with 100x100 tiles
        let tile = PixelRect::new(50, 50, 100, 100);
        let canvas_size = (200, 200);

        let tile_vp = tile_to_viewport(&tile, &vp, canvas_size);

        // Center should be at origin (0, 0)
        assert!((tile_vp.center.0.to_f64() - 0.0).abs() < 0.001);
        assert!((tile_vp.center.1.to_f64() - 0.0).abs() < 0.001);

        // Width/height should be 2.0 (half of viewport since tile is half of canvas)
        assert!((tile_vp.width.to_f64() - 2.0).abs() < 0.001);
        assert!((tile_vp.height.to_f64() - 2.0).abs() < 0.001);
    }

    #[test]
    fn tile_to_viewport_top_left_tile() {
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        // Top-left tile
        let tile = PixelRect::new(0, 0, 100, 100);
        let canvas_size = (200, 200);

        let tile_vp = tile_to_viewport(&tile, &vp, canvas_size);

        // Center should be at (-1, 1) - left and up from origin
        // Pixel center is at (50, 50), canvas center at (100, 100)
        // Offset: (-50, -50) pixels = (-1, +1) in fractal space (Y inverted)
        assert!((tile_vp.center.0.to_f64() - (-1.0)).abs() < 0.001);
        assert!((tile_vp.center.1.to_f64() - 1.0).abs() < 0.001);
    }
}
```

**Step 2: Update mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod async_progressive_renderer;

pub use async_progressive_renderer::AsyncProgressiveRenderer;
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-ui async_progressive_renderer -- --nocapture
cargo clippy --package fractalwonder-ui -- -D warnings
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(rendering): add AsyncProgressiveRenderer with tile-by-tile async rendering"
```

---

## Task 7: Refactor InteractiveCanvas to Use Renderer

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Replace synchronous rendering with AsyncProgressiveRenderer**

Replace the entire file content with:

```rust
use crate::config::FractalConfig;
use crate::hooks::use_canvas_interaction;
use crate::rendering::{AsyncProgressiveRenderer, RenderProgress};
use fractalwonder_core::{apply_pixel_transform_to_viewport, Viewport};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Callback fired when canvas dimensions change
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Store canvas size for use in callbacks
    let canvas_size = create_rw_signal((0u32, 0u32));

    // Create renderer - recreate when config changes
    let renderer = create_memo(move |_| {
        AsyncProgressiveRenderer::new(config.get())
    });

    // Wire up interaction hook with cancel on start
    let renderer_for_cancel = renderer.clone();
    let _interaction = use_canvas_interaction(
        canvas_ref,
        move || {
            renderer_for_cancel.get().cancel();
        },
        move |transform| {
            let current_vp = viewport.get_untracked();
            let size = canvas_size.get_untracked();

            if size.0 > 0 && size.1 > 0 {
                let new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform, size);
                on_viewport_change.call(new_vp);
            }
        },
    );

    // Effect to handle window resize
    create_effect(move |_| {
        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let window = web_sys::window().expect("should have window");
        let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
        let height = window.inner_height().unwrap().as_f64().unwrap() as u32;

        if width == 0 || height == 0 {
            return;
        }

        let current_size = canvas_size.get_untracked();
        if current_size.0 == width && current_size.1 == height {
            return;
        }

        // Update canvas dimensions
        canvas.set_width(width);
        canvas.set_height(height);

        // Store for interaction callback
        canvas_size.set((width, height));

        // Notify parent of dimensions
        if let Some(callback) = on_resize {
            callback.call((width, height));
        }
    });

    // Render effect - triggers async render on viewport/config change
    create_effect(move |_| {
        let vp = viewport.get();
        let _cfg = config.get(); // Subscribe to config changes
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            return;
        }

        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        // Start async render (previous render is auto-cancelled)
        renderer.get().render(&vp, canvas);
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
```

**Step 2: Check it compiles**

```bash
cargo check --package fractalwonder-ui
cargo clippy --package fractalwonder-ui -- -D warnings
```

**Step 3: Manual browser test**

Open app, verify:
1. Image renders progressively (tiles appear from center outward)
2. Pan/zoom works (preview shows, then re-render starts)
3. No UI freezing during render

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "refactor(canvas): use AsyncProgressiveRenderer for non-blocking tile rendering"
```

---

## Task 8: Add Progress Display to UI Panel

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs`
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Add progress prop to UIPanel**

Update `fractalwonder-ui/src/components/ui_panel.rs` to accept and display progress:

First, check the current UIPanel implementation to understand its structure, then add a progress display section.

**Step 2: Pass progress from App to UIPanel**

Wire the renderer's progress signal through App to UIPanel.

**Step 3: Manual browser test**

Verify progress bar/percentage shows during render.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs fractalwonder-ui/src/app.rs
git commit -m "feat(ui): add render progress display to UI panel"
```

---

## Summary

| Task | Description | Est. Time |
|------|-------------|-----------|
| 1 | Add `on_interaction_start` callback | 10 min |
| 2 | Create RenderProgress struct | 5 min |
| 3 | Create tile generation utilities | 10 min |
| 4 | Create canvas utilities | 10 min |
| 5 | Add Colorizer type and dispatch | 5 min |
| 6 | Create AsyncProgressiveRenderer | 20 min |
| 7 | Refactor InteractiveCanvas | 15 min |
| 8 | Add progress display to UI | 10 min |

**Total: ~85 minutes**

## Verification Checklist

After all tasks complete:

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] App loads without errors
- [ ] Tiles render progressively from center
- [ ] Pan/zoom cancels render and shows preview
- [ ] Progress updates in UI panel
- [ ] No UI freezing during long renders
