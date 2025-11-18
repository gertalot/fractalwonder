# Progressive Rendering (Single-Threaded) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build true async progressive rendering with responsive UI during long renders

**Architecture:** Replace synchronous tile rendering loop with async scheduling via `requestAnimationFrame`. Each tile renders asynchronously, yielding control to browser between tiles. Main thread stays responsive, user can interact during render, immediate cancellation on pan/zoom.

**Tech Stack:** Rust/WASM, Leptos 0.6, web-sys, wasm-bindgen, JS Closures

---

## Background

**Current Problem (TilingCanvasRenderer):**
- Line 148: Synchronous `for (_tile_idx, tile_rect) in tiles.iter().enumerate()`
- Loop never yields → blocks main thread until all tiles complete
- Progressive *display* (tiles appear one by one) but NOT progressive *execution*
- UI frozen during 30-minute renders

**Solution:**
- Replace synchronous loop with async tile queue
- Use `requestAnimationFrame` to schedule one tile per frame
- Browser event loop handles UI events between tiles
- Cancel render immediately on viewport change

---

## Task 1: Create AsyncProgressiveRenderer Structure

**Files:**
- Create: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create skeleton async_progressive_renderer.rs**

Create `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`:

```rust
use crate::rendering::{CanvasRenderer, Colorizer};
use fractalwonder_compute::Renderer;
use fractalwonder_core::{PixelRect, Rect, Viewport};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Rendering state for async tile processing
struct RenderState<S, D: Clone> {
    viewport: Viewport<S>,
    canvas_size: (u32, u32),
    remaining_tiles: Vec<PixelRect>,
    computed_data: Vec<D>,
    render_id: u32,
}

/// Cached state between renders
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

/// Async progressive canvas renderer - yields between tiles
pub struct AsyncProgressiveRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
    current_render: Rc<RefCell<Option<RenderState<S, D>>>>,
}

impl<S, D: Clone> Clone for AsyncProgressiveRenderer<S, D> {
    fn clone(&self) -> Self {
        Self {
            renderer: dyn_clone::clone_box(&*self.renderer),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state),
            current_render: Rc::clone(&self.current_render),
        }
    }
}

impl<S: Clone + PartialEq, D: Clone + Default + 'static> AsyncProgressiveRenderer<S, D> {
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
            current_render: Rc::new(RefCell::new(None)),
        }
    }
}
```

**Step 2: Add to module exports**

In `fractalwonder-ui/src/rendering/mod.rs`, add:

```rust
pub mod async_progressive_renderer;
pub use async_progressive_renderer::AsyncProgressiveRenderer;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully (skeleton only, no methods yet)

**Step 4: Commit skeleton**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs \
        fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat: add AsyncProgressiveRenderer skeleton structure"
```

---

## Task 2: Implement Basic Methods (Non-Async)

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Add basic renderer management methods**

Add to `impl<S: Clone + PartialEq, D: Clone + Default + 'static> AsyncProgressiveRenderer<S, D>`:

```rust
pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
    self.renderer = renderer;
    self.clear_cache();
}

pub fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
    self.colorizer = colorizer;
    // Cache preserved - no recomputation needed
}

pub fn natural_bounds(&self) -> Rect<S> {
    self.renderer.natural_bounds()
}

pub fn cancel_render(&self) {
    // Cancel in-progress async render
    let cache = self.cached_state.lock().unwrap();
    cache.render_id.fetch_add(1, Ordering::SeqCst);
    drop(cache);

    // Clear current render state
    *self.current_render.borrow_mut() = None;
}

fn clear_cache(&mut self) {
    let mut cache = self.cached_state.lock().unwrap();
    cache.viewport = None;
    cache.canvas_size = None;
    cache.data.clear();
}
```

**Step 2: Add tile computation helper (from TilingCanvasRenderer)**

Add at module level (outside impl):

```rust
/// Compute tiles for given canvas dimensions and tile size
fn compute_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let x = x_start;
            let y = y_start;
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);

            tiles.push(PixelRect::new(x, y, w, h));
        }
    }

    tiles
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 4: Commit basic methods**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "feat: add basic methods to AsyncProgressiveRenderer"
```

---

## Task 3: Implement Synchronous Colorization Helper

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Add colorize_and_display_tile method**

Add to impl block (adapted from TilingCanvasRenderer):

```rust
fn colorize_and_display_tile(&self, data: &[D], rect: PixelRect, canvas: &HtmlCanvasElement) {
    use wasm_bindgen::Clamped;
    use web_sys::{CanvasRenderingContext2d, ImageData};

    // Verify data length
    let expected_pixels = (rect.width * rect.height) as usize;
    if data.len() != expected_pixels {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Tile data mismatch: {} pixels, expected {}",
            data.len(),
            expected_pixels
        )));
        return;
    }

    // Convert data to RGBA pixels
    let pixels: Vec<u8> = data
        .iter()
        .flat_map(|d| {
            let (r, g, b, a) = (self.colorizer)(d);
            [r, g, b, a]
        })
        .collect();

    // Get 2D context
    let context = canvas
        .get_context("2d")
        .expect("Failed to get 2d context")
        .expect("2d context is None")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("Failed to cast to 2D context");

    // Create ImageData
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(&pixels),
        rect.width,
        rect.height,
    )
    .expect("Failed to create ImageData");

    // Put on canvas at tile position
    context
        .put_image_data(&image_data, rect.x as f64, rect.y as f64)
        .expect("Failed to put image data");
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 3: Commit colorization method**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "feat: add tile colorization method to AsyncProgressiveRenderer"
```

---

## Task 4: Implement Async Tile Rendering with requestAnimationFrame

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Add render_next_tile_async method**

Add to impl block:

```rust
fn render_next_tile_async(
    &self,
    canvas: HtmlCanvasElement,
) where
    S: Clone + 'static,
{
    // Clone Rc for closure
    let current_render = Rc::clone(&self.current_render);
    let cached_state = Arc::clone(&self.cached_state);
    let renderer = dyn_clone::clone_box(&*self.renderer);
    let colorizer = self.colorizer;
    let self_clone = self.clone();

    // Get current render state
    let mut render_state = current_render.borrow_mut();
    let state = match render_state.as_mut() {
        Some(s) => s,
        None => {
            // No active render
            return;
        }
    };

    // Check if cancelled
    let cache = cached_state.lock().unwrap();
    let current_render_id = cache.render_id.load(Ordering::SeqCst);
    drop(cache);

    if current_render_id != state.render_id {
        // Render cancelled
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Render {} cancelled",
            state.render_id
        )));
        *render_state = None;
        return;
    }

    // Get next tile
    let tile_rect = match state.remaining_tiles.pop() {
        Some(tile) => tile,
        None => {
            // All tiles complete - finalize render
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Render {} complete - all tiles finished",
                state.render_id
            )));

            // Update cache
            let mut cache = cached_state.lock().unwrap();
            cache.viewport = Some(state.viewport.clone());
            cache.canvas_size = Some(state.canvas_size);
            cache.data = state.computed_data.clone();

            *render_state = None;
            return;
        }
    };

    let viewport = state.viewport.clone();
    let canvas_size = state.canvas_size;

    // Drop mutable borrow before calling renderer
    drop(render_state);

    // Compute tile (synchronous computation, but async scheduling)
    let tile_data = renderer.render(&viewport, tile_rect, canvas_size);

    // Store in cache
    let mut render_state = current_render.borrow_mut();
    if let Some(state) = render_state.as_mut() {
        // Store tile data in raster order
        let width = state.canvas_size.0;
        let mut tile_data_idx = 0;
        for local_y in 0..tile_rect.height {
            let canvas_y = tile_rect.y + local_y;
            for local_x in 0..tile_rect.width {
                let canvas_x = tile_rect.x + local_x;
                let cache_idx = (canvas_y * width + canvas_x) as usize;
                state.computed_data[cache_idx] = tile_data[tile_data_idx].clone();
                tile_data_idx += 1;
            }
        }
    }
    drop(render_state);

    // Display tile immediately
    self_clone.colorize_and_display_tile(&tile_data, tile_rect, &canvas);

    // Schedule next tile via requestAnimationFrame
    let window = web_sys::window().expect("no global window");

    let closure = Closure::once(move || {
        self_clone.render_next_tile_async(canvas);
    });

    window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("requestAnimationFrame failed");

    closure.forget(); // Keep closure alive
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 3: Commit async tile rendering**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "feat: implement async tile rendering with requestAnimationFrame"
```

---

## Task 5: Implement Main render() Entry Point

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Add render() method**

Add to impl block:

```rust
pub fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement)
where
    S: Clone + 'static,
{
    let width = canvas.width();
    let height = canvas.height();
    let mut cache = self.cached_state.lock().unwrap();

    // Increment render ID to cancel any in-progress renders
    let current_render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

    // Decision: compute vs recolorize
    if cache.viewport.as_ref() == Some(viewport) && cache.canvas_size == Some((width, height)) {
        // Same viewport/size → recolorize from cache (synchronous)
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "RECOLORIZE from cache (render_id: {})",
            current_render_id
        )));

        let expected_pixels = (width * height) as usize;
        if cache.data.len() == expected_pixels {
            let full_rect = PixelRect::full_canvas(width, height);
            drop(cache); // Release lock before rendering
            self.colorize_and_display_tile(&cache.data, full_rect, canvas);
        } else {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Cache size mismatch: {} pixels, expected {}",
                cache.data.len(),
                expected_pixels
            )));
        }
    } else {
        // Viewport/size changed → async recompute
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "ASYNC RECOMPUTE (render_id: {})",
            current_render_id
        )));

        drop(cache); // Release lock
        self.start_async_render(viewport.clone(), canvas.clone(), current_render_id);
    }
}

fn start_async_render(
    &self,
    viewport: Viewport<S>,
    canvas: HtmlCanvasElement,
    render_id: u32,
) where
    S: Clone + 'static,
{
    let width = canvas.width();
    let height = canvas.height();

    // Compute all tiles up front
    let tiles = compute_tiles(width, height, self.tile_size);
    let total_tiles = tiles.len();

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
        "Starting async render: {} tiles ({}x{} canvas, {} tile_size)",
        total_tiles, width, height, self.tile_size
    )));

    // Initialize render state
    let render_state = RenderState {
        viewport,
        canvas_size: (width, height),
        remaining_tiles: tiles,
        computed_data: vec![D::default(); (width * height) as usize],
        render_id,
    };

    *self.current_render.borrow_mut() = Some(render_state);

    // Kick off first tile
    self.render_next_tile_async(canvas);
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 3: Commit render entry point**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "feat: implement main render entry point with async dispatch"
```

---

## Task 6: Implement CanvasRenderer Trait

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Implement CanvasRenderer trait**

Add at end of file:

```rust
impl<S: Clone + PartialEq + 'static, D: Clone + Default + 'static> CanvasRenderer
    for AsyncProgressiveRenderer<S, D>
{
    type Scalar = S;
    type Data = D;

    fn set_renderer(
        &mut self,
        renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    ) {
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
        self.cancel_render();
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 3: Commit trait implementation**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "feat: implement CanvasRenderer trait for AsyncProgressiveRenderer"
```

---

## Task 7: Add Tests for AsyncProgressiveRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`

**Step 1: Add test module**

Add at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_compute::TestImageComputer;
    use fractalwonder_core::{Point, Rect};

    fn test_colorizer(data: &fractalwonder_core::AppData) -> (u8, u8, u8, u8) {
        (255, 0, 0, 255) // Red
    }

    #[test]
    fn test_async_renderer_creation() {
        let computer = TestImageComputer::default();
        let renderer = fractalwonder_compute::PixelRenderer::new(Box::new(computer));

        let async_renderer = AsyncProgressiveRenderer::new(
            Box::new(renderer),
            test_colorizer,
            256,
        );

        assert_eq!(async_renderer.tile_size, 256);
    }

    #[test]
    fn test_cancel_render() {
        let computer = TestImageComputer::default();
        let renderer = fractalwonder_compute::PixelRenderer::new(Box::new(computer));

        let async_renderer = AsyncProgressiveRenderer::new(
            Box::new(renderer),
            test_colorizer,
            256,
        );

        // Cancel should increment render_id
        let cache = async_renderer.cached_state.lock().unwrap();
        let initial_id = cache.render_id.load(Ordering::SeqCst);
        drop(cache);

        async_renderer.cancel_render();

        let cache = async_renderer.cached_state.lock().unwrap();
        let new_id = cache.render_id.load(Ordering::SeqCst);
        assert_eq!(new_id, initial_id + 1);
    }

    #[test]
    fn test_compute_tiles() {
        // 512x512 canvas with 256 tile size → 4 tiles
        let tiles = compute_tiles(512, 512, 256);
        assert_eq!(tiles.len(), 4);

        // Verify tile positions
        assert_eq!(tiles[0], PixelRect::new(0, 0, 256, 256));
        assert_eq!(tiles[1], PixelRect::new(256, 0, 256, 256));
        assert_eq!(tiles[2], PixelRect::new(0, 256, 256, 256));
        assert_eq!(tiles[3], PixelRect::new(256, 256, 256, 256));
    }

    #[test]
    fn test_compute_tiles_partial() {
        // 300x200 with 256 tile size → edge tiles are smaller
        let tiles = compute_tiles(300, 200, 256);
        assert_eq!(tiles.len(), 2);

        assert_eq!(tiles[0], PixelRect::new(0, 0, 256, 200));
        assert_eq!(tiles[1], PixelRect::new(256, 0, 44, 200)); // Width = 300 - 256 = 44
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-ui`
Expected: All tests pass

**Step 3: Commit tests**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_renderer.rs
git commit -m "test: add unit tests for AsyncProgressiveRenderer"
```

---

## Task 8: Update App to Use AsyncProgressiveRenderer

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Find current TilingCanvasRenderer usage**

Search for `TilingCanvasRenderer` in app.rs to find where it's instantiated.

Expected location: Around line 110-130 where renderers are created.

**Step 2: Add AsyncProgressiveRenderer import**

At top of `fractalwonder-ui/src/app.rs`, add:

```rust
use crate::rendering::AsyncProgressiveRenderer;
```

**Step 3: Replace TilingCanvasRenderer with AsyncProgressiveRenderer**

Find the line creating `TilingCanvasRenderer` (example):

```rust
let canvas_renderer = TilingCanvasRenderer::new(renderer, colorizer, 256);
```

Replace with:

```rust
let canvas_renderer = AsyncProgressiveRenderer::new(renderer, colorizer, 256);
```

**Step 4: Update type annotations if needed**

If there are explicit type annotations referencing `TilingCanvasRenderer`, update them to `AsyncProgressiveRenderer` or use `Box<dyn CanvasRenderer<...>>`.

**Step 5: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 6: Build and test in browser**

Run: `trunk serve`

Then in browser:
- Navigate to `http://localhost:8080`
- Verify tiles appear progressively
- Verify UI is responsive during render (can click dropdown, etc.)
- Verify pan/zoom cancels current render

**Step 7: Commit app integration**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat: switch app to AsyncProgressiveRenderer"
```

---

## Task 9: Add Manual Browser Tests

**Files:**
- Create: `docs/testing/iteration-2-manual-tests.md`

**Step 1: Create manual test checklist**

Create `docs/testing/iteration-2-manual-tests.md`:

```markdown
# Iteration 2 Manual Browser Tests

**Feature:** Progressive Rendering (Single-Threaded)

**Date:** 2025-11-17

## Test Environment
- Browser: Chrome/Firefox/Safari
- URL: http://localhost:8080 (trunk serve)

## Test Checklist

### Progressive Display
- [ ] Tiles appear one by one during render (not all at once)
- [ ] Progress is visible (tiles gradually fill the canvas)
- [ ] No blank screen while waiting for render

### UI Responsiveness
- [ ] Can click dropdown menus during render
- [ ] Can hover over UI elements during render
- [ ] Mouse movements are smooth during render
- [ ] No perceptible lag in UI interactions

### Cancellation
- [ ] Pan (click-drag) immediately stops current render
- [ ] Zoom (scroll wheel) immediately stops current render
- [ ] New render starts immediately after cancellation
- [ ] Cancellation happens within 100ms (feels instant)

### Render Quality
- [ ] Tiles align correctly (no gaps or overlaps)
- [ ] Colors are consistent across tile boundaries
- [ ] Full canvas is eventually rendered
- [ ] Cached re-colorization works (change color scheme without re-computation)

### Edge Cases
- [ ] Resize browser window → renders correctly
- [ ] Very small viewport → renders correctly
- [ ] Rapid pan/zoom → no crashes, renders stay responsive
- [ ] Switch renderer during active render → cancels and restarts

## Test Procedure

1. **Start dev server:** `trunk serve`
2. **Open browser:** Navigate to http://localhost:8080
3. **Initial render:** Wait for default view to render
   - Observe: Tiles appear progressively
   - Verify: UI responsive checkbox
4. **Test pan:** Click and drag during render
   - Observe: Render stops immediately
   - Verify: New render starts at new position
5. **Test zoom:** Scroll wheel during render
   - Observe: Render stops immediately
   - Verify: New render starts at new zoom level
6. **Test UI:** Open dropdown menu during render
   - Observe: Menu opens without delay
   - Verify: Menu stays open, render continues
7. **Test rapid interaction:** Pan/zoom quickly multiple times
   - Observe: Each interaction cancels previous render
   - Verify: No crashes, UI stays responsive

## Pass Criteria

- ✓ All checklist items pass
- ✓ No console errors
- ✓ Subjective feel: UI is responsive and smooth
```

**Step 2: Run manual tests**

Follow the test procedure in the document. Mark each checkbox as you verify.

**Step 3: Commit test documentation**

```bash
mkdir -p docs/testing
git add docs/testing/iteration-2-manual-tests.md
git commit -m "docs: add manual browser tests for Iteration 2"
```

---

## Task 10: Performance Benchmarking

**Files:**
- Create: `docs/benchmarks/iteration-2-performance.md`

**Step 1: Add console timing to AsyncProgressiveRenderer**

Modify `start_async_render()` in `async_progressive_renderer.rs`:

Add at start:
```rust
#[cfg(target_arch = "wasm32")]
let start_time = web_sys::window()
    .and_then(|w| w.performance())
    .map(|p| p.now());
```

Modify the "All tiles complete" message to include timing:
```rust
#[cfg(target_arch = "wasm32")]
{
    let elapsed = start_time
        .and_then(|start| web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() - start))
        .unwrap_or(0.0);

    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
        "Render {} complete - {} tiles in {:.2}ms ({:.2}ms/tile avg)",
        state.render_id,
        total_tiles,
        elapsed,
        elapsed / total_tiles as f64
    )));
}
```

**Step 2: Create benchmark document**

Create `docs/benchmarks/iteration-2-performance.md`:

```markdown
# Iteration 2 Performance Benchmarks

**Configuration:**
- Tile size: 256x256
- Canvas: 1920x1080
- Renderer: Mandelbrot (default zoom)

## Metrics

### Baseline (TilingCanvasRenderer - Synchronous)
- Total render time: [measure]
- UI responsiveness: Blocked until complete
- Cancellation latency: N/A (cannot cancel mid-render)

### Iteration 2 (AsyncProgressiveRenderer)
- Total render time: [measure]
- Time to first tile: [measure]
- Average time per tile: [measure]
- UI responsiveness: Responsive throughout
- Cancellation latency: <100ms

## Methodology

1. Open browser DevTools → Console
2. Start render from default view
3. Record timing from console output
4. Test pan/zoom during render
5. Measure time from interaction to render stop

## Results

[To be filled after implementation]

## Analysis

Expected:
- Slightly slower total render time (overhead from async scheduling)
- Acceptable tradeoff for responsive UI
- First tile appears almost immediately
```

**Step 3: Run benchmarks and record results**

Use browser DevTools to measure and record timing.

**Step 4: Commit benchmark documentation**

```bash
git add docs/benchmarks/iteration-2-performance.md
git commit -m "docs: add performance benchmarks for Iteration 2"
```

---

## Task 11: Update Documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/workspace-structure.md`

**Step 1: Update README.md**

Add to the "Features" section:

```markdown
## Features

- **Progressive Rendering**: Tiles appear incrementally during long renders
- **Responsive UI**: Interact with controls while rendering (pan, zoom, change settings)
- **Immediate Cancellation**: Pan/zoom instantly stops current render and starts new one
```

**Step 2: Update workspace-structure.md**

Add section under `fractalwonder-ui`:

```markdown
### Progressive Rendering

**AsyncProgressiveRenderer** (Iteration 2):
- Async tile scheduling via `requestAnimationFrame`
- Main thread yields between tiles
- UI stays responsive during 30-minute renders
- Immediate cancellation on viewport change
- Single-threaded (workers added in Iteration 3)
```

**Step 3: Commit documentation updates**

```bash
git add README.md docs/architecture/workspace-structure.md
git commit -m "docs: document progressive rendering architecture"
```

---

## Task 12: Final Validation

**Files:**
- None (verification only)

**Step 1: Run full test suite**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 2: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings

**Step 3: Format check**

Run: `cargo fmt --all -- --check`
Expected: All code formatted

**Step 4: Build release**

Run: `trunk build --release`
Expected: Builds successfully

**Step 5: Manual browser testing**

Run: `trunk serve`

Verify all items in `docs/testing/iteration-2-manual-tests.md`:
- Progressive display ✓
- UI responsiveness ✓
- Cancellation ✓
- Render quality ✓
- Edge cases ✓

**Step 6: Create final commit and tag**

```bash
git add .
git commit -m "feat: complete Iteration 2 - Progressive Rendering (Single-Threaded)

- Implemented AsyncProgressiveRenderer with async tile scheduling
- Main thread stays responsive during renders via requestAnimationFrame
- Immediate cancellation on pan/zoom
- All tests pass, manual browser tests verified
"

git tag -a v0.3.0-progressive-single-threaded -m "Iteration 2 complete - Progressive Rendering"
```

---

## Success Criteria

**All of these must be true:**

- [x] AsyncProgressiveRenderer implemented and working
- [x] Tiles appear one by one during render (progressive display)
- [x] UI responds to clicks and keypresses while rendering
- [x] Pan or zoom stops current render within 100ms
- [x] All automated tests pass
- [x] No Clippy warnings
- [x] Code properly formatted
- [x] Manual browser tests all pass
- [x] Documentation updated

**Observable behavior:**
- User sees progress during long renders (not blank screen)
- UI never freezes (can always interact)
- Cancellation feels instant (no lag)
- Quality identical to synchronous renderer

---

## Next Steps After Completion

This progressive rendering foundation (Iteration 2) enables:

**Iteration 3:** Web Workers with wasm-bindgen-rayon (multi-core)
**Iteration 4:** Responsive cancellation (Worker-side)
**Iteration 5:** Tile scheduling optimization

Reference: `docs/multicore-plans/2025-11-17-progressive-parallel-rendering-design.md`
