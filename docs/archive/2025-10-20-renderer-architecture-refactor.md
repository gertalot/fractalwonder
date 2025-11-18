# Renderer Architecture Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Refactor canvas rendering to separate pixel-space interactions from image-space rendering through composable trait-based architecture.

**Architecture:** Create `Renderer` and `PixelCompute` traits with composable wrappers (`TiledRenderer`, `PixelRenderer`). Extract canvas lifecycle and interaction handling into reusable `InteractiveCanvas<T, R>` component. Convert `TestImageRenderer` to `TestImageCompute` implementing `PixelCompute`.

**Tech Stack:** Rust, Leptos, existing rendering transforms, use_canvas_interaction hook (unchanged)

---

## Task 1: Create PixelRect type

**Files:**
- Create: `src/rendering/pixel_rect.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write the PixelRect struct**

Create `src/rendering/pixel_rect.rs`:

```rust
/// Rectangle in pixel space, used for rendering sub-regions of canvas
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelRect {
    /// X coordinate of top-left corner
    pub x: u32,
    /// Y coordinate of top-left corner
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl PixelRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a PixelRect covering full canvas
    pub fn full_canvas(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let rect = PixelRect::new(10, 20, 100, 200);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_full_canvas() {
        let rect = PixelRect::full_canvas(800, 600);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 800);
        assert_eq!(rect.height, 600);
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --lib pixel_rect`
Expected: 2 tests pass

**Step 3: Export PixelRect from rendering module**

In `src/rendering/mod.rs`, add:

```rust
pub mod pixel_rect;
pub use pixel_rect::PixelRect;
```

**Step 4: Verify module compiles**

Run: `cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src/rendering/pixel_rect.rs src/rendering/mod.rs
git commit -m "feat: add PixelRect type for pixel-space rectangles"
```

---

## Task 2: Create Renderer trait

**Files:**
- Modify: `src/rendering/renderer_trait.rs` (rename from CanvasRenderer)
- Modify: `src/rendering/mod.rs`

**Step 1: Update renderer trait signature**

Replace the entire contents of `src/rendering/renderer_trait.rs` with:

```rust
use crate::rendering::{coords::Rect, viewport::Viewport, PixelRect};

/// Core trait for rendering pixel data given viewport and pixel-space dimensions
///
/// Implementations can be composed (e.g., TiledRenderer wrapping PixelRenderer)
pub trait Renderer {
    /// Coordinate type for image space (f64, rug::Float, etc.)
    type Coord;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Render pixels for a given viewport and pixel rectangle
    ///
    /// # Arguments
    /// * `viewport` - What image coordinates the full canvas shows
    /// * `pixel_rect` - Which portion of canvas to render (for tiling)
    /// * `canvas_size` - Full canvas dimensions (width, height)
    ///
    /// # Returns
    /// RGBA pixel data for the specified pixel_rect (length = width * height * 4)
    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8>;
}
```

**Step 2: Update mod.rs exports**

In `src/rendering/mod.rs`, update the renderer_trait export:

```rust
pub mod renderer_trait;
pub use renderer_trait::Renderer;  // Changed from CanvasRenderer
```

**Step 3: Verify it compiles (will have errors in other files)**

Run: `cargo check 2>&1 | head -20`
Expected: Errors about CanvasRenderer not found - this is expected

**Step 4: Commit**

```bash
git add src/rendering/renderer_trait.rs src/rendering/mod.rs
git commit -m "feat: update Renderer trait with new signature for composability"
```

---

## Task 3: Create PixelCompute trait

**Files:**
- Create: `src/rendering/pixel_compute.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write the PixelCompute trait**

Create `src/rendering/pixel_compute.rs`:

```rust
use crate::rendering::coords::{Coord, Rect};

/// Trait for computing individual pixel colors from image coordinates
///
/// This is the lowest-level rendering abstraction - pure computation with no loops.
/// Typically wrapped by PixelRenderer which adds the pixel iteration logic.
pub trait PixelCompute {
    /// Coordinate type for image space
    type Coord;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Compute RGBA color for a single point in image space
    ///
    /// # Arguments
    /// * `coord` - Point in image-space coordinates
    ///
    /// # Returns
    /// (R, G, B, A) tuple, each 0-255
    fn compute(&self, coord: Coord<Self::Coord>) -> (u8, u8, u8, u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test implementation
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl PixelCompute for SolidColorCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Coord<f64>) -> (u8, u8, u8, u8) {
            self.color
        }
    }

    #[test]
    fn test_pixel_compute_trait() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let result = computer.compute(Coord::new(50.0, 50.0));
        assert_eq!(result, (255, 0, 0, 255));
    }
}
```

**Step 2: Run test**

Run: `cargo test --lib pixel_compute`
Expected: 1 test passes

**Step 3: Export from rendering module**

In `src/rendering/mod.rs`, add:

```rust
pub mod pixel_compute;
pub use pixel_compute::PixelCompute;
```

**Step 4: Verify**

Run: `cargo check`
Expected: Still has errors from Task 2, but pixel_compute compiles

**Step 5: Commit**

```bash
git add src/rendering/pixel_compute.rs src/rendering/mod.rs
git commit -m "feat: add PixelCompute trait for single-pixel computation"
```

---

## Task 4: Create PixelRenderer wrapper

**Files:**
- Create: `src/rendering/pixel_renderer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write test for PixelRenderer**

Create `src/rendering/pixel_renderer.rs`:

```rust
use crate::rendering::{
    coords::{Coord, Rect},
    pixel_compute::PixelCompute,
    renderer_trait::Renderer,
    transforms::pixel_to_image,
    viewport::Viewport,
    PixelRect,
};

/// Renderer that wraps a PixelCompute, adding pixel iteration logic
///
/// This is a composable wrapper that converts PixelCompute (single pixel)
/// into a full Renderer (pixel rectangle).
#[derive(Clone)]
pub struct PixelRenderer<C: PixelCompute> {
    computer: C,
}

impl<C: PixelCompute> PixelRenderer<C> {
    pub fn new(computer: C) -> Self {
        Self { computer }
    }
}

impl<C> Renderer for PixelRenderer<C>
where
    C: PixelCompute,
    C::Coord: Clone,
{
    type Coord = C::Coord;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.computer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; (pixel_rect.width * pixel_rect.height * 4) as usize];

        for local_y in 0..pixel_rect.height {
            for local_x in 0..pixel_rect.width {
                // Convert local pixel coords to absolute canvas coords
                let abs_x = pixel_rect.x + local_x;
                let abs_y = pixel_rect.y + local_y;

                // Map pixel to image coordinates
                let image_coord = pixel_to_image(
                    abs_x as f64,
                    abs_y as f64,
                    viewport,
                    canvas_size.0,
                    canvas_size.1,
                );

                // Compute color
                let (r, g, b, a) = self.computer.compute(image_coord);

                // Write to output
                let idx = ((local_y * pixel_rect.width + local_x) * 4) as usize;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = a;
            }
        }

        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCompute;

    impl PixelCompute for TestCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(-10.0, -10.0), Coord::new(10.0, 10.0))
        }

        fn compute(&self, coord: Coord<f64>) -> (u8, u8, u8, u8) {
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
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            renderer.natural_bounds(),
        );
        let pixel_rect = PixelRect::full_canvas(10, 10);
        let pixels = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(pixels.len(), 10 * 10 * 4);

        // First pixel (top-left, x < 0) should be blue
        assert_eq!(&pixels[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_pixel_renderer_partial_rect() {
        let renderer = PixelRenderer::new(TestCompute);
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            renderer.natural_bounds(),
        );
        // Render just a 5x5 tile starting at (2, 2)
        let pixel_rect = PixelRect::new(2, 2, 5, 5);
        let pixels = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(pixels.len(), 5 * 5 * 4);
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib pixel_renderer`
Expected: 2 tests pass

**Step 3: Export from module**

In `src/rendering/mod.rs`:

```rust
pub mod pixel_renderer;
pub use pixel_renderer::PixelRenderer;
```

**Step 4: Verify**

Run: `cargo check`
Expected: Compiles

**Step 5: Commit**

```bash
git add src/rendering/pixel_renderer.rs src/rendering/mod.rs
git commit -m "feat: add PixelRenderer wrapper for PixelCompute implementations"
```

---

## Task 5: Create TiledRenderer wrapper

**Files:**
- Create: `src/rendering/tiled_renderer.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Write TiledRenderer implementation**

Create `src/rendering/tiled_renderer.rs`:

```rust
use crate::rendering::{
    coords::Rect,
    renderer_trait::Renderer,
    viewport::Viewport,
    PixelRect,
};

/// Renderer that splits rendering into tiles, delegating to inner renderer
///
/// This is a composable wrapper that adds tiling to any Renderer implementation.
/// Useful for parallelization, progress tracking, or memory management.
#[derive(Clone)]
pub struct TiledRenderer<R: Renderer> {
    inner: R,
    tile_size: u32,
}

impl<R: Renderer> TiledRenderer<R> {
    pub fn new(inner: R, tile_size: u32) -> Self {
        Self { inner, tile_size }
    }
}

impl<R> Renderer for TiledRenderer<R>
where
    R: Renderer,
    R::Coord: Clone,
{
    type Coord = R::Coord;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.inner.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8> {
        let mut output = vec![0u8; (pixel_rect.width * pixel_rect.height * 4) as usize];

        // Iterate over tiles within pixel_rect
        let mut tile_y = 0;
        while tile_y < pixel_rect.height {
            let mut tile_x = 0;
            while tile_x < pixel_rect.width {
                let tile_width = self.tile_size.min(pixel_rect.width - tile_x);
                let tile_height = self.tile_size.min(pixel_rect.height - tile_y);

                // Create tile rect in absolute canvas coordinates
                let tile_rect = PixelRect::new(
                    pixel_rect.x + tile_x,
                    pixel_rect.y + tile_y,
                    tile_width,
                    tile_height,
                );

                // Render this tile
                let tile_pixels = self.inner.render(viewport, tile_rect, canvas_size);

                // Copy tile pixels into output buffer
                for y in 0..tile_height {
                    for x in 0..tile_width {
                        let tile_idx = ((y * tile_width + x) * 4) as usize;
                        let output_x = tile_x + x;
                        let output_y = tile_y + y;
                        let output_idx = ((output_y * pixel_rect.width + output_x) * 4) as usize;

                        output[output_idx..output_idx + 4]
                            .copy_from_slice(&tile_pixels[tile_idx..tile_idx + 4]);
                    }
                }

                tile_x += self.tile_size;
            }
            tile_y += self.tile_size;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{coords::Coord, pixel_compute::PixelCompute, pixel_renderer::PixelRenderer};

    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl PixelCompute for SolidColorCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Coord<f64>) -> (u8, u8, u8, u8) {
            self.color
        }
    }

    #[test]
    fn test_tiled_renderer_produces_same_output() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let direct_renderer = PixelRenderer::new(computer.clone());
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 16);

        let viewport = Viewport::new(
            Coord::new(50.0, 50.0),
            1.0,
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0)),
        );
        let pixel_rect = PixelRect::full_canvas(32, 32);

        let direct_pixels = direct_renderer.render(&viewport, pixel_rect, (32, 32));
        let tiled_pixels = tiled_renderer.render(&viewport, pixel_rect, (32, 32));

        assert_eq!(direct_pixels, tiled_pixels);
    }

    #[test]
    fn test_tiled_renderer_with_non_multiple_size() {
        // Test that tiling works when canvas size is not a multiple of tile_size
        let computer = SolidColorCompute {
            color: (0, 255, 0, 255),
        };
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 10);

        let viewport = Viewport::new(
            Coord::new(50.0, 50.0),
            1.0,
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0)),
        );
        let pixel_rect = PixelRect::full_canvas(27, 27); // Not divisible by 10

        let pixels = tiled_renderer.render(&viewport, pixel_rect, (27, 27));

        assert_eq!(pixels.len(), 27 * 27 * 4);
        // All pixels should be green
        assert_eq!(&pixels[0..4], &[0, 255, 0, 255]);
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib tiled_renderer`
Expected: 2 tests pass

**Step 3: Export from module**

In `src/rendering/mod.rs`:

```rust
pub mod tiled_renderer;
pub use tiled_renderer::TiledRenderer;
```

**Step 4: Verify**

Run: `cargo check`
Expected: Compiles

**Step 5: Commit**

```bash
git add src/rendering/tiled_renderer.rs src/rendering/mod.rs
git commit -m "feat: add TiledRenderer wrapper for splitting rendering into tiles"
```

---

## Task 6: Convert TestImageRenderer to TestImageCompute

**Files:**
- Modify: `src/components/test_image.rs` (major refactor)

**Step 1: Update TestImageRenderer to TestImageCompute**

In `src/components/test_image.rs`, replace the `impl CanvasRenderer for TestImageRenderer` block with:

```rust
use crate::rendering::{
    coords::{Coord, Rect},
    pixel_compute::PixelCompute,
};

impl PixelCompute for TestImageRenderer {
    type Coord = f64;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0))
    }

    fn compute(&self, coord: Coord<f64>) -> (u8, u8, u8, u8) {
        self.compute_pixel_color(*coord.x(), *coord.y())
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Errors in TestImageView component (uses old API)

**Step 3: Commit this step**

```bash
git add src/components/test_image.rs
git commit -m "refactor: convert TestImageRenderer to implement PixelCompute"
```

---

## Task 7: Create InteractiveCanvas component

**Files:**
- Create: `src/components/interactive_canvas.rs`
- Modify: `src/components/mod.rs`

**Step 1: Create InteractiveCanvas component**

Create `src/components/interactive_canvas.rs`:

```rust
use crate::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
use crate::rendering::{
    apply_pixel_transform_to_viewport,
    coords::Coord,
    render_with_viewport,
    renderer_trait::Renderer,
    viewport::Viewport,
    PixelRect,
};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::AddEventListenerOptions;

/// Generic interactive canvas component with pan/zoom support
///
/// Manages canvas lifecycle, viewport state, and interaction handling.
/// Works with any Renderer implementation.
///
/// # Type Parameters
/// * `T` - Coordinate type for image space (f64, rug::Float, etc.)
/// * `R` - Renderer implementation
///
/// # Example
/// ```rust,no_run
/// use fractalwonder::components::InteractiveCanvas;
/// use fractalwonder::rendering::{PixelRenderer, pixel_renderer};
///
/// let renderer = PixelRenderer::new(MyCompute::new());
/// view! { <InteractiveCanvas renderer=renderer /> }
/// ```
#[component]
pub fn InteractiveCanvas<T, R>(renderer: R) -> impl IntoView
where
    T: Clone + 'static,
    R: Renderer<Coord = T> + Clone + 'static,
{
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Initialize viewport from renderer's natural bounds
    let natural_bounds = renderer.natural_bounds();
    let center = Coord::new(
        // Calculate center from bounds
        // For f64: (max + min) / 2, but we need generic approach
        // Use existing Viewport logic or make this configurable
        // For now, assume Coord implements a center calculation
        natural_bounds.center().x().clone(),
        natural_bounds.center().y().clone(),
    );
    let viewport = create_rw_signal(Viewport::new(center, 1.0, natural_bounds));

    // Set up interaction hook with viewport update
    let handle = use_canvas_interaction(canvas_ref, move |result: TransformResult| {
        if let Some(canvas) = canvas_ref.get_untracked() {
            let width = canvas.width();
            let height = canvas.height();

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport.get_untracked(),
                &result,
                width,
                height,
            );

            viewport.set(new_viewport);
        }
    });

    // Initialize canvas on mount
    let renderer_for_init = renderer.clone();
    create_effect(move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let window = web_sys::window().expect("should have window");
            canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            // Initial render
            render_with_viewport(&canvas, &renderer_for_init, &viewport.get());
        }
    });

    // Re-render whenever viewport changes
    let renderer_for_updates = renderer.clone();
    create_effect(move |_| {
        let current_viewport = viewport.get();
        if let Some(canvas) = canvas_ref.get_untracked() {
            render_with_viewport(&canvas, &renderer_for_updates, &current_viewport);
        }
    });

    // Handle window resize
    create_effect({
        let on_canvas_resize = handle.on_canvas_resize.clone();
        let viewport_clone = viewport;
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                use wasm_bindgen::closure::Closure;

                let canvas_clone = canvas.clone();
                let on_canvas_resize = on_canvas_resize.clone();
                let viewport_for_resize = viewport_clone;

                let resize_handler = Closure::wrap(Box::new(move || {
                    let window = web_sys::window().expect("should have window");
                    let new_width = window.inner_width().unwrap().as_f64().unwrap() as u32;
                    let new_height = window.inner_height().unwrap().as_f64().unwrap() as u32;

                    let old_width = canvas_clone.width();
                    let old_height = canvas_clone.height();

                    if old_width != new_width || old_height != new_height {
                        (on_canvas_resize)(new_width, new_height);
                        canvas_clone.set_width(new_width);
                        canvas_clone.set_height(new_height);
                        viewport_for_resize.update(|v| {
                            *v = v.clone();
                        });
                    }
                }) as Box<dyn Fn() + 'static>);

                web_sys::window()
                    .expect("should have window")
                    .add_event_listener_with_callback(
                        "resize",
                        resize_handler.as_ref().unchecked_ref(),
                    )
                    .expect("should add resize listener");

                resize_handler.forget();
            }
        }
    });

    // Manually attach wheel event listener with passive: false
    create_effect({
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                let options = AddEventListenerOptions::new();
                options.set_passive(false);

                let on_wheel = handle.on_wheel.clone();
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |ev: web_sys::WheelEvent| {
                        (on_wheel)(ev);
                    },
                )
                    as Box<dyn Fn(web_sys::WheelEvent) + 'static>);

                canvas
                    .add_event_listener_with_callback_and_add_event_listener_options(
                        "wheel",
                        closure.as_ref().unchecked_ref(),
                        &options,
                    )
                    .expect("should add wheel listener");

                closure.forget();
            }
        }
    });

    view! {
        <div class="relative w-full h-full">
            <canvas
                node_ref=canvas_ref
                class="block w-full h-full"
                on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
                on:pointermove=move |ev| (handle.on_pointer_move)(ev)
                on:pointerup=move |ev| (handle.on_pointer_up)(ev)
                style="touch-action: none; cursor: grab;"
            />
        </div>
    }
}
```

**Step 2: Add center() method to Rect**

In `src/rendering/coords.rs`, add to the `Rect` impl block:

```rust
impl<T> Rect<T>
where
    T: Clone + std::ops::Add<Output = T> + std::ops::Div<Output = T> + From<f64>,
{
    /// Calculate center point of rectangle
    pub fn center(&self) -> Coord<T> {
        let two = T::from(2.0);
        let center_x = (self.min.x().clone() + self.max.x().clone()) / two.clone();
        let center_y = (self.min.y().clone() + self.max.y().clone()) / two;
        Coord::new(center_x, center_y)
    }
}
```

**Step 3: Update render_with_viewport to use new Renderer trait**

In `src/rendering/transforms.rs`, update `render_with_viewport`:

```rust
use crate::rendering::{renderer_trait::Renderer, PixelRect};

pub fn render_with_viewport<R>(
    canvas: &HtmlCanvasElement,
    renderer: &R,
    viewport: &Viewport<R::Coord>,
) where
    R: Renderer,
    R::Coord: Clone,
{
    let width = canvas.width();
    let height = canvas.height();
    let pixel_rect = PixelRect::full_canvas(width, height);
    let pixels = renderer.render(viewport, pixel_rect, (width, height));

    // Put pixels on canvas
    let context = canvas
        .get_context("2d")
        .expect("Failed to get context")
        .expect("Context is None")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("Failed to cast to 2D context");

    let image_data =
        ImageData::new_with_u8_clamped_array_and_sh(Clamped(&pixels), width, height)
            .expect("Failed to create ImageData");

    context
        .put_image_data(&image_data, 0.0, 0.0)
        .expect("Failed to put image data");
}
```

**Step 4: Export InteractiveCanvas**

In `src/components/mod.rs`:

```rust
pub mod interactive_canvas;
pub use interactive_canvas::InteractiveCanvas;
```

**Step 5: Verify (will have compile errors - center() method might need adjustment)**

Run: `cargo check 2>&1 | head -30`
Expected: Might have errors about trait bounds on Rect::center()

**Step 6: Commit**

```bash
git add src/components/interactive_canvas.rs src/components/mod.rs src/rendering/coords.rs src/rendering/transforms.rs
git commit -m "feat: add InteractiveCanvas generic component"
```

---

## Task 8: Simplify TestImageView to use InteractiveCanvas

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Replace TestImageView implementation**

In `src/components/test_image.rs`, replace the entire `TestImageView` component with:

```rust
use crate::components::InteractiveCanvas;
use crate::rendering::PixelRenderer;

#[component]
pub fn TestImageView() -> impl IntoView {
    let renderer = PixelRenderer::new(TestImageRenderer::new());
    view! { <InteractiveCanvas renderer=renderer /> }
}
```

**Step 2: Remove unused imports**

Remove all imports that are no longer needed (viewport, use_canvas_interaction, etc.). Keep only:

```rust
use crate::components::InteractiveCanvas;
use crate::rendering::{
    coords::{Coord, Rect},
    pixel_compute::PixelCompute,
    PixelRenderer,
};
use leptos::*;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Should compile now

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass (existing tests should still work)

**Step 5: Commit**

```bash
git add src/components/test_image.rs
git commit -m "refactor: simplify TestImageView to use InteractiveCanvas"
```

---

## Task 9: Clean up and remove dead code

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Remove all deleted tests that reference old API**

In `src/components/test_image.rs`, remove the `make_transform_result` helper and any tests that used the old direct viewport manipulation (tests from line ~245 onwards that test `apply_pixel_transform_to_viewport` directly).

These tests now belong at the rendering/transforms.rs level, not in the component.

**Step 2: Keep only renderer-specific tests**

Keep tests like:
- `test_renderer_natural_bounds`
- `test_renderer_produces_correct_pixel_count`
- `test_checkerboard_pattern_at_origin`
- `test_circle_at_radius_10`
- `test_origin_is_corner_of_four_squares`

**Step 3: Verify tests still pass**

Run: `cargo test --lib components::test_image`
Expected: Remaining tests pass

**Step 4: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 5: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings

**Step 6: Commit**

```bash
git add src/components/test_image.rs
git commit -m "cleanup: remove old test helpers and component-level viewport tests"
```

---

## Task 10: Run full test suite and build

**Files:**
- None (verification only)

**Step 1: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 2: Run WASM browser tests**

Run: `wasm-pack test --headless --chrome`
Expected: Browser tests pass

**Step 3: Build release**

Run: `trunk build --release`
Expected: Builds successfully

**Step 4: Manual browser testing**

1. Run: `trunk serve`
2. Open: http://localhost:8080
3. Verify: Test image renders with checkerboard and circles
4. Verify: Pan and zoom work correctly
5. Verify: Window resize works

**Step 5: Final commit if any fixes needed**

If you had to fix anything:
```bash
git add .
git commit -m "fix: address issues found in manual testing"
```

---

## Verification Checklist

After completing all tasks:

- [ ] All unit tests pass (`cargo test`)
- [ ] Browser tests pass (`wasm-pack test`)
- [ ] Clippy shows no warnings
- [ ] Code is formatted (`cargo fmt`)
- [ ] Release build succeeds
- [ ] Manual browser testing confirms pan/zoom/resize work
- [ ] Code follows 120-char line length
- [ ] No placeholders or TODOs in code
- [ ] All commits follow conventional commit format

---

## Next Steps (Future Work)

After this refactor is complete and tested:

1. **Add Mandelbrot renderer** - Implement `MandelbrotCompute` with `rug::Float` coordinates
2. **Add renderer selection UI** - Enum + match for switching renderers
3. **Optimize TiledRenderer** - Add parallelization, progress tracking
4. **Add GPU renderer** - WebGL-based renderer implementing same `Renderer` trait

---

## Notes

**Architecture achieved:**
- ✅ Separation of pixel-space (interactions) from image-space (rendering)
- ✅ Composable renderer wrappers (Tiled, Pixel)
- ✅ Generic canvas component reusable for any renderer
- ✅ Type-safe coordinate systems via associated types
- ✅ Clean trait boundaries with single responsibilities

**Lines of code reduction in test_image.rs:** ~130 lines → ~60 lines (canvas lifecycle moved to InteractiveCanvas)

**Extensibility:** Adding new renderer = implement PixelCompute trait (~30 lines) + 2-line view component
