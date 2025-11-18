# Separate Computation from Coloring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Decouple fractal computation from color generation, enabling instant recoloring without expensive recomputation.

**Architecture:** Add generic `Data` type to `ImagePointComputer` and `Renderer` traits, create `CanvasRenderCoordinator` for progressive rendering with data caching, update `InteractiveCanvas` to use render callbacks.

**Tech Stack:** Rust, Leptos, web-sys (Canvas API)

---

## Task 1: Add Data Type to ImagePointComputer Trait

**Files:**
- Modify: `src/rendering/point_compute.rs:7-22`

**Step 1: Write failing test for new Data-based trait**

Add to `src/rendering/point_compute.rs` after existing tests:

```rust
#[test]
fn test_image_point_computer_with_data_type() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestData {
        value: u8,
    }

    struct DataComputer;

    impl ImagePointComputer for DataComputer {
        type Coord = f64;
        type Data = TestData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Point<f64>) -> Self::Data {
            TestData { value: 42 }
        }
    }

    let computer = DataComputer;
    let result = computer.compute(Point::new(50.0, 50.0));
    assert_eq!(result, TestData { value: 42 });
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_image_point_computer_with_data_type --lib`
Expected: Compilation error - "no associated type `Data` in trait `ImagePointComputer`"

**Step 3: Update ImagePointComputer trait**

Replace `src/rendering/point_compute.rs:7-22` with:

```rust
/// Trait for computing data values at points in image space
///
/// This is the lowest-level rendering abstraction - pure computation with no loops.
/// Returns generic Data (not RGBA colors) to separate computation from visualization.
/// Typically wrapped by PixelRenderer which adds the pixel iteration logic.
pub trait ImagePointComputer {
    /// Coordinate type for image space
    type Coord;

    /// Data type produced by computation (e.g., MandelbrotData, TestImageData)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Compute data for a single point in image space
    ///
    /// # Arguments
    /// * `coord` - Point in image-space coordinates
    ///
    /// # Returns
    /// Data value (not RGBA - colorization happens separately)
    fn compute(&self, coord: Point<Self::Coord>) -> Self::Data;
}
```

**Step 4: Update existing test to use Data type**

Replace `src/rendering/point_compute.rs:29-43` with:

```rust
// Simple test implementation
struct SolidColorCompute {
    color: (u8, u8, u8, u8),
}

impl ImagePointComputer for SolidColorCompute {
    type Coord = f64;
    type Data = (u8, u8, u8, u8);  // Data is RGBA tuple for this simple case

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
    }

    fn compute(&self, _coord: Point<f64>) -> Self::Data {
        self.color
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --lib point_compute`
Expected: Both tests pass

**Step 6: Commit**

```bash
git add src/rendering/point_compute.rs
git commit -m "feat: add Data associated type to ImagePointComputer trait

Separates computation from colorization by making ImagePointComputer
return generic Data instead of RGBA tuples."
```

---

## Task 2: Add Data Type to Renderer Trait

**Files:**
- Modify: `src/rendering/renderer_trait.rs:6-28`
- Modify: `src/rendering/pixel_renderer.rs` (entire file)

**Step 1: Update Renderer trait**

Replace `src/rendering/renderer_trait.rs:6-28` with:

```rust
/// Core trait for rendering data given viewport and pixel-space dimensions
///
/// Implementations can be composed (e.g., TiledRenderer wrapping PixelRenderer)
/// Returns generic Data (not RGBA) to separate computation from colorization.
pub trait Renderer {
    /// Coordinate type for image space (f64, rug::Float, etc.)
    type Coord;

    /// Data type produced by rendering (matches ImagePointComputer::Data)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Render data for a given viewport and pixel rectangle
    ///
    /// # Arguments
    /// * `viewport` - What image coordinates the full canvas shows
    /// * `pixel_rect` - Which portion of canvas to render (for tiling)
    /// * `canvas_size` - Full canvas dimensions (width, height)
    ///
    /// # Returns
    /// Data values for the specified pixel_rect (length = width * height)
    /// Note: Returns Data, not RGBA bytes - colorization happens separately
    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data>;
}
```

**Step 2: Update PixelRenderer to use Data**

Read current `src/rendering/pixel_renderer.rs` to see the implementation, then replace with:

```rust
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::Rect,
    renderer_trait::Renderer,
    transforms::pixel_to_image,
    viewport::Viewport,
    PixelRect,
};
use std::ops::{Add, Div, Mul, Sub};

/// Wraps an ImagePointComputer and adds pixel iteration logic
///
/// Converts pixel coordinates to image coordinates and calls compute() for each pixel.
/// Returns Data values (not RGBA) - colorization is handled separately.
pub struct PixelRenderer<C: ImagePointComputer> {
    computer: C,
}

impl<C: ImagePointComputer> PixelRenderer<C> {
    pub fn new(computer: C) -> Self {
        Self { computer }
    }
}

impl<C> Renderer for PixelRenderer<C>
where
    C: ImagePointComputer,
    C::Coord: Clone
        + Sub<Output = C::Coord>
        + Add<Output = C::Coord>
        + Mul<f64, Output = C::Coord>
        + Div<f64, Output = C::Coord>,
{
    type Coord = C::Coord;
    type Data = C::Data;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.computer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data> {
        let target_rect = crate::rendering::transforms::calculate_visible_bounds(
            viewport,
            &self.natural_bounds(),
            canvas_size.0,
            canvas_size.1,
        );

        let mut data = Vec::with_capacity((pixel_rect.width * pixel_rect.height) as usize);

        for py in pixel_rect.y..(pixel_rect.y + pixel_rect.height) {
            for px in pixel_rect.x..(pixel_rect.x + pixel_rect.width) {
                let image_coord = pixel_to_image(
                    px as f64 + 0.5,
                    py as f64 + 0.5,
                    &target_rect,
                    canvas_size.0,
                    canvas_size.1,
                );

                let point_data = self.computer.compute(image_coord);
                data.push(point_data);
            }
        }

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::points::Point;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestData {
        x: f64,
        y: f64,
    }

    struct CoordEchoComputer;

    impl ImagePointComputer for CoordEchoComputer {
        type Coord = f64;
        type Data = TestData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, coord: Point<f64>) -> Self::Data {
            TestData {
                x: *coord.x(),
                y: *coord.y(),
            }
        }
    }

    #[test]
    fn test_pixel_renderer_returns_data() {
        let computer = CoordEchoComputer;
        let renderer = PixelRenderer::new(computer);

        let viewport = Viewport::new(Point::new(50.0, 50.0), 1.0);
        let pixel_rect = PixelRect::new(0, 0, 2, 2);
        let canvas_size = (2, 2);

        let data = renderer.render(&viewport, pixel_rect, canvas_size);

        assert_eq!(data.len(), 4); // 2x2 pixels
        // Data should contain coordinate information (not RGBA)
        for datum in data {
            assert!(datum.x >= 0.0 && datum.x <= 100.0);
            assert!(datum.y >= 0.0 && datum.y <= 100.0);
        }
    }
}
```

**Step 3: Run tests to verify they pass**

Run: `cargo test --lib pixel_renderer`
Expected: Test passes

**Step 4: Check for compilation errors**

Run: `cargo check --workspace`
Expected: Compilation errors in files using Renderer trait (TiledRenderer, InteractiveCanvas, TestImageRenderer)

**Step 5: Commit**

```bash
git add src/rendering/renderer_trait.rs src/rendering/pixel_renderer.rs
git commit -m "feat: add Data type to Renderer trait and update PixelRenderer

PixelRenderer now returns Vec<Data> instead of Vec<u8> (RGBA bytes).
Computation is now separated from colorization."
```

---

## Task 3: Create Colorizer Module

**Files:**
- Create: `src/rendering/colorizer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write failing test for colorizer**

Create `src/rendering/colorizer.rs`:

```rust
/// Colorizer function type - converts Data to RGBA
///
/// This is where computation (Data) gets converted to colors for display.
/// Simple function type allows easy swapping of color schemes.
pub type Colorizer<D> = fn(&D) -> (u8, u8, u8, u8);

/// Helper to colorize a slice of data into RGBA bytes
///
/// # Arguments
/// * `data` - Slice of Data values to colorize
/// * `colorizer` - Colorizer function to apply
///
/// # Returns
/// RGBA byte array (length = data.len() * 4)
pub fn colorize_data<D>(data: &[D], colorizer: Colorizer<D>) -> Vec<u8> {
    data.iter()
        .flat_map(|d| {
            let (r, g, b, a) = colorizer(d);
            [r, g, b, a]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestData {
        intensity: u8,
    }

    fn test_colorizer(data: &TestData) -> (u8, u8, u8, u8) {
        (data.intensity, 0, 0, 255)
    }

    #[test]
    fn test_colorize_data_converts_to_rgba() {
        let data = vec![
            TestData { intensity: 100 },
            TestData { intensity: 200 },
        ];

        let rgba = colorize_data(&data, test_colorizer);

        assert_eq!(rgba.len(), 8); // 2 pixels * 4 bytes
        assert_eq!(rgba[0..4], [100, 0, 0, 255]);
        assert_eq!(rgba[4..8], [200, 0, 0, 255]);
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --lib colorizer`
Expected: Test passes

**Step 3: Add colorizer to rendering module exports**

Add to `src/rendering/mod.rs` (after other module declarations):

```rust
pub mod colorizer;
pub use colorizer::{colorize_data, Colorizer};
```

**Step 4: Run tests**

Run: `cargo test --lib rendering`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/rendering/colorizer.rs src/rendering/mod.rs
git commit -m "feat: add Colorizer module for Data to RGBA conversion

Provides Colorizer function type and colorize_data helper.
Enables separation of computation from color generation."
```

---

## Task 4: Update TestImageRenderer to Use Data Type

**Files:**
- Modify: `src/components/test_image.rs:14-68`

**Step 1: Define TestImageData type**

Add after imports in `src/components/test_image.rs`:

```rust
/// Data produced by test image computation
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
    pub on_center_line: bool,
}
```

**Step 2: Update TestImageRenderer::compute to return Data**

Replace `TestImageRenderer::compute_point_color` method (lines 30-55) with:

```rust
fn compute_point_data(&self, x: f64, y: f64) -> TestImageData {
    // Check if on center line
    let on_center_line = x.abs() < self.circle_line_thickness;

    // Check circle distance
    let distance = (x * x + y * y).sqrt();
    let nearest_ring = (distance / self.circle_radius_step).round();
    let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();
    let circle_distance = if nearest_ring > 0.0 { ring_distance } else { f64::MAX };

    // Checkerboard calculation
    let square_x = (x / self.checkerboard_size).floor() as i32;
    let square_y = (y / self.checkerboard_size).floor() as i32;
    let checkerboard = (square_x + square_y) % 2 == 0;

    TestImageData {
        checkerboard,
        circle_distance,
        on_center_line,
    }
}
```

**Step 3: Update ImagePointComputer implementation**

Replace implementation (lines 58-68) with:

```rust
impl ImagePointComputer for TestImageRenderer {
    type Coord = f64;
    type Data = TestImageData;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0))
    }

    fn compute(&self, coord: Point<f64>) -> Self::Data {
        self.compute_point_data(*coord.x(), *coord.y())
    }
}
```

**Step 4: Add colorizer function**

Add after `ImagePointComputer` implementation:

```rust
/// Colorizer for TestImageData
pub fn test_image_colorizer(data: &TestImageData) -> (u8, u8, u8, u8) {
    // Center line is bright green
    if data.on_center_line {
        return (0, 255, 0, 255);
    }

    // Circle is red
    if data.circle_distance < 0.05 {
        return (255, 0, 0, 255);
    }

    // Checkerboard background
    if data.checkerboard {
        (255, 255, 255, 255) // White
    } else {
        (204, 204, 204, 255) // Light grey
    }
}
```

**Step 5: Check compilation**

Run: `cargo check --workspace`
Expected: Compilation errors in TestImageView component (InteractiveCanvas needs updating)

**Step 6: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: update TestImageRenderer to return TestImageData

Separates computation (TestImageData) from colorization
(test_image_colorizer function). Computation returns structured
data about checkerboard, circles, and center line."
```

---

## Task 5: Remove TiledRenderer (Functionality Will Move to Coordinator)

**Files:**
- Delete: `src/rendering/tiled_renderer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Check if TiledRenderer is used anywhere**

Run: `git grep -n "TiledRenderer" src/`
Expected: Find usages that need to be removed

**Step 2: Remove TiledRenderer from mod.rs**

Remove these lines from `src/rendering/mod.rs`:
```rust
mod tiled_renderer;
pub use tiled_renderer::TiledRenderer;
```

**Step 3: Delete TiledRenderer file**

```bash
git rm src/rendering/tiled_renderer.rs
```

**Step 4: Check compilation**

Run: `cargo check --workspace`
Expected: Compilation errors where TiledRenderer was used (we'll fix in next tasks)

**Step 5: Commit**

```bash
git commit -m "refactor: remove TiledRenderer

Tiling functionality will be handled by CanvasRenderCoordinator.
TiledRenderer no longer needed in new architecture."
```

---

## Task 6: Create CanvasRenderCoordinator

**Files:**
- Create: `src/rendering/canvas_render_coordinator.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create CanvasRenderCoordinator skeleton with test**

Create `src/rendering/canvas_render_coordinator.rs`:

```rust
use crate::rendering::{
    colorizer::{colorize_data, Colorizer},
    point_compute::ImagePointComputer,
    points::Rect,
    viewport::Viewport,
    PixelRect, PixelRenderer,
};
use web_sys::ImageData;

/// Coordinates rendering: tiling, caching, progressive display, recoloring
///
/// Central orchestrator that:
/// - Splits canvas into tiles for progressive rendering
/// - Caches computed Data for instant recoloring
/// - Writes RGBA directly to ImageData
/// - Decides when to compute vs recolorize
pub struct CanvasRenderCoordinator<T, C>
where
    C: ImagePointComputer<Coord = T>,
{
    renderer: PixelRenderer<C>,
    colorizer: Colorizer<C::Data>,
    tile_size: u32,

    // Cached state
    cached_viewport: Option<Viewport<T>>,
    cached_canvas_size: Option<(u32, u32)>,
    cached_data: Vec<C::Data>,
}

impl<T, C> CanvasRenderCoordinator<T, C>
where
    C: ImagePointComputer<Coord = T>,
    T: Clone + PartialEq,
    C::Data: Default,
{
    pub fn new(renderer: PixelRenderer<C>, colorizer: Colorizer<C::Data>) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size: 128,
            cached_viewport: None,
            cached_canvas_size: None,
            cached_data: Vec::new(),
        }
    }

    pub fn set_tile_size(&mut self, tile_size: u32) {
        self.tile_size = tile_size;
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<C::Data>) {
        self.colorizer = colorizer;
    }

    pub fn set_computer(&mut self, renderer: PixelRenderer<C>) {
        self.renderer = renderer;
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.cached_viewport = None;
        self.cached_canvas_size = None;
        self.cached_data.clear();
    }

    fn has_cached_data(&self) -> bool {
        !self.cached_data.is_empty() && self.cached_viewport.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::points::Point;

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct TestData {
        value: u8,
    }

    struct TestComputer;

    impl ImagePointComputer for TestComputer {
        type Coord = f64;
        type Data = TestData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Point<f64>) -> Self::Data {
            TestData { value: 42 }
        }
    }

    fn test_colorizer(data: &TestData) -> (u8, u8, u8, u8) {
        (data.value, 0, 0, 255)
    }

    #[test]
    fn test_coordinator_creation() {
        let renderer = PixelRenderer::new(TestComputer);
        let coordinator = CanvasRenderCoordinator::new(renderer, test_colorizer);

        assert_eq!(coordinator.tile_size, 128);
        assert!(!coordinator.has_cached_data());
    }

    #[test]
    fn test_invalidate_cache() {
        let renderer = PixelRenderer::new(TestComputer);
        let mut coordinator = CanvasRenderCoordinator::new(renderer, test_colorizer);

        // Simulate having cached data
        coordinator.cached_data.push(TestData { value: 1 });
        coordinator.cached_viewport = Some(Viewport::new(Point::new(0.0, 0.0), 1.0));
        assert!(coordinator.has_cached_data());

        // Invalidate
        coordinator.invalidate_cache();
        assert!(!coordinator.has_cached_data());
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib canvas_render_coordinator`
Expected: Tests pass

**Step 3: Add to mod.rs**

Add to `src/rendering/mod.rs`:

```rust
mod canvas_render_coordinator;
pub use canvas_render_coordinator::CanvasRenderCoordinator;
```

**Step 4: Run tests**

Run: `cargo test --lib rendering`
Expected: All rendering tests pass

**Step 5: Commit**

```bash
git add src/rendering/canvas_render_coordinator.rs src/rendering/mod.rs
git commit -m "feat: add CanvasRenderCoordinator skeleton

Basic structure for coordinator with caching and invalidation.
Rendering logic to be added in next task."
```

---

## Task 7: Implement Progressive Rendering in Coordinator

**Files:**
- Modify: `src/rendering/canvas_render_coordinator.rs`

**Step 1: Add render method**

Add to `impl` block in `src/rendering/canvas_render_coordinator.rs`:

```rust
/// Main rendering entry point
///
/// Decides whether to compute or reuse cached data based on viewport/size changes.
pub fn render(&mut self, viewport: &Viewport<T>, canvas_size: (u32, u32), image_data: &mut ImageData) {
    let viewport_changed = self.cached_viewport.as_ref() != Some(viewport);
    let size_changed = self.cached_canvas_size != Some(canvas_size);

    if viewport_changed || size_changed {
        self.render_with_computation(viewport, canvas_size, image_data);
    }
    // If neither changed, data already rendered and displayed
}

/// Render with full computation (progressive tile-by-tile)
fn render_with_computation(&mut self, viewport: &Viewport<T>, canvas_size: (u32, u32), image_data: &mut ImageData) {
    let (width, height) = canvas_size;

    // Prepare storage
    self.cached_data.clear();
    self.cached_data
        .resize((width * height) as usize, C::Data::default());

    // Compute tiles progressively
    for tile_rect in self.compute_tiles(canvas_size) {
        // Compute tile data
        let tile_data = self.renderer.render(viewport, tile_rect, canvas_size);

        // Store in cache
        self.store_tile(tile_rect, &tile_data, width);

        // Colorize tile
        let rgba = colorize_data(&tile_data, self.colorizer);

        // Display immediately (progressive!)
        self.put_tile_to_image_data(image_data, tile_rect, &rgba, width);
    }

    // Update cache keys
    self.cached_viewport = Some(viewport.clone());
    self.cached_canvas_size = Some(canvas_size);
}

/// Recolorize from cached data
pub fn recolorize(&self, image_data: &mut ImageData) {
    if !self.has_cached_data() {
        return;
    }

    let (width, height) = self.cached_canvas_size.unwrap();

    // Colorize all cached data
    let rgba = colorize_data(&self.cached_data, self.colorizer);

    // Display full rect
    let full_rect = PixelRect::new(0, 0, width, height);
    self.put_tile_to_image_data(image_data, full_rect, &rgba, width);
}

/// Generate tile rectangles covering canvas
fn compute_tiles(&self, canvas_size: (u32, u32)) -> Vec<PixelRect> {
    let (width, height) = canvas_size;
    let mut tiles = Vec::new();

    for y in (0..height).step_by(self.tile_size as usize) {
        for x in (0..width).step_by(self.tile_size as usize) {
            let tile_width = self.tile_size.min(width - x);
            let tile_height = self.tile_size.min(height - y);
            tiles.push(PixelRect::new(x, y, tile_width, tile_height));
        }
    }

    tiles
}

/// Store tile data into cached grid
fn store_tile(&mut self, tile_rect: PixelRect, tile_data: &[C::Data], canvas_width: u32) {
    for tile_y in 0..tile_rect.height {
        let canvas_y = tile_rect.y + tile_y;
        let canvas_x = tile_rect.x;
        let cache_offset = (canvas_y * canvas_width + canvas_x) as usize;

        let tile_row_start = (tile_y * tile_rect.width) as usize;
        let tile_row_end = tile_row_start + tile_rect.width as usize;
        let tile_row = &tile_data[tile_row_start..tile_row_end];

        self.cached_data[cache_offset..(cache_offset + tile_rect.width as usize)]
            .clone_from_slice(tile_row);
    }
}

/// Write RGBA bytes to ImageData at tile position
fn put_tile_to_image_data(&self, image_data: &mut ImageData, tile_rect: PixelRect, rgba: &[u8], canvas_width: u32) {
    let data = image_data.data();
    let mut data_vec = data.to_vec();

    for tile_y in 0..tile_rect.height {
        let canvas_y = tile_rect.y + tile_y;
        let canvas_x = tile_rect.x;
        let data_offset = ((canvas_y * canvas_width + canvas_x) * 4) as usize;

        let rgba_row_start = (tile_y * tile_rect.width * 4) as usize;
        let rgba_row_end = rgba_row_start + (tile_rect.width * 4) as usize;
        let rgba_row = &rgba[rgba_row_start..rgba_row_end];

        data_vec[data_offset..(data_offset + rgba_row.len())].copy_from_slice(rgba_row);
    }

    // Write back to ImageData
    image_data.data_mut().copy_from_slice(&data_vec);
}
```

**Step 2: Add test for rendering flow**

Add to test module:

```rust
#[test]
fn test_render_creates_tiles() {
    let renderer = PixelRenderer::new(TestComputer);
    let coordinator = CanvasRenderCoordinator::new(renderer, test_colorizer);

    let tiles = coordinator.compute_tiles((300, 200));

    // 300/128 = 3 columns (128, 128, 44)
    // 200/128 = 2 rows (128, 72)
    // Total: 6 tiles
    assert_eq!(tiles.len(), 6);

    // Check first tile
    assert_eq!(tiles[0].x, 0);
    assert_eq!(tiles[0].y, 0);
    assert_eq!(tiles[0].width, 128);
    assert_eq!(tiles[0].height, 128);

    // Check last tile (partial)
    let last = &tiles[5];
    assert_eq!(last.x, 256);
    assert_eq!(last.y, 128);
    assert_eq!(last.width, 44);
    assert_eq!(last.height, 72);
}
```

**Step 3: Run tests**

Run: `cargo test --lib canvas_render_coordinator`
Expected: Tests pass

**Step 4: Commit**

```bash
git add src/rendering/canvas_render_coordinator.rs
git commit -m "feat: implement progressive rendering in CanvasRenderCoordinator

Tiles computed progressively with immediate display. Caches data
for instant recoloring. Handles viewport/size change detection."
```

---

## Task 8: Update InteractiveCanvas to Use Generic Rendering Callback

**Files:**
- Modify: `src/components/interactive_canvas.rs`

This task is complex - we need to see the current implementation first to understand what needs changing.

**Step 1: Read current InteractiveCanvas**

Run: `cat src/components/interactive_canvas.rs`

**Step 2: Update InteractiveCanvas signature**

Replace the component signature to accept a render callback instead of being generic over Renderer. The new signature should:
- Accept `viewport` and `set_viewport` signals
- Accept `natural_bounds` for zoom/pan calculations
- Accept `render_trigger` signal to force re-renders
- Accept `on_render` callback: `Fn(&Viewport<T>, (u32, u32), &mut ImageData)`

**Step 3: Update render effect**

The effect should:
- Track both `viewport` and `render_trigger` signals
- Get ImageData from canvas
- Call `on_render` callback with viewport, canvas_size, and image_data
- Put ImageData back to canvas

**Step 4: Update interaction handlers**

Keep zoom/pan handlers but use `natural_bounds` prop instead of calling `renderer.natural_bounds()`

**Step 5: Remove RendererInfo dependencies**

InteractiveCanvas should no longer handle renderer info display - that moves to App level.

**Step 6: Test compilation**

Run: `cargo check --workspace`

**Step 7: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "refactor: make InteractiveCanvas use generic render callback

InteractiveCanvas now accepts on_render callback instead of being
generic over Renderer. Enables flexible rendering strategies."
```

---

## Task 9: Update TestImageView to Use New Architecture

**Files:**
- Modify: `src/components/test_image.rs:97-end`

**Step 1: Update TestImageView component**

Replace the component implementation with:

```rust
#[component]
pub fn TestImageView() -> impl IntoView {
    use crate::rendering::{CanvasRenderCoordinator, PixelRenderer, Viewport};
    use crate::rendering::points::Point;
    use leptos::*;

    // Computer and colorizer
    let computer = TestImageRenderer::new();
    let renderer = PixelRenderer::new(computer.clone());
    let natural_bounds = computer.natural_bounds();

    // Viewport state
    let (viewport, set_viewport) = create_signal(Viewport::new(Point::new(0.0, 0.0), 1.0));

    // Render trigger (for future colorizer changes)
    let (render_trigger, _set_render_trigger) = create_signal(0u32);

    // Coordinator
    let coordinator = create_rw_signal(CanvasRenderCoordinator::new(renderer, test_image_colorizer));

    // Render callback
    let on_render = move |vp: &Viewport<f64>, canvas_size: (u32, u32), image_data: &mut web_sys::ImageData| {
        coordinator.update(|coord| {
            coord.render(vp, canvas_size, image_data);
        });
    };

    // UI visibility
    let (ui_visible, toggle_ui) = use_ui_visibility();

    let toggle_ui_action = move |_| toggle_ui();
    let toggle_fullscreen_action = move |_| toggle_fullscreen();

    let canvas_with_info = InteractiveCanvas(
        viewport,
        set_viewport,
        natural_bounds,
        render_trigger,
        Box::new(on_render),
    );

    view! {
        <div class="relative w-screen h-screen">
            {canvas_with_info.canvas}
            <Show when=move || ui_visible.get()>
                <UI
                    info=canvas_with_info.info
                    on_fullscreen_toggle=toggle_fullscreen_action
                    on_ui_toggle=toggle_ui_action
                />
            </Show>
        </div>
    }
}
```

**Step 2: Test compilation**

Run: `cargo check --workspace`
Expected: Should compile successfully

**Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 4: Test in browser**

Start dev server (if not running): `trunk serve`
Visit: http://localhost:8080
Expected: Test image renders correctly with checkerboard, circles, and center line

**Step 5: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: update TestImageView to use CanvasRenderCoordinator

TestImageView now uses new architecture with Data-based rendering
and CanvasRenderCoordinator for progressive display."
```

---

## Task 10: Add Colorizer Switching Example

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Add alternative colorizer**

Add after `test_image_colorizer`:

```rust
/// Alternative colorizer showing only circles
pub fn circles_only_colorizer(data: &TestImageData) -> (u8, u8, u8, u8) {
    if data.circle_distance < 0.05 {
        (255, 255, 0, 255) // Yellow circles
    } else {
        (0, 0, 0, 255) // Black background
    }
}

/// Alternative colorizer showing only checkerboard
pub fn checkerboard_only_colorizer(data: &TestImageData) -> (u8, u8, u8, u8) {
    if data.checkerboard {
        (100, 100, 255, 255) // Blue
    } else {
        (255, 100, 100, 255) // Red
    }
}
```

**Step 2: Add colorizer selection UI**

Update TestImageView component to include colorizer switching:

```rust
#[component]
pub fn TestImageView() -> impl IntoView {
    // ... existing setup ...

    // Colorizer selection
    let (colorizer_mode, set_colorizer_mode) = create_signal(0u32);

    // Update coordinator when colorizer changes
    create_effect(move |_| {
        let mode = colorizer_mode.get();
        let new_colorizer = match mode {
            0 => test_image_colorizer as fn(&TestImageData) -> (u8, u8, u8, u8),
            1 => circles_only_colorizer,
            2 => checkerboard_only_colorizer,
            _ => test_image_colorizer,
        };

        coordinator.update(|coord| {
            coord.set_colorizer(new_colorizer);
        });

        // Trigger recolorization
        _set_render_trigger.update(|n| *n += 1);
    });

    // ... existing render callback ...

    view! {
        <div class="relative w-screen h-screen">
            {canvas_with_info.canvas}
            <Show when=move || ui_visible.get()>
                <div class="absolute top-4 left-4 bg-black/80 text-white p-4 rounded">
                    <div class="mb-2">
                        <button
                            class="px-3 py-1 bg-blue-500 rounded mr-2"
                            class:bg-blue-700=move || colorizer_mode.get() == 0
                            on:click=move |_| set_colorizer_mode.set(0)
                        >
                            "Standard"
                        </button>
                        <button
                            class="px-3 py-1 bg-blue-500 rounded mr-2"
                            class:bg-blue-700=move || colorizer_mode.get() == 1
                            on:click=move |_| set_colorizer_mode.set(1)
                        >
                            "Circles Only"
                        </button>
                        <button
                            class="px-3 py-1 bg-blue-500 rounded"
                            class:bg-blue-700=move || colorizer_mode.get() == 2
                            on:click=move |_| set_colorizer_mode.set(2)
                        >
                            "Checkerboard Only"
                        </button>
                    </div>
                </div>
                <UI
                    info=canvas_with_info.info
                    on_fullscreen_toggle=toggle_fullscreen_action
                    on_ui_toggle=toggle_ui_action
                />
            </Show>
        </div>
    }
}
```

**Step 3: Test in browser**

Visit: http://localhost:8080
Test:
1. Click "Circles Only" - should instantly recolor to show yellow circles on black
2. Click "Checkerboard Only" - should instantly show blue/red checkerboard
3. Click "Standard" - should return to original colors
Expected: Recoloring is instant (< 50ms), no recomputation

**Step 4: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: add colorizer switching UI to TestImageView

Demonstrates instant recoloring without recomputation.
Three colorizer modes: Standard, Circles Only, Checkerboard Only."
```

---

## Task 11: Run Full Test Suite and Fix Issues

**Files:**
- Various (as needed)

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors
If errors: Fix them and commit

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass
If failures: Fix them and commit

**Step 4: Check compilation**

Run: `cargo check --workspace --all-targets --all-features`
Expected: Clean compilation

**Step 5: Final browser test**

Visit: http://localhost:8080
Test all interactions:
- Zoom in/out
- Pan
- Colorizer switching
- Verify progressive rendering (tiles appear)

**Step 6: Commit if fixes were needed**

```bash
git add .
git commit -m "fix: address clippy warnings and test failures"
```

---

## Task 12: Update Documentation

**Files:**
- Modify: `RENDERER-ARCHITECTURE.md`

**Step 1: Update architecture diagram**

Update the mermaid diagram to show:
- `ImagePointComputer` returns `Data` (not RGBA)
- `Renderer` returns `Vec<Data>` (not `Vec<u8>`)
- New `Colorizer` type
- `CanvasRenderCoordinator` (replaces `TiledRenderer`)

**Step 2: Update trait documentation**

Update trait code examples to show `type Data` associated type.

**Step 3: Add Colorizer section**

Add documentation explaining:
- Colorizer function type
- How colorization is separated from computation
- Example colorizers

**Step 4: Add CanvasRenderCoordinator section**

Document:
- Purpose (tiling, caching, progressive rendering)
- Usage example
- How it handles recoloring

**Step 5: Update "Adding New Renderers" section**

Show how to define `Data` types and colorizers.

**Step 6: Commit**

```bash
git add RENDERER-ARCHITECTURE.md
git commit -m "docs: update architecture docs for Data-based rendering

Documents separation of computation from colorization,
CanvasRenderCoordinator, and Colorizer functions."
```

---

## Completion Checklist

- [ ] All tests pass
- [ ] Clippy clean
- [ ] Browser testing successful
- [ ] Colorizer switching works instantly
- [ ] Progressive rendering visible
- [ ] Documentation updated
- [ ] All tasks committed

## Future Enhancements (Not in This Plan)

- Parallel tile computation (web workers)
- Smart re-rendering (only changed tiles on pan)
- RGBA caching (avoid recolorization if colorizer unchanged)
- IndexedDB persistence
- Mandelbrot implementation with new architecture
