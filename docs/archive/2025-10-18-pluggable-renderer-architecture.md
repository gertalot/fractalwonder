# Pluggable Renderer Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Create a type-safe, pluggable renderer architecture that prevents coordinate space confusion and supports both simple test images (f64) and extreme-precision fractals (rug::Float).

**Architecture:** Generic coordinate newtypes (PixelCoord, ImageCoord<T>) with private fields enforce compile-time separation between pixel and image space. CanvasRenderer trait with associated type allows renderers to declare their precision requirements. Component-level switching enables dynamic renderer selection while preserving full type safety.

**Tech Stack:** Rust, Leptos, WASM, generic traits with associated types

---

## Task 1: Create Core Coordinate Types Module

**Files:**

- Create: `src/rendering/mod.rs`
- Create: `src/rendering/coords.rs`

**Step 1: Write failing test for PixelCoord construction and accessors**

Create `src/rendering/coords.rs`:

```rust

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelCoord {
  x: f64,
  y: f64,
}

impl PixelCoord {
  pub fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }

  pub fn x(&self) -> f64 {
    self.x
  }

  pub fn y(&self) -> f64 {
    self.y
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pixel_coord_construction() {
    let coord = PixelCoord::new(10.0, 20.0);
    assert_eq!(coord.x(), 10.0);
    assert_eq!(coord.y(), 20.0);
  }

  #[test]
  fn test_pixel_coord_equality() {
    let coord1 = PixelCoord::new(10.0, 20.0);
    let coord2 = PixelCoord::new(10.0, 20.0);
    let coord3 = PixelCoord::new(10.0, 21.0);
    assert_eq!(coord1, coord2);
    assert_ne!(coord1, coord3);
  }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test rendering::coords::tests`

Expected: PASS (2 tests)

**Step 3: Add ImageCoord<T> generic type with tests**

Add to `src/rendering/coords.rs` after PixelCoord:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ImageCoord<T> {
  x: T,
  y: T,
}

impl<T> ImageCoord<T> {
  pub fn new(x: T, y: T) -> Self {
    Self { x, y }
  }

  pub fn x(&self) -> &T {
    &self.x
  }

  pub fn y(&self) -> &T {
    &self.y
  }
}
```

Add to tests module:

```rust
#[test]
fn test_image_coord_f64() {
  let coord = ImageCoord::new(10.5, 20.5);
  assert_eq!(*coord.x(), 10.5);
  assert_eq!(*coord.y(), 20.5);
}

#[test]
fn test_image_coord_generic() {
  let coord_f64 = ImageCoord::new(10.5, 20.5);
  let coord_i32 = ImageCoord::new(10, 20);
  assert_eq!(*coord_f64.x(), 10.5);
  assert_eq!(*coord_i32.x(), 10);
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test rendering::coords::tests`

Expected: PASS (4 tests)

**Step 5: Add ImageRect<T> type with tests**

Add to `src/rendering/coords.rs` after ImageCoord:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ImageRect<T> {
  pub min: ImageCoord<T>,
  pub max: ImageCoord<T>,
}

impl<T: Clone> ImageRect<T> {
  pub fn new(min: ImageCoord<T>, max: ImageCoord<T>) -> Self {
    Self { min, max }
  }

  pub fn width(&self) -> T
  where
    T: std::ops::Sub<Output = T>,
  {
    self.max.x().clone() - self.min.x().clone()
  }

  pub fn height(&self) -> T
  where
    T: std::ops::Sub<Output = T>,
  {
    self.max.y().clone() - self.min.y().clone()
  }
}
```

Add to tests module:

```rust
#[test]
fn test_image_rect_dimensions() {
  let rect = ImageRect::new(ImageCoord::new(0.0, 0.0), ImageCoord::new(100.0, 50.0));
  assert_eq!(rect.width(), 100.0);
  assert_eq!(rect.height(), 50.0);
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test rendering::coords::tests`

Expected: PASS (5 tests)

**Step 7: Create rendering module file**

Create `src/rendering/mod.rs`:

```rust

pub mod coords;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
```

**Step 8: Add rendering module to lib.rs**

Modify `src/lib.rs` - add after `pub mod components;`:

```rust
pub mod rendering;
```

**Step 9: Run all tests to verify module integration**

Run: `cargo test`

Expected: PASS (12 tests = 7 existing + 5 new)

**Step 10: Commit**

```bash
git add src/rendering/mod.rs src/rendering/coords.rs src/lib.rs
git commit -m "feat: add type-safe coordinate newtypes (PixelCoord, ImageCoord<T>, ImageRect<T>)"
```

---

## Task 2: Create Viewport Structure

**Files:**

- Create: `src/rendering/viewport.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write Viewport struct with tests**

Create `src/rendering/viewport.rs`:

```rust

use crate::rendering::coords::{ImageCoord, ImageRect};

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport<T> {
  pub center: ImageCoord<T>,
  pub zoom: f64,
  pub natural_bounds: ImageRect<T>,
}

impl<T: Clone> Viewport<T> {
  pub fn new(center: ImageCoord<T>, zoom: f64, natural_bounds: ImageRect<T>) -> Self {
    Self {
      center,
      zoom,
      natural_bounds,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_viewport_construction() {
    let viewport = Viewport::new(
      ImageCoord::new(0.0, 0.0),
      1.0,
      ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
    );
    assert_eq!(*viewport.center.x(), 0.0);
    assert_eq!(*viewport.center.y(), 0.0);
    assert_eq!(viewport.zoom, 1.0);
  }

  #[test]
  fn test_viewport_generic_types() {
    let viewport_f64 = Viewport::new(
      ImageCoord::new(0.0, 0.0),
      1.0,
      ImageRect::new(ImageCoord::new(-1.0, -1.0), ImageCoord::new(1.0, 1.0)),
    );
    let viewport_i32 = Viewport::new(
      ImageCoord::new(0, 0),
      2.0,
      ImageRect::new(ImageCoord::new(-10, -10), ImageCoord::new(10, 10)),
    );
    assert_eq!(viewport_f64.zoom, 1.0);
    assert_eq!(viewport_i32.zoom, 2.0);
  }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test rendering::viewport::tests`

Expected: PASS (2 tests)

**Step 3: Add viewport to rendering module exports**

Modify `src/rendering/mod.rs`:

```rust
pub mod coords;
pub mod viewport;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
pub use viewport::Viewport;
```

**Step 4: Run all tests to verify module integration**

Run: `cargo test`

Expected: PASS (14 tests)

**Step 5: Commit**

```bash
git add src/rendering/viewport.rs src/rendering/mod.rs
git commit -m "feat: add generic Viewport struct with center, zoom, and natural bounds"
```

---

## Task 3: Create Coordinate Transformation Utilities

**Files:**

- Create: `src/rendering/transforms.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write failing test for calculate_visible_bounds with landscape aspect**

Create `src/rendering/transforms.rs`:

```rust

use crate::rendering::{coords::{ImageCoord, ImageRect, PixelCoord}, viewport::Viewport};

pub fn calculate_visible_bounds<T>(
  viewport: &Viewport<T>,
  canvas_width: u32,
  canvas_height: u32,
) -> ImageRect<T>
where
  T: Clone
    + std::ops::Sub<Output = T>
    + std::ops::Div<f64, Output = T>
    + std::ops::Mul<f64, Output = T>
    + std::ops::Add<Output = T>,
{
  let natural_width = viewport.natural_bounds.max.x().clone() - viewport.natural_bounds.min.x().clone();
  let natural_height = viewport.natural_bounds.max.y().clone() - viewport.natural_bounds.min.y().clone();

  // Apply zoom (1.0 = show entire natural bounds)
  let view_width = natural_width / viewport.zoom;
  let view_height = natural_height / viewport.zoom;

  // Adjust for canvas aspect ratio - extend the wider dimension
  let canvas_aspect = canvas_width as f64 / canvas_height as f64;

  // For generic T, we need to convert to f64 for aspect comparison
  // This works for f64 directly, rug::Float would need custom implementation
  let view_width_f64 = view_width.clone();
  let view_height_f64 = view_height.clone();

  // Simplified: assume T can be multiplied by f64
  let (final_width, final_height) = if canvas_aspect > 1.0 {
    // Landscape - extend width
    (view_height.clone() * canvas_aspect, view_height)
  } else {
    // Portrait - extend height
    (view_width, view_width.clone() / canvas_aspect)
  };

  // Calculate bounds centered on viewport.center
  ImageRect::new(
    ImageCoord::new(
      viewport.center.x().clone() - final_width.clone() / 2.0,
      viewport.center.y().clone() - final_height.clone() / 2.0,
    ),
    ImageCoord::new(
      viewport.center.x().clone() + final_width / 2.0,
      viewport.center.y().clone() + final_height / 2.0,
    ),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_calculate_visible_bounds_landscape() {
    let viewport = Viewport::new(
      ImageCoord::new(0.0, 0.0),
      1.0,
      ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
    );

    // Landscape canvas: 1600x900 (aspect ratio ~1.78)
    let bounds = calculate_visible_bounds(&viewport, 1600, 900);

    // At zoom 1.0, should show entire natural height (100 units)
    // Width should extend to maintain aspect ratio
    assert_eq!(bounds.height(), 100.0);
    assert!((bounds.width() - 177.77).abs() < 0.1); // 100 * 1.78
    assert_eq!(*bounds.min.y(), -50.0);
    assert_eq!(*bounds.max.y(), 50.0);
  }

  #[test]
  fn test_calculate_visible_bounds_portrait() {
    let viewport = Viewport::new(
      ImageCoord::new(0.0, 0.0),
      1.0,
      ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
    );

    // Portrait canvas: 900x1600
    let bounds = calculate_visible_bounds(&viewport, 900, 1600);

    // At zoom 1.0, should show entire natural width (100 units)
    // Height should extend to maintain aspect ratio
    assert_eq!(bounds.width(), 100.0);
    assert!((bounds.height() - 177.77).abs() < 0.1);
    assert_eq!(*bounds.min.x(), -50.0);
    assert_eq!(*bounds.max.x(), 50.0);
  }

  #[test]
  fn test_calculate_visible_bounds_zoom() {
    let viewport = Viewport::new(
      ImageCoord::new(0.0, 0.0),
      2.0, // 2x zoom
      ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
    );

    // Square canvas
    let bounds = calculate_visible_bounds(&viewport, 1000, 1000);

    // At zoom 2.0, should show half the natural area (50 units)
    assert_eq!(bounds.width(), 50.0);
    assert_eq!(bounds.height(), 50.0);
  }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test rendering::transforms::tests`

Expected: PASS (3 tests)

**Step 3: Add pixel_to_image transformation with tests**

Add to `src/rendering/transforms.rs` after `calculate_visible_bounds`:

```rust
pub fn pixel_to_image<T>(
  pixel: PixelCoord,
  visible_bounds: &ImageRect<T>,
  canvas_width: u32,
  canvas_height: u32,
) -> ImageCoord<T>
where
  T: Clone
    + std::ops::Sub<Output = T>
    + std::ops::Mul<f64, Output = T>
    + std::ops::Add<Output = T>,
{
  let bounds_width = visible_bounds.max.x().clone() - visible_bounds.min.x().clone();
  let bounds_height = visible_bounds.max.y().clone() - visible_bounds.min.y().clone();

  ImageCoord::new(
    visible_bounds.min.x().clone() + bounds_width * (pixel.x() / canvas_width as f64),
    visible_bounds.min.y().clone() + bounds_height * (pixel.y() / canvas_height as f64),
  )
}
```

Add to tests module:

```rust
#[test]
fn test_pixel_to_image_center() {
  let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
  let pixel = PixelCoord::new(500.0, 500.0); // Center of 1000x1000 canvas
  let image = pixel_to_image(pixel, &bounds, 1000, 1000);

  assert_eq!(*image.x(), 0.0);
  assert_eq!(*image.y(), 0.0);
}

#[test]
fn test_pixel_to_image_corners() {
  let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));

  // Top-left corner
  let image = pixel_to_image(PixelCoord::new(0.0, 0.0), &bounds, 1000, 1000);
  assert_eq!(*image.x(), -50.0);
  assert_eq!(*image.y(), -50.0);

  // Bottom-right corner
  let image = pixel_to_image(PixelCoord::new(1000.0, 1000.0), &bounds, 1000, 1000);
  assert_eq!(*image.x(), 50.0);
  assert_eq!(*image.y(), 50.0);
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test rendering::transforms::tests`

Expected: PASS (5 tests)

**Step 5: Add image_to_pixel transformation with tests**

Add to `src/rendering/transforms.rs` after `pixel_to_image`:

```rust
pub fn image_to_pixel<T>(
  image: &ImageCoord<T>,
  visible_bounds: &ImageRect<T>,
  canvas_width: u32,
  canvas_height: u32,
) -> PixelCoord
where
  T: Clone + std::ops::Sub<Output = T> + std::ops::Div<Output = T>,
  f64: std::ops::Mul<T, Output = f64>,
{
  let bounds_width = visible_bounds.max.x().clone() - visible_bounds.min.x().clone();
  let bounds_height = visible_bounds.max.y().clone() - visible_bounds.min.y().clone();

  let normalized_x = (image.x().clone() - visible_bounds.min.x().clone()) / bounds_width;
  let normalized_y = (image.y().clone() - visible_bounds.min.y().clone()) / bounds_height;

  PixelCoord::new(
    canvas_width as f64 * normalized_x,
    canvas_height as f64 * normalized_y,
  )
}
```

Add to tests module:

```rust
#[test]
fn test_image_to_pixel_center() {
  let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
  let image = ImageCoord::new(0.0, 0.0);
  let pixel = image_to_pixel(&image, &bounds, 1000, 1000);

  assert_eq!(pixel.x(), 500.0);
  assert_eq!(pixel.y(), 500.0);
}

#[test]
fn test_round_trip_pixel_image_pixel() {
  let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
  let original = PixelCoord::new(123.0, 456.0);

  let image = pixel_to_image(original, &bounds, 1000, 1000);
  let result = image_to_pixel(&image, &bounds, 1000, 1000);

  assert!((result.x() - original.x()).abs() < 0.001);
  assert!((result.y() - original.y()).abs() < 0.001);
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test rendering::transforms::tests`

Expected: PASS (7 tests)

**Step 7: Add transforms to rendering module exports**

Modify `src/rendering/mod.rs`:

```rust
pub mod coords;
pub mod transforms;
pub mod viewport;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
pub use transforms::{calculate_visible_bounds, image_to_pixel, pixel_to_image};
pub use viewport::Viewport;
```

**Step 8: Run all tests to verify module integration**

Run: `cargo test`

Expected: PASS (21 tests)

**Step 9: Commit**

```bash
git add src/rendering/transforms.rs src/rendering/mod.rs
git commit -m "feat: add coordinate transformation utilities with aspect ratio handling"
```

---

## Task 4: Create CanvasRenderer Trait

**Files:**

- Create: `src/rendering/renderer_trait.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Define CanvasRenderer trait**

Create `src/rendering/renderer_trait.rs`:

```rust

use crate::rendering::coords::ImageRect;

pub trait CanvasRenderer {
  type Coord: Clone;

  /// Returns the natural bounds of this renderer (what zoom 1.0 should display)
  fn natural_bounds(&self) -> ImageRect<Self::Coord>;

  /// Renders the specified image-space rectangle to pixel data
  /// Returns RGBA pixel data (width * height * 4 bytes)
  fn render(&self, target_rect: &ImageRect<Self::Coord>, width: u32, height: u32) -> Vec<u8>;
}
```

**Step 2: Add renderer_trait to rendering module exports**

Modify `src/rendering/mod.rs`:

```rust
pub mod coords;
pub mod renderer_trait;
pub mod transforms;
pub mod viewport;

pub use coords::{ImageCoord, ImageRect, PixelCoord};
pub use renderer_trait::CanvasRenderer;
pub use transforms::{calculate_visible_bounds, image_to_pixel, pixel_to_image};
pub use viewport::Viewport;
```

**Step 3: Verify compilation**

Run: `cargo check`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/rendering/renderer_trait.rs src/rendering/mod.rs
git commit -m "feat: add CanvasRenderer trait with associated coordinate type"
```

---

## Task 5: Implement TestImageRenderer

**Files:**

- Create: `src/components/test_image.rs`
- Modify: `src/components/mod.rs`

**Step 1: Create TestImageRenderer struct and implement CanvasRenderer**

Create `src/components/test_image.rs`:

```rust

use crate::rendering::{
  coords::{ImageCoord, ImageRect},
  renderer_trait::CanvasRenderer,
};

pub struct TestImageRenderer {
  checkerboard_size: f64,
  circle_radius_step: f64,
  circle_line_thickness: f64,
}

impl TestImageRenderer {
  pub fn new() -> Self {
    Self {
      checkerboard_size: 10.0,
      circle_radius_step: 10.0,
      circle_line_thickness: 0.5,
    }
  }

  fn compute_pixel_color(&self, x: f64, y: f64) -> (u8, u8, u8, u8) {
    // Check if on circle first (circles drawn on top)
    let distance = (x * x + y * y).sqrt();
    let nearest_ring = (distance / self.circle_radius_step).round();
    let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

    if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
      return (255, 0, 0, 255); // Red circle
    }

    // Checkerboard: (0,0) is corner of four squares
    let square_x = (x / self.checkerboard_size).floor() as i32;
    let square_y = (y / self.checkerboard_size).floor() as i32;
    let is_light = (square_x + square_y) % 2 == 0;

    if is_light {
      (255, 255, 255, 255) // White
    } else {
      (204, 204, 204, 255) // Light grey
    }
  }
}

impl CanvasRenderer for TestImageRenderer {
  type Coord = f64;

  fn natural_bounds(&self) -> ImageRect<f64> {
    ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0))
  }

  fn render(&self, target_rect: &ImageRect<f64>, width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for py in 0..height {
      for px in 0..width {
        // Map pixel to image coordinates
        let img_x = target_rect.min.x()
          + (px as f64 / width as f64) * (target_rect.max.x() - target_rect.min.x());
        let img_y = target_rect.min.y()
          + (py as f64 / height as f64) * (target_rect.max.y() - target_rect.min.y());

        let color = self.compute_pixel_color(img_x, img_y);

        let idx = ((py * width + px) * 4) as usize;
        pixels[idx] = color.0; // R
        pixels[idx + 1] = color.1; // G
        pixels[idx + 2] = color.2; // B
        pixels[idx + 3] = color.3; // A
      }
    }

    pixels
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_renderer_natural_bounds() {
    let renderer = TestImageRenderer::new();
    let bounds = renderer.natural_bounds();
    assert_eq!(*bounds.min.x(), -50.0);
    assert_eq!(*bounds.max.x(), 50.0);
  }

  #[test]
  fn test_renderer_produces_correct_pixel_count() {
    let renderer = TestImageRenderer::new();
    let bounds = ImageRect::new(ImageCoord::new(-10.0, -10.0), ImageCoord::new(10.0, 10.0));
    let pixels = renderer.render(&bounds, 100, 100);
    assert_eq!(pixels.len(), 100 * 100 * 4);
  }

  #[test]
  fn test_checkerboard_pattern_at_origin() {
    let renderer = TestImageRenderer::new();

    // Point at (-5, -5) should be in one square
    let color1 = renderer.compute_pixel_color(-5.0, -5.0);
    // Point at (5, 5) should be in same color (both negative square indices sum to even)
    let color2 = renderer.compute_pixel_color(5.0, 5.0);
    // Point at (5, -5) should be opposite color
    let color3 = renderer.compute_pixel_color(5.0, -5.0);

    assert_eq!(color1, color2);
    assert_ne!(color1, color3);
  }

  #[test]
  fn test_circle_at_radius_10() {
    let renderer = TestImageRenderer::new();

    // Point exactly on circle (radius 10)
    let color_on = renderer.compute_pixel_color(10.0, 0.0);
    assert_eq!(color_on, (255, 0, 0, 255)); // Red

    // Point between circles
    let color_off = renderer.compute_pixel_color(15.0, 0.0);
    assert_ne!(color_off, (255, 0, 0, 255)); // Not red
  }

  #[test]
  fn test_origin_is_corner_of_four_squares() {
    let renderer = TestImageRenderer::new();

    // (0,0) is corner, so nearby points in different quadrants have different colors
    let q1 = renderer.compute_pixel_color(1.0, 1.0);
    let q2 = renderer.compute_pixel_color(-1.0, 1.0);
    let q3 = renderer.compute_pixel_color(-1.0, -1.0);
    let q4 = renderer.compute_pixel_color(1.0, -1.0);

    // Opposite quadrants should have same color
    assert_eq!(q1, q3);
    assert_eq!(q2, q4);
    assert_ne!(q1, q2);
  }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test components::test_image::tests`

Expected: PASS (5 tests)

**Step 3: Add test_image to components module**

Modify `src/components/mod.rs` - add after existing modules:

```rust
pub mod test_image;
```

**Step 4: Run all tests to verify integration**

Run: `cargo test`

Expected: PASS (26 tests)

**Step 5: Commit**

```bash
git add src/components/test_image.rs src/components/mod.rs
git commit -m "feat: implement TestImageRenderer with checkerboard and concentric circles"
```

---

## Task 6: Modify Canvas Component to Be Generic

**Files:**

- Modify: `src/components/canvas.rs`

**Step 1: Update Canvas to be generic over CanvasRenderer**

Replace entire contents of `src/components/canvas.rs`:

```rust

use leptos::html::Canvas;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::rendering::{renderer_trait::CanvasRenderer, transforms::calculate_visible_bounds, viewport::Viewport};

#[component]
pub fn Canvas<R>(renderer: R, viewport: ReadSignal<Viewport<R::Coord>>) -> impl IntoView
where
  R: CanvasRenderer + 'static,
  R::Coord: Clone
    + std::ops::Sub<Output = R::Coord>
    + std::ops::Div<f64, Output = R::Coord>
    + std::ops::Mul<f64, Output = R::Coord>
    + std::ops::Add<Output = R::Coord>,
{
  let canvas_ref = NodeRef::<Canvas>::new();

  // Main render function
  let render = move || {
    let canvas = canvas_ref.get().expect("canvas element should be mounted");
    let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

    let context = canvas_element
      .get_context("2d")
      .expect("should get 2d context")
      .expect("context should not be null")
      .dyn_into::<CanvasRenderingContext2d>()
      .expect("should cast to CanvasRenderingContext2d");

    let width = canvas_element.width();
    let height = canvas_element.height();

    // Calculate what image-space rectangle is visible
    let visible_bounds = calculate_visible_bounds(&viewport.get(), width, height);

    // Ask renderer for pixel data
    let pixel_data = renderer.render(&visible_bounds, width, height);

    // Put pixels on canvas
    let image_data =
      ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&pixel_data), width, height)
        .expect("should create ImageData");

    context.put_image_data(&image_data, 0.0, 0.0).expect("should put image data");
  };

  // Initialize canvas on mount
  create_effect(move |_| {
    if canvas_ref.get().is_some() {
      let canvas = canvas_ref.get().expect("canvas should exist");
      let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

      let window = web_sys::window().expect("should have window");
      canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
      canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

      render();
    }
  });

  // Re-render when viewport changes
  create_effect(move |_| {
    viewport.track(); // Track viewport signal
    if canvas_ref.get().is_some() {
      render();
    }
  });

  // Handle window resize
  let handle_resize = move || {
    if let Some(canvas) = canvas_ref.get() {
      let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();
      let window = web_sys::window().expect("should have window");

      canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
      canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

      render();
    }
  };

  let _ = leptos_use::use_event_listener(leptos_use::use_window(), leptos::ev::resize, move |_| {
    handle_resize();
  });

  view! {
    <canvas
      node_ref=canvas_ref
      class="block w-full h-full"
      style="touch-action: none; cursor: grab;"
    />
  }
}
```

**Step 2: Verify compilation**

Run: `cargo check`

Expected: SUCCESS (canvas tests removed, will verify with integration)

**Step 3: Commit**

```bash
git add src/components/canvas.rs
git commit -m "feat: make Canvas component generic over CanvasRenderer trait"
```

---

## Task 7: Create TestImageView Wrapper Component

**Files:**

- Modify: `src/components/test_image.rs`

**Step 1: Add TestImageView component to test_image.rs**

Add to `src/components/test_image.rs` after the impl blocks:

```rust
use crate::components::canvas::Canvas;
use leptos::*;

#[component]
pub fn TestImageView() -> impl IntoView {
  let renderer = TestImageRenderer::new();

  // Initialize viewport - center at (0,0), zoom 1.0 shows full natural bounds
  let (viewport, _set_viewport) = create_signal(Viewport {
    center: ImageCoord::new(0.0, 0.0),
    zoom: 1.0,
    natural_bounds: renderer.natural_bounds(),
  });

  view! { <Canvas renderer=renderer viewport=viewport /> }
}
```

Add import at top of file:

```rust
use crate::rendering::viewport::Viewport;
```

**Step 2: Verify compilation**

Run: `cargo check`

Expected: SUCCESS

**Step 3: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: add TestImageView wrapper component with viewport state"
```

---

## Task 8: Update App Component for Dynamic Renderer Switching

**Files:**

- Modify: `src/app.rs`

**Step 1: Add RendererType enum and update App component**

Replace entire contents of `src/app.rs`:

```rust

use leptos::*;

use crate::components::{test_image::TestImageView, ui::UI};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
  TestImage,
  // Future: Mandelbrot, Julia, BurningShip, etc.
}

#[component]
pub fn App() -> impl IntoView {
  // UI controls this signal (currently fixed to TestImage)
  let (current_renderer, _set_current_renderer) = create_signal(RendererType::TestImage);

  view! {
    <div class="relative w-screen h-screen overflow-hidden bg-black">
      // Dynamic renderer switching
      {move || match current_renderer.get() {
        RendererType::TestImage => {
          view! { <TestImageView /> }.into_view()
        }
      }}

      <UI />
    </div>
  }
}
```

**Step 2: Verify compilation**

Run: `cargo check`

Expected: SUCCESS

**Step 3: Build WASM to verify full integration**

Run: `cargo build --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: add dynamic renderer switching with RendererType enum"
```

---

## Task 9: Integration Testing and Documentation

**Files:**

- Create: `docs/RENDERER-ARCHITECTURE.md`

**Step 1: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features`

Expected: PASS (21+ tests)

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: No warnings

**Step 3: Run formatter check**

Run: `cargo fmt --all -- --check`

Expected: All files formatted

**Step 4: Create architecture documentation**

Create `docs/RENDERER-ARCHITECTURE.md`:

````markdown
# Pluggable Renderer Architecture

## Overview

Type-safe, pluggable renderer system with compile-time coordinate space separation.

## Core Abstractions

### Coordinate Types (`src/rendering/coords.rs`)

**PixelCoord** - Screen-space coordinates (always f64)

- Private fields prevent raw access
- Used for canvas pixel positions

**ImageCoord<T>** - Image-space coordinates (generic over precision)

- T = f64 for test images
- T = rug::Float for high-precision fractals
- Private fields enforce transformation through utilities

**ImageRect<T>** - Rectangular region in image space

- Defines rendering target area

### Viewport (`src/rendering/viewport.rs`)

```rust
struct Viewport<T> {
  center: ImageCoord<T>,
  zoom: f64,
  natural_bounds: ImageRect<T>,
}
```
````

- `zoom = 1.0` displays entire natural_bounds
- `zoom = 2.0` displays half the area (2x magnification)

### Transformations (`src/rendering/transforms.rs`)

**calculate_visible_bounds** - Computes visible ImageRect from viewport + canvas size

- Handles aspect ratio by extending wider dimension
- Ensures natural_bounds fits in constraint dimension at zoom 1.0

**pixel_to_image** - Converts PixelCoord → ImageCoord<T>

- Requires viewport context
- Type system prevents conversion without context

**image_to_pixel** - Converts ImageCoord<T> → PixelCoord

- Inverse transformation
- Round-trip guarantees precision

### CanvasRenderer Trait (`src/rendering/renderer_trait.rs`)

```rust
trait CanvasRenderer {
  type Coord: Clone;
  fn natural_bounds(&self) -> ImageRect<Self::Coord>;
  fn render(&self, target_rect: &ImageRect<Self::Coord>, width: u32, height: u32) -> Vec<u8>;
}
```

- Associated type `Coord` declares precision requirements
- `render()` receives arbitrary ImageRect for future tiling support
- Returns raw RGBA pixel data

## Type Safety Guarantees

1. **Cannot mix coordinate spaces** - PixelCoord and ImageCoord<T> are distinct types
2. **Cannot access raw coordinates without explicit call** - Private fields require `.x()`, `.y()`
3. **Cannot convert without context** - Transformations require viewport + canvas dimensions
4. **Cannot mix precision types** - f64 and rug::Float renderers use different ImageCoord<T>

## Component Architecture

**Canvas<R: CanvasRenderer>** - Generic rendering component

- Calculates visible bounds from viewport
- Calls renderer.render()
- Puts pixels on HTML canvas

**TestImageView** - Wrapper owning viewport state

- Creates renderer instance
- Manages viewport signal
- Composes with Canvas

**App** - Top-level with dynamic switching

- RendererType enum for selection
- Component-level match expression
- Each renderer branch fully typed

## Adding New Renderers

1. Implement `CanvasRenderer` trait
2. Declare `type Coord = f64` or `rug::Float`
3. Implement `natural_bounds()` and `render()`
4. Create wrapper component owning viewport
5. Add variant to `RendererType` enum
6. Add branch to App match expression

## Future Extensions

- Tiling system: Renderer already receives arbitrary ImageRect
- Progressive rendering: Wrapper component manages tile queue
- Pan/zoom: Wrapper component modifies viewport signal
- URL persistence: Serialize viewport to URL params

````

**Step 5: Commit**

```bash
git add docs/RENDERER-ARCHITECTURE.md
git commit -m "docs: add pluggable renderer architecture documentation"
````

---

## Task 10: Manual Testing and Verification

**Files:**

- None (manual testing)

**Step 1: Start dev server**

Run: `trunk serve`

Expected: Server starts at http://localhost:8080

**Step 2: Open browser and verify test image renders**

Open: http://localhost:8080

**Visual verification checklist:**

- [ ] Checkerboard pattern visible (white and light grey squares)
- [ ] Squares are 10x10 image units (size varies with zoom)
- [ ] Concentric red circles centered at (0,0)
- [ ] Circles have radius 10, 20, 30, ... units
- [ ] (0,0) is corner of four checkerboard squares (center is between colors)
- [ ] Pattern fills entire canvas
- [ ] Aspect ratio preserved (no stretching)

**Step 3: Test window resize**

- Resize browser window to different aspect ratios
- Verify pattern re-renders without distortion
- Verify aspect ratio handling (landscape extends width, portrait extends height)

**Step 4: Check console for errors**

Open browser DevTools console

Expected: No errors

**Step 5: Document test results**

Create verification checklist in commit message for next step

**Step 6: Stop dev server**

Ctrl+C to stop trunk serve

**Step 7: Final commit**

```bash
git commit --allow-empty -m "test: manual verification of test image renderer

Checklist:
- [x] Checkerboard pattern renders correctly
- [x] Concentric circles centered at origin
- [x] (0,0) is corner point of four squares
- [x] Aspect ratio preserved on resize
- [x] No console errors
- [x] UI overlay still functional"
```

---

## Completion Checklist

- [ ] All 21+ tests pass
- [ ] Clippy shows no warnings
- [ ] Code formatted with rustfmt
- [ ] Manual visual verification complete
- [ ] Architecture documentation created
- [ ] All files committed with descriptive messages
- [ ] Ready for code review

## Notes for Future Work

**Immediate next steps:**

1. Add pan/zoom interactions (modify viewport signal)
2. Implement Mandelbrot renderer with rug::Float
3. Add tiling system for progressive rendering
4. URL persistence for viewport state

**Known limitations:**

- Transform trait bounds assume numeric types support f64 operations
- rug::Float will need custom implementations for some operations
- No performance optimization yet (single-threaded, no WebGPU)
