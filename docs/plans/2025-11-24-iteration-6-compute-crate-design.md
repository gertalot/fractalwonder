# Iteration 6: Compute Crate & Renderer Trait Design

## Goal

Create the compute layer foundation with proper separation of computation from colorization.

**Key insight:** TestImage uses viewport-relative (normalized) coordinates, while Mandelbrot uses
fractal-space (BigFloat) coordinates. These are fundamentally different coordinate systems.

## Crate Structure

```
fractalwonder-core/     (existing + new types)
  └── compute_data.rs   NEW: TestImageData, ComputeData enum

fractalwonder-compute/  NEW CRATE
  ├── lib.rs
  ├── renderer.rs       Renderer trait
  ├── point_computer.rs ImagePointComputer trait (BigFloat, for Mandelbrot)
  └── renderers/
      └── test_image.rs TestImageRenderer (normalized coords)

fractalwonder-ui/       (existing + refactored)
  └── rendering/
      ├── colorizers/   NEW
      │   ├── mod.rs
      │   └── test_image.rs
      └── test_pattern.rs  (remove after migration)
```

## Dependencies

```
fractalwonder-core     → (none)
fractalwonder-compute  → fractalwonder-core
fractalwonder-ui       → fractalwonder-core (NOT compute)
```

The UI layer imports only types from core. It does not call compute functions directly.

## Core Types (fractalwonder-core)

### compute_data.rs

```rust
/// Data computed for a test image pixel.
/// All fields are bools derived from normalized coordinate comparisons.
#[derive(Clone, Debug)]
pub struct TestImageData {
    pub is_on_origin: bool,
    pub is_on_x_axis: bool,
    pub is_on_y_axis: bool,
    pub is_on_major_tick_x: bool,
    pub is_on_medium_tick_x: bool,
    pub is_on_minor_tick_x: bool,
    pub is_on_major_tick_y: bool,
    pub is_on_medium_tick_y: bool,
    pub is_on_minor_tick_y: bool,
    pub is_light_cell: bool,
}

/// Unified enum for all compute results.
#[derive(Clone, Debug)]
pub enum ComputeData {
    TestImage(TestImageData),
    // Mandelbrot(MandelbrotData),  // iteration 7
}
```

## Compute Layer (fractalwonder-compute)

### Renderer Trait

```rust
use fractalwonder_core::Viewport;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}
```

### ImagePointComputer Trait (for fractal-space renderers like Mandelbrot)

```rust
use fractalwonder_core::BigFloat;

/// Computes data for a single point in fractal space.
/// Used by fractals that operate in absolute fractal coordinates (e.g., Mandelbrot).
/// NOT used by TestImage (which uses normalized viewport coordinates).
pub trait ImagePointComputer {
    type Data;

    fn compute(&self, x: &BigFloat, y: &BigFloat) -> Self::Data;
}
```

### TestImageRenderer (normalized coordinates)

TestImage is fundamentally viewport-relative, not fractal-space. At 10^-1000 zoom, the
checkerboard cells would have 1000-digit indices in fractal space - impossible to handle.
Instead, we work in normalized viewport coordinates where spacing is always 0.2.

```rust
use fractalwonder_core::{Viewport, TestImageData};

pub struct TestImageRenderer;

impl Renderer for TestImageRenderer {
    type Data = TestImageData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<TestImageData> {
        let (width, height) = canvas_size;

        // Compute origin position in normalized coords (SAFE: result is small value)
        // origin_norm_x = (0 - center_x) / viewport_width
        let origin_norm_x = compute_origin_norm_x(viewport);
        let origin_norm_y = compute_origin_norm_y(viewport);

        (0..height).flat_map(|py| {
            (0..width).map(move |px| {
                // Pixel to normalized coords (pure f64 arithmetic)
                let norm_x = (px as f64 / width as f64) - 0.5;
                let norm_y = (py as f64 / height as f64) - 0.5;

                compute_test_image_data(norm_x, norm_y, origin_norm_x, origin_norm_y)
            })
        }).collect()
    }
}

/// Compute origin's normalized x position.
/// Returns (0 - viewport.center_x) / viewport.width as f64.
/// SAFE: Result is a small normalized value, not an extreme BigFloat.
fn compute_origin_norm_x(viewport: &Viewport) -> f64 {
    let zero = BigFloat::zero(viewport.center().0.precision_bits());
    let offset = zero.sub(&viewport.center().0);
    let normalized = offset.div(&viewport.width());
    normalized.to_f64()  // SAFE: normalized position is small
}

fn compute_test_image_data(
    norm_x: f64,
    norm_y: f64,
    origin_norm_x: f64,
    origin_norm_y: f64,
) -> TestImageData {
    // Position relative to origin
    let fx = norm_x - origin_norm_x;
    let fy = norm_y - origin_norm_y;

    // Fixed spacing in normalized coords
    const MAJOR_SPACING: f64 = 0.2;
    const AXIS_THRESHOLD: f64 = 0.003;
    const ORIGIN_THRESHOLD: f64 = 0.02;
    // ... other constants

    TestImageData {
        is_on_origin: origin_visible && dist_to_origin < ORIGIN_THRESHOLD,
        is_on_x_axis: y_axis_visible && fy.abs() < AXIS_THRESHOLD,
        // ... remaining fields (same logic as current test_pattern.rs)
    }
}
```

## UI Layer (fractalwonder-ui)

### Colorizer

```rust
// fractalwonder-ui/src/rendering/colorizers/test_image.rs

use fractalwonder_core::TestImageData;

pub type TestImageColorizer = fn(&TestImageData) -> [u8; 4];

const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255];
const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255];
const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255];
const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255];
const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255];

pub fn default_colorizer(data: &TestImageData) -> [u8; 4] {
    if data.is_on_origin { return ORIGIN_COLOR; }
    if data.is_on_major_tick_x || data.is_on_major_tick_y { return MAJOR_TICK_COLOR; }
    if data.is_on_medium_tick_x || data.is_on_medium_tick_y { return MEDIUM_TICK_COLOR; }
    if data.is_on_minor_tick_x || data.is_on_minor_tick_y { return MINOR_TICK_COLOR; }
    if data.is_on_x_axis || data.is_on_y_axis { return AXIS_COLOR; }
    if data.is_light_cell { BACKGROUND_LIGHT } else { BACKGROUND_DARK }
}
```

### Integration in InteractiveCanvas

```rust
// Simplified render loop
let computer = TestImageComputer::new(&viewport);
let data: Vec<TestImageData> = render_viewport(&computer, &viewport, canvas_size);

let colorizer = default_colorizer;
for (i, pixel_data) in data.iter().enumerate() {
    let color = colorizer(pixel_data);
    // write color to ImageData at position i
}
```

## Data Flow

**TestImage (normalized coordinates):**
```
Pixel (px, py)
    │
    ▼
Normalized coords: norm_x = px/width - 0.5  (pure f64)
    │
    ▼
Origin position: origin_norm = -center/width  (BigFloat→f64, SAFE: small value)
    │
    ▼
compute_test_image_data(norm_x, norm_y, origin_norm_x, origin_norm_y)
    │
    ▼
TestImageData { is_on_origin: bool, is_light_cell: bool, ... }
    │
    ▼
Colorizer(data) → [u8; 4]
    │
    ▼
Canvas ImageData
```

**Mandelbrot (BigFloat coordinates, iteration 7):**
```
Pixel (px, py)
    │
    ▼
pixel_to_fractal(px, py, viewport, canvas_size)
    │
    ▼
BigFloat (fx, fy)
    │
    ▼
MandelbrotComputer::compute(&fx, &fy)  (all math in BigFloat)
    │
    ▼
MandelbrotData { iterations: u32, escaped: bool }
    │
    ▼
Colorizer(data) → [u8; 4]
```

## Key Design Decisions

1. **TestImage uses normalized coordinates:** Viewport-relative, not fractal-space. Avoids impossible cell indices at extreme zoom.

2. **Safe `.to_f64()` for normalized positions:** Converting `(0 - center) / width` to f64 is safe because the result is a small normalized value.

3. **ImagePointComputer is for fractal-space renderers:** Mandelbrot uses it (BigFloat coords). TestImage doesn't (uses normalized coords directly).

4. **Data types in core:** Keeps UI independent of compute; both import shared types.

5. **No `natural_bounds()` on Renderer:** `FractalConfig` handles default viewports.

6. **Typed colorizers:** Each data type has its own colorizer signature.

## Testing

- `TestImageRenderer::render()` produces correct TestImageData for known viewport
- `compute_test_image_data()` matches current test_pattern_color_normalized() behavior
- Origin position calculation works at extreme zoom (origin far from viewport)
- Colorizer maps TestImageData to expected colors
- Visual output matches current test_pattern.rs exactly

## Migration

1. Create compute crate with Renderer trait and TestImageRenderer
2. Add TestImageData and ComputeData to core
3. Create colorizers in UI
4. Update InteractiveCanvas to use compute pipeline
5. Verify visual output matches
6. Remove old test_pattern.rs code
