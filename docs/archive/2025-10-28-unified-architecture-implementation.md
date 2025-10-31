# Unified Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor rendering architecture to separate computation from coloring, enable runtime renderer swapping, and establish clean component ownership hierarchy.

**Architecture:** Three-layer abstraction (ImagePointComputer → Renderer → CanvasRenderer) with trait objects for runtime polymorphism. Cache preservation via Arc-shared state when colorizer changes. App owns domain state, InteractiveCanvas owns interaction, UI owns presentation.

**Tech Stack:** Rust, Leptos 0.6, Web Canvas API, Arc/Mutex for shared state

---

## Task 1: Add Data Type to ImagePointComputer Trait

**Files:**
- Modify: `src/rendering/point_compute.rs:7-22`
- Modify: `src/rendering/point_compute.rs:32-42` (test implementation)

**Step 1: Add type Data to ImagePointComputer trait**

In `src/rendering/point_compute.rs`, update the trait:

```rust
pub trait ImagePointComputer {
    /// Coordinate type for image space
    type Coord;

    /// Data type output (NOT colors - will be colorized later)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Compute data for a single point in image space
    ///
    /// # Arguments
    /// * `coord` - Point in image-space coordinates
    ///
    /// # Returns
    /// Computation data (not RGBA - colorizer converts to colors)
    fn compute(&self, coord: Point<Self::Coord>) -> Self::Data;
}
```

**Step 2: Update test implementation**

In same file, update `SolidColorCompute`:

```rust
// Simple test implementation
struct SolidColorCompute {
    color: (u8, u8, u8, u8),
}

impl ImagePointComputer for SolidColorCompute {
    type Coord = f64;
    type Data = (u8, u8, u8, u8);  // For tests, Data = RGBA

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
    }

    fn compute(&self, _coord: Point<f64>) -> Self::Data {
        self.color
    }
}
```

**Step 3: Run tests**

Run: `cargo test --lib rendering::point_compute`
Expected: PASS (test still works with Data = RGBA)

**Step 4: Commit**

```bash
git add src/rendering/point_compute.rs
git commit -m "feat: add Data associated type to ImagePointComputer trait"
```

---

## Task 2: Add Data Type to Renderer Trait

**Files:**
- Modify: `src/rendering/renderer_trait.rs:6-28`

**Step 1: Add type Data and update render signature**

In `src/rendering/renderer_trait.rs`:

```rust
pub trait Renderer {
    /// Coordinate type for image space (f64, rug::Float, etc.)
    type Coord;

    /// Data type output (NOT colors - will be colorized later)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Render data for pixels in a given viewport and pixel rectangle
    ///
    /// # Arguments
    /// * `viewport` - What image coordinates the full canvas shows
    /// * `pixel_rect` - Which portion of canvas to render (for tiling)
    /// * `canvas_size` - Full canvas dimensions (width, height)
    ///
    /// # Returns
    /// Data for the specified pixel_rect (length = width * height)
    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data>;
}
```

**Step 2: Attempt compile**

Run: `cargo check`
Expected: FAIL - PixelRenderer and other implementors don't have type Data

**Step 3: Commit**

```bash
git add src/rendering/renderer_trait.rs
git commit -m "feat: add Data associated type to Renderer trait"
```

---

## Task 3: Update PixelRenderer to Work with Generic Data

**Files:**
- Modify: `src/rendering/pixel_renderer.rs:27-84`
- Modify: `src/rendering/pixel_renderer.rs:102-119` (test)

**Step 1: Add type Data and update render to return Vec<Data>**

In `src/rendering/pixel_renderer.rs`:

```rust
impl<C> Renderer for PixelRenderer<C>
where
    C: ImagePointComputer,
    C::Coord: Clone
        + std::ops::Sub<Output = C::Coord>
        + std::ops::Add<Output = C::Coord>
        + std::ops::Mul<f64, Output = C::Coord>
        + std::ops::Div<f64, Output = C::Coord>,
{
    type Coord = C::Coord;
    type Data = C::Data;  // Pass through Data from computer

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.computer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data> {
        let mut data = Vec::with_capacity((pixel_rect.width * pixel_rect.height) as usize);

        // Calculate visible bounds from viewport once
        let natural_bounds = self.computer.natural_bounds();
        let visible_bounds =
            calculate_visible_bounds(viewport, &natural_bounds, canvas_size.0, canvas_size.1);

        for local_y in 0..pixel_rect.height {
            for local_x in 0..pixel_rect.width {
                // Convert local pixel coords to absolute canvas coords
                let abs_x = pixel_rect.x + local_x;
                let abs_y = pixel_rect.y + local_y;

                // Map pixel to image coordinates
                let image_coord = pixel_to_image(
                    abs_x as f64,
                    abs_y as f64,
                    &visible_bounds,
                    canvas_size.0,
                    canvas_size.1,
                );

                // Compute data (not color!)
                let point_data = self.computer.compute(image_coord);
                data.push(point_data);
            }
        }

        data
    }
}
```

**Step 2: Update test implementation**

In same file:

```rust
struct TestCompute;

impl ImagePointComputer for TestCompute {
    type Coord = f64;
    type Data = (u8, u8, u8, u8);  // For test, Data = RGBA

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(-10.0, -10.0), Point::new(10.0, 10.0))
    }

    fn compute(&self, coord: Point<f64>) -> Self::Data {
        // Red if x > 0, blue otherwise
        if *coord.x() > 0.0 {
            (255, 0, 0, 255)
        } else {
            (0, 0, 255, 255)
        }
    }
}

#[test]
fn test_pixel_renderer_full_canvas() {
    let renderer = PixelRenderer::new(TestCompute);
    let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
    let pixel_rect = PixelRect::full_canvas(10, 10);
    let data = renderer.render(&viewport, pixel_rect, (10, 10));

    assert_eq!(data.len(), 10 * 10);

    // First pixel (top-left, x < 0) should be blue
    assert_eq!(data[0], (0, 0, 255, 255));
}

#[test]
fn test_pixel_renderer_partial_rect() {
    let renderer = PixelRenderer::new(TestCompute);
    let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
    // Render just a 5x5 tile starting at (2, 2)
    let pixel_rect = PixelRect::new(2, 2, 5, 5);
    let data = renderer.render(&viewport, pixel_rect, (10, 10));

    assert_eq!(data.len(), 5 * 5);
}
```

**Step 3: Run tests**

Run: `cargo test --lib rendering::pixel_renderer`
Expected: PASS

**Step 4: Commit**

```bash
git add src/rendering/pixel_renderer.rs
git commit -m "feat: update PixelRenderer to work with generic Data"
```

---

## Task 4: Create AppData Enum and TestImageData

**Files:**
- Create: `src/rendering/app_data.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create app_data.rs with enum**

Create `src/rendering/app_data.rs`:

```rust
/// Unified data type for all renderer implementations
///
/// Each renderer wraps its specific data type in this enum to enable
/// runtime polymorphism via trait objects.
#[derive(Clone, Debug)]
pub enum AppData {
    TestImage(TestImageData),
    // Future: Mandelbrot(MandelbrotData), etc.
}

/// Data computed by TestImageRenderer
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

impl TestImageData {
    pub fn new(checkerboard: bool, circle_distance: f64) -> Self {
        Self {
            checkerboard,
            circle_distance,
        }
    }
}
```

**Step 2: Export from mod.rs**

In `src/rendering/mod.rs`, add:

```rust
mod app_data;
pub use app_data::{AppData, TestImageData};
```

**Step 3: Compile check**

Run: `cargo check --lib`
Expected: PASS (new module compiles)

**Step 4: Commit**

```bash
git add src/rendering/app_data.rs src/rendering/mod.rs
git commit -m "feat: add AppData enum and TestImageData"
```

---

## Task 5: Create Colorizer Functions

**Files:**
- Create: `src/rendering/colorizers.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create colorizers.rs**

Create `src/rendering/colorizers.rs`:

```rust
use super::app_data::{AppData, TestImageData};

/// Colorizer function type - converts Data to RGBA
pub type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);

/// Colorize TestImageData
pub fn test_image_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImage(d) => colorize_test_image(d),
        #[allow(unreachable_patterns)]
        _ => (0, 0, 0, 255), // Black for wrong type
    }
}

fn colorize_test_image(data: &TestImageData) -> (u8, u8, u8, u8) {
    // Circle distance < 0.1 means on a circle -> red
    if data.circle_distance < 0.1 {
        return (255, 0, 0, 255); // Red circle
    }

    // Checkerboard pattern
    if data.checkerboard {
        (255, 255, 255, 255) // White
    } else {
        (204, 204, 204, 255) // Light grey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorizer_on_circle() {
        let data = AppData::TestImage(TestImageData::new(true, 0.05));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (255, 0, 0, 255)); // Red
    }

    #[test]
    fn test_colorizer_checkerboard_white() {
        let data = AppData::TestImage(TestImageData::new(true, 5.0));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (255, 255, 255, 255)); // White
    }

    #[test]
    fn test_colorizer_checkerboard_grey() {
        let data = AppData::TestImage(TestImageData::new(false, 5.0));
        let color = test_image_colorizer(&data);
        assert_eq!(color, (204, 204, 204, 255)); // Grey
    }
}
```

**Step 2: Export from mod.rs**

In `src/rendering/mod.rs`:

```rust
mod colorizers;
pub use colorizers::{test_image_colorizer, Colorizer};
```

**Step 3: Run tests**

Run: `cargo test --lib rendering::colorizers`
Expected: PASS

**Step 4: Commit**

```bash
git add src/rendering/colorizers.rs src/rendering/mod.rs
git commit -m "feat: add colorizer functions for AppData"
```

---

## Task 6: Update TestImageRenderer to Return TestImageData

**Files:**
- Modify: `src/components/test_image.rs:14-95`

**Step 1: Change compute_point_color to return TestImageData**

In `src/components/test_image.rs`:

```rust
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
    PixelRenderer, TestImageData, // Add TestImageData
};

impl TestImageRenderer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 5.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.1,
        }
    }

    fn compute_point_data(&self, x: f64, y: f64) -> TestImageData {
        // Calculate circle distance
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        // On circle if within line thickness and not at origin
        let circle_distance = if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
            ring_distance
        } else {
            ring_distance + 1.0  // Definitely not on circle
        };

        // Also treat vertical green line as a circle for now
        if x.abs() < self.circle_line_thickness {
            return TestImageData::new(false, 0.0); // Mark as on circle
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor() as i32;
        let square_y = (y / self.checkerboard_size).floor() as i32;
        let is_light = (square_x + square_y) % 2 == 0;

        TestImageData::new(is_light, circle_distance)
    }
}

impl ImagePointComputer for TestImageRenderer {
    type Coord = f64;
    type Data = TestImageData;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0))
    }

    fn compute(&self, coord: Point<f64>) -> TestImageData {
        self.compute_point_data(*coord.x(), *coord.y())
    }
}
```

**Step 2: Update tests**

In same file, update tests:

```rust
#[test]
fn test_checkerboard_pattern_at_origin() {
    let renderer = TestImageRenderer::new();

    // Point at (-2.5, -2.5) in square (-1, -1), sum=-2 (even) -> light
    let data1 = renderer.compute_point_data(-2.5, -2.5);
    // Point at (2.5, 2.5) in square (0, 0), sum=0 (even) -> light
    let data2 = renderer.compute_point_data(2.5, 2.5);
    // Point at (2.5, -2.5) in square (0, -1), sum=-1 (odd) -> dark
    let data3 = renderer.compute_point_data(2.5, -2.5);

    assert_eq!(data1.checkerboard, data2.checkerboard); // Both light
    assert_ne!(data1.checkerboard, data3.checkerboard); // data1 light, data3 dark
}

#[test]
fn test_circle_at_radius_10() {
    let renderer = TestImageRenderer::new();

    // Point exactly on circle (radius 10)
    let data_on = renderer.compute_point_data(10.0, 0.0);
    assert!(data_on.circle_distance < 0.1); // On circle

    // Point between circles
    let data_off = renderer.compute_point_data(15.0, 0.0);
    assert!(data_off.circle_distance > 0.1); // Not on circle
}

#[test]
fn test_origin_is_corner_of_four_squares() {
    let renderer = TestImageRenderer::new();

    // (0,0) is corner, so nearby points in different quadrants have different checkerboard
    let q1 = renderer.compute_point_data(1.0, 1.0);
    let q2 = renderer.compute_point_data(-1.0, 1.0);
    let q3 = renderer.compute_point_data(-1.0, -1.0);
    let q4 = renderer.compute_point_data(1.0, -1.0);

    // Opposite quadrants should have same checkerboard
    assert_eq!(q1.checkerboard, q3.checkerboard);
    assert_eq!(q2.checkerboard, q4.checkerboard);
    assert_ne!(q1.checkerboard, q2.checkerboard);
}
```

**Step 3: Run tests**

Run: `cargo test --lib components::test_image`
Expected: PASS

**Step 4: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: update TestImageRenderer to return TestImageData"
```

---

## Task 7: Create TilingCanvasRenderer

**Files:**
- Create: `src/rendering/tiling_canvas_renderer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create tiling_canvas_renderer.rs structure**

Create `src/rendering/tiling_canvas_renderer.rs`:

```rust
use crate::rendering::{
    canvas_utils::{get_image_data, put_image_data},
    points::Rect,
    renderer_trait::Renderer,
    viewport::Viewport,
    Colorizer, PixelRect,
};
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Cached rendering state
struct CachedState<R: Renderer> {
    viewport: Option<Viewport<R::Coord>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<R::Data>,
}

impl<R: Renderer> Default for CachedState<R> {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
        }
    }
}

/// Canvas renderer with tiling, progressive rendering, and caching
pub struct TilingCanvasRenderer<R: Renderer> {
    renderer: R,
    colorizer: Colorizer<R::Data>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<R>>>,
}

impl<R: Renderer> TilingCanvasRenderer<R> {
    pub fn new(renderer: R, colorizer: Colorizer<R::Data>, tile_size: u32) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
        }
    }

    /// Create new renderer with different colorizer, preserving cached data
    pub fn with_colorizer(&self, colorizer: Colorizer<R::Data>) -> Self
    where
        R: Clone,
    {
        Self {
            renderer: self.renderer.clone(),
            colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state), // Shared cache!
        }
    }

    pub fn natural_bounds(&self) -> Rect<R::Coord>
    where
        R::Coord: Clone,
    {
        self.renderer.natural_bounds()
    }
}
```

**Step 2: Compile check**

Run: `cargo check --lib`
Expected: PASS (structure compiles, no methods yet)

**Step 3: Commit**

```bash
git add src/rendering/tiling_canvas_renderer.rs src/rendering/mod.rs
git commit -m "feat: add TilingCanvasRenderer structure with cache"
```

---

## Task 8: Implement TilingCanvasRenderer Rendering Logic

**Files:**
- Modify: `src/rendering/tiling_canvas_renderer.rs` (add methods)

**Step 1: Add render method and helpers**

In `src/rendering/tiling_canvas_renderer.rs`, add methods to impl block:

```rust
impl<R: Renderer> TilingCanvasRenderer<R> {
    // ... existing new() and with_colorizer() ...

    /// Main render entry point
    pub fn render(&self, viewport: &Viewport<R::Coord>, canvas: &HtmlCanvasElement)
    where
        R::Coord: Clone + PartialEq,
    {
        let width = canvas.width();
        let height = canvas.height();
        let mut cache = self.cached_state.lock().unwrap();

        // Decision: compute vs recolorize
        if cache.viewport.as_ref() == Some(viewport) && cache.canvas_size == Some((width, height))
        {
            // Same viewport/size → recolorize from cache
            self.recolorize_from_cache(&cache, canvas);
        } else {
            // Viewport/size changed → recompute
            self.render_with_computation(viewport, canvas, &mut cache);
        }
    }

    fn render_with_computation(
        &self,
        viewport: &Viewport<R::Coord>,
        canvas: &HtmlCanvasElement,
        cache: &mut CachedState<R>,
    ) where
        R::Coord: Clone,
    {
        let width = canvas.width();
        let height = canvas.height();

        cache.data.clear();
        cache.data.reserve((width * height) as usize);

        // Progressive tiled rendering
        for tile_rect in compute_tiles(width, height, self.tile_size) {
            // Compute tile data
            let tile_data = self
                .renderer
                .render(viewport, tile_rect, (width, height));

            // Store in cache
            cache.data.extend(tile_data.iter().cloned());

            // Colorize and display tile immediately (progressive!)
            self.colorize_and_display_tile(&tile_data, tile_rect, canvas);
        }

        // Update cache metadata
        cache.viewport = Some(viewport.clone());
        cache.canvas_size = Some((width, height));
    }

    fn recolorize_from_cache(&self, cache: &CachedState<R>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();
        let full_rect = PixelRect::full_canvas(width, height);

        self.colorize_and_display_tile(&cache.data, full_rect, canvas);
    }

    fn colorize_and_display_tile(
        &self,
        data: &[R::Data],
        rect: PixelRect,
        canvas: &HtmlCanvasElement,
    ) {
        let rgba_bytes: Vec<u8> = data
            .iter()
            .flat_map(|d| {
                let (r, g, b, a) = (self.colorizer)(d);
                [r, g, b, a]
            })
            .collect();

        // Get canvas context and put image data
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&rgba_bytes),
            rect.width,
            rect.height,
        )
        .unwrap();

        context
            .put_image_data(&image_data, rect.x as f64, rect.y as f64)
            .unwrap();
    }
}

/// Compute tile rectangles for progressive rendering
fn compute_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y in (0..height).step_by(tile_size as usize) {
        for x in (0..width).step_by(tile_size as usize) {
            let tile_width = tile_size.min(width - x);
            let tile_height = tile_size.min(height - y);
            tiles.push(PixelRect::new(x, y, tile_width, tile_height));
        }
    }

    tiles
}
```

**Step 2: Export from mod.rs**

In `src/rendering/mod.rs`:

```rust
mod tiling_canvas_renderer;
pub use tiling_canvas_renderer::TilingCanvasRenderer;
```

**Step 3: Compile check**

Run: `cargo check --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/rendering/tiling_canvas_renderer.rs src/rendering/mod.rs
git commit -m "feat: implement TilingCanvasRenderer render logic"
```

---

## Task 9: Refactor TestImageView to Use New Architecture

**Files:**
- Modify: `src/components/test_image.rs:97-128` (component)

**Step 1: Update TestImageView component**

In `src/components/test_image.rs`, replace the component:

```rust
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
    PixelRenderer, TestImageData, AppData, test_image_colorizer,
    TilingCanvasRenderer, Colorizer,
};

#[component]
pub fn TestImageView() -> impl IntoView {
    // Create renderer (PixelRenderer wrapping TestImageRenderer)
    let test_computer = TestImageRenderer::new();
    let pixel_renderer = PixelRenderer::new(test_computer.clone());

    // IMPORTANT: Wrap in AppData by creating adapter
    // TODO: This will be cleaner once we create AppDataRenderer wrapper

    // For now, create TilingCanvasRenderer
    let canvas_renderer = create_rw_signal(TilingCanvasRenderer::new(
        pixel_renderer,
        test_image_colorizer as Colorizer<AppData>,
        128,
    ));

    let (viewport, set_viewport) = create_signal(
        Viewport::new(test_computer.natural_bounds().center())
    );

    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

    let renderer_info = create_memo(move |_| {
        test_computer.info(&viewport.get())
    });

    // UI visibility
    let ui_visibility = use_ui_visibility();

    // Reset viewport callback
    let on_home_click = move || {
        set_viewport.set(Viewport::new(test_computer.natural_bounds().center()));
    };

    // Fullscreen callback
    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="w-full h-full">
            // TODO: Replace InteractiveCanvas with new version
            <p>"New architecture placeholder"</p>
        </div>
        <UI
            info=renderer_info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
        />
    }
}
```

**Step 2: Compile check**

Run: `cargo check`
Expected: May fail due to AppData wrapper issue - that's okay, we'll fix next

**Step 3: Commit**

```bash
git add src/components/test_image.rs
git commit -m "wip: refactor TestImageView to use new architecture (incomplete)"
```

---

## Task 10: Create AppDataRenderer Wrapper

**Files:**
- Create: `src/rendering/app_data_renderer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create wrapper that converts Data to AppData**

Create `src/rendering/app_data_renderer.rs`:

```rust
use super::{
    app_data::AppData, points::Rect, renderer_trait::Renderer, viewport::Viewport, PixelRect,
};

/// Wrapper that converts a Renderer<Data=D> to Renderer<Data=AppData>
///
/// This enables using specific renderers (like PixelRenderer<TestImageRenderer>)
/// in contexts that expect AppData.
pub struct AppDataRenderer<R, F>
where
    R: Renderer,
    F: Fn(&R::Data) -> AppData + Clone,
{
    renderer: R,
    wrap_fn: F,
}

impl<R, F> AppDataRenderer<R, F>
where
    R: Renderer,
    F: Fn(&R::Data) -> AppData + Clone,
{
    pub fn new(renderer: R, wrap_fn: F) -> Self {
        Self { renderer, wrap_fn }
    }
}

impl<R, F> Clone for AppDataRenderer<R, F>
where
    R: Renderer + Clone,
    F: Fn(&R::Data) -> AppData + Clone,
{
    fn clone(&self) -> Self {
        Self {
            renderer: self.renderer.clone(),
            wrap_fn: self.wrap_fn.clone(),
        }
    }
}

impl<R, F> Renderer for AppDataRenderer<R, F>
where
    R: Renderer,
    R::Coord: Clone,
    F: Fn(&R::Data) -> AppData + Clone,
{
    type Coord = R::Coord;
    type Data = AppData;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.renderer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<AppData> {
        let data = self.renderer.render(viewport, pixel_rect, canvas_size);
        data.iter().map(&self.wrap_fn).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{
        app_data::TestImageData, point_compute::ImagePointComputer, points::Point,
        PixelRenderer,
    };

    struct DummyComputer;

    impl ImagePointComputer for DummyComputer {
        type Coord = f64;
        type Data = TestImageData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0))
        }

        fn compute(&self, _coord: Point<f64>) -> TestImageData {
            TestImageData::new(true, 5.0)
        }
    }

    #[test]
    fn test_app_data_renderer_wraps_data() {
        let pixel_renderer = PixelRenderer::new(DummyComputer);
        let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImage(*d));

        let viewport = Viewport::new(Point::new(5.0, 5.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(2, 2);
        let data = app_renderer.render(&viewport, pixel_rect, (2, 2));

        assert_eq!(data.len(), 4);
        // All wrapped in AppData::TestImage
        matches!(data[0], AppData::TestImage(_));
    }
}
```

**Step 2: Export from mod.rs**

In `src/rendering/mod.rs`:

```rust
mod app_data_renderer;
pub use app_data_renderer::AppDataRenderer;
```

**Step 3: Run tests**

Run: `cargo test --lib rendering::app_data_renderer`
Expected: PASS

**Step 4: Commit**

```bash
git add src/rendering/app_data_renderer.rs src/rendering/mod.rs
git commit -m "feat: add AppDataRenderer wrapper for Data→AppData conversion"
```

---

## Task 11: Update TestImageView to Use AppDataRenderer

**Files:**
- Modify: `src/components/test_image.rs:97-140`

**Step 1: Complete TestImageView with AppDataRenderer**

In `src/components/test_image.rs`:

```rust
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
    PixelRenderer, TestImageData, AppData, test_image_colorizer,
    TilingCanvasRenderer, Colorizer, AppDataRenderer,
};

#[component]
pub fn TestImageView() -> impl IntoView {
    let test_computer = TestImageRenderer::new();

    // Create renderer chain: TestImageRenderer → PixelRenderer → AppDataRenderer
    let pixel_renderer = PixelRenderer::new(test_computer.clone());
    let app_renderer = AppDataRenderer::new(
        pixel_renderer,
        |d: &TestImageData| AppData::TestImage(*d),
    );

    // Create TilingCanvasRenderer with colorizer
    let canvas_renderer = create_rw_signal(TilingCanvasRenderer::new(
        app_renderer,
        test_image_colorizer as Colorizer<AppData>,
        128,
    ));

    let (viewport, set_viewport) = create_signal(
        Viewport::new(test_computer.natural_bounds().center())
    );

    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

    let renderer_info = create_memo(move |_| {
        let mut info = test_computer.info(&viewport.get());
        info.render_time_ms = render_time_ms.get();
        info
    });

    // UI visibility
    let ui_visibility = use_ui_visibility();

    // Callbacks
    let natural_bounds = test_computer.natural_bounds();
    let on_home_click = move || {
        set_viewport.set(Viewport::new(natural_bounds.center()));
    };

    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="w-full h-full">
            // TODO: New InteractiveCanvas component
            <p>"Architecture updated - awaiting InteractiveCanvas"</p>
        </div>
        <UI
            info=renderer_info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
        />
    }
}
```

**Step 2: Compile check**

Run: `cargo check`
Expected: PASS (component structure complete, just missing InteractiveCanvas)

**Step 3: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: update TestImageView to use AppDataRenderer wrapper"
```

---

## Task 12: Create New InteractiveCanvas Component

**Files:**
- Modify: `src/components/interactive_canvas.rs` (replace entire file)

**Step 1: Replace InteractiveCanvas with new implementation**

Replace contents of `src/components/interactive_canvas.rs`:

```rust
use crate::hooks::use_canvas_interaction::use_canvas_interaction;
use crate::rendering::{
    points::Rect, viewport::Viewport, TilingCanvasRenderer, Renderer,
};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[component]
pub fn InteractiveCanvas<R>(
    canvas_renderer: RwSignal<TilingCanvasRenderer<R>>,
    viewport: ReadSignal<Viewport<R::Coord>>,
    set_viewport: WriteSignal<Viewport<R::Coord>>,
    set_render_time_ms: WriteSignal<Option<f64>>,
    natural_bounds: Rect<R::Coord>,
) -> impl IntoView
where
    R: Renderer + Clone + 'static,
    R::Coord: Clone + PartialEq + 'static,
    R::Coord: std::ops::Sub<Output = R::Coord>
        + std::ops::Add<Output = R::Coord>
        + std::ops::Mul<f64, Output = R::Coord>
        + std::ops::Div<f64, Output = R::Coord>
        + From<f64>,
{
    let canvas_ref = create_node_ref::<HtmlCanvasElement>();

    // Effect: Render when canvas_renderer OR viewport changes
    create_effect(move |_| {
        let vp = viewport.get();
        canvas_renderer.track();

        if let Some(canvas) = canvas_ref.get() {
            let start = web_sys::window()
                .unwrap()
                .performance()
                .unwrap()
                .now();

            canvas_renderer.with(|cr| cr.render(&vp, &canvas));

            let elapsed = web_sys::window()
                .unwrap()
                .performance()
                .unwrap()
                .now()
                - start;
            set_render_time_ms.set(Some(elapsed));
        }
    });

    // Canvas interaction hook
    let interaction = use_canvas_interaction(
        canvas_ref,
        viewport,
        set_viewport,
        natural_bounds,
    );

    view! {
        <canvas
            node_ref=canvas_ref
            class="w-full h-full"
            width="800"
            height="600"
            on:wheel=interaction.on_wheel
            on:mousedown=interaction.on_mousedown
            on:touchstart=interaction.on_touchstart
        />
    }
}
```

**Step 2: Compile check**

Run: `cargo check`
Expected: May fail due to use_canvas_interaction signature - check and adapt

**Step 3: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "feat: create new InteractiveCanvas component for new architecture"
```

---

## Task 13: Update TestImageView to Use New InteractiveCanvas

**Files:**
- Modify: `src/components/test_image.rs:97-150`

**Step 1: Wire up InteractiveCanvas in TestImageView**

In `src/components/test_image.rs`:

```rust
#[component]
pub fn TestImageView() -> impl IntoView {
    let test_computer = TestImageRenderer::new();

    // Create renderer chain
    let pixel_renderer = PixelRenderer::new(test_computer.clone());
    let app_renderer = AppDataRenderer::new(
        pixel_renderer,
        |d: &TestImageData| AppData::TestImage(*d),
    );

    let canvas_renderer = create_rw_signal(TilingCanvasRenderer::new(
        app_renderer,
        test_image_colorizer as Colorizer<AppData>,
        128,
    ));

    let natural_bounds = test_computer.natural_bounds();
    let (viewport, set_viewport) = create_signal(
        Viewport::new(natural_bounds.center())
    );

    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

    let renderer_info = create_memo(move |_| {
        let mut info = test_computer.info(&viewport.get());
        info.render_time_ms = render_time_ms.get();
        info
    });

    let ui_visibility = use_ui_visibility();

    let on_home_click = move || {
        set_viewport.set(Viewport::new(natural_bounds.center()));
    };

    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="w-full h-full">
            <InteractiveCanvas
                canvas_renderer=canvas_renderer
                viewport=viewport
                set_viewport=set_viewport
                set_render_time_ms=set_render_time_ms
                natural_bounds=natural_bounds
            />
        </div>
        <UI
            info=renderer_info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
        />
    }
}
```

**Step 2: Compile and fix any remaining issues**

Run: `cargo check`
Expected: Fix any trait bound or import issues

**Step 3: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: wire up new InteractiveCanvas in TestImageView"
```

---

## Task 14: Test in Browser

**Files:**
- No code changes

**Step 1: Build and run**

Run: `trunk serve`
Expected: Dev server starts

**Step 2: Manual browser test**

Open: http://localhost:8080
Expected:
- Test image renders (checkerboard, circles, green line)
- Pan/zoom works
- UI shows correct info
- No console errors

**Step 3: Performance check**

Test colorizer change (when implemented in UI):
Expected: Recolor happens instantly (< 50ms)

**Step 4: Document results**

Note any issues found for fixing in next task.

---

## Task 15: Cleanup and Final Testing

**Files:**
- Run linting and formatting

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: format and lint new architecture"
```

---

## Success Criteria Checklist

After completing all tasks, verify:

- [ ] Test image renders correctly via new architecture
- [ ] ImagePointComputer trait has `type Data` (not RGBA)
- [ ] Renderer trait has `type Data` (not Vec<u8>)
- [ ] PixelRenderer works with generic Data
- [ ] AppData enum exists with TestImage variant
- [ ] test_image_colorizer function converts AppData → RGBA
- [ ] TilingCanvasRenderer implements caching and progressive rendering
- [ ] TilingCanvasRenderer.with_colorizer() preserves cache via Arc
- [ ] AppDataRenderer wrapper works correctly
- [ ] InteractiveCanvas is generic over Renderer
- [ ] TestImageView uses new architecture
- [ ] Pan/zoom interaction works
- [ ] UI displays correct information
- [ ] No regression in render performance
- [ ] All tests pass
- [ ] No Clippy warnings

---

## Next Steps (Future Work)

After this plan is complete:

1. **Add colorizer selection to UI** - Dropdown to swap colorizers
2. **Add renderer selection to UI** - Dropdown to swap renderers (future: Mandelbrot)
3. **Implement MandelbrotComputer** - With MandelbrotData
4. **Add multiple colorizers** - Distance estimate, z-max, etc.
5. **Parallelize tile computation** - Web workers or rayon-wasm
6. **Smart re-rendering** - Only recompute changed tiles on pan

---

## References

- Design doc: `docs/plans/2025-10-28-unified-architecture-design.md`
- Original thoughts: `docs/architecture-redesign.md`
- Previous design: `docs/plans/2025-10-26-separate-computation-coloring-design.md`
