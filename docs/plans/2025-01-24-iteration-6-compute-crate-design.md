# Iteration 6: Compute Crate & Renderer Trait Design

## Goal

Create the compute layer foundation with proper separation of computation from colorization. All fractal-space computations use BigFloat with no `.to_f64()` conversions in the compute layer.

## Crate Structure

```
fractalwonder-core/     (existing + new types)
  └── compute_data.rs   NEW: TestImageData, ComputeData enum

fractalwonder-compute/  NEW CRATE
  ├── lib.rs
  ├── renderer.rs       Renderer trait
  ├── point_computer.rs ImagePointComputer trait
  ├── pixel_renderer.rs render_viewport() function
  └── computers/
      └── test_image.rs TestImageComputer

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
/// All fields are bools derived from BigFloat comparisons.
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

### ImagePointComputer Trait

```rust
use fractalwonder_core::BigFloat;

/// Computes data for a single point in fractal space.
pub trait ImagePointComputer {
    type Data;

    /// Compute data for point (x, y) in fractal space.
    /// Coordinates are BigFloat - no .to_f64() allowed.
    fn compute(&self, x: &BigFloat, y: &BigFloat) -> Self::Data;
}
```

### Renderer Trait

```rust
use fractalwonder_core::Viewport;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}
```

### render_viewport Function

```rust
use fractalwonder_core::{Viewport, pixel_to_fractal};

pub fn render_viewport<C: ImagePointComputer>(
    computer: &C,
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> Vec<C::Data> {
    let (width, height) = canvas_size;
    (0..height).flat_map(|py| {
        (0..width).map(move |px| {
            let (fx, fy) = pixel_to_fractal(px as f64, py as f64, viewport, canvas_size);
            computer.compute(&fx, &fy)
        })
    }).collect()
}
```

### TestImageComputer

```rust
use fractalwonder_core::{BigFloat, TestImageData};

pub struct TestImageComputer {
    // Thresholds as BigFloat for comparison
    origin_threshold: BigFloat,
    axis_threshold: BigFloat,
    major_spacing: BigFloat,
    major_threshold: BigFloat,
    medium_spacing: BigFloat,
    medium_threshold: BigFloat,
    minor_spacing: BigFloat,
    minor_threshold: BigFloat,
    major_tick_length: BigFloat,
    medium_tick_length: BigFloat,
    minor_tick_length: BigFloat,
}

impl TestImageComputer {
    pub fn new(viewport: &Viewport) -> Self {
        // Derive thresholds from viewport dimensions
        // All values as BigFloat
    }
}

impl ImagePointComputer for TestImageComputer {
    type Data = TestImageData;

    fn compute(&self, x: &BigFloat, y: &BigFloat) -> TestImageData {
        // All comparisons in BigFloat, return bools
        let x_abs = x.abs();
        let y_abs = y.abs();

        TestImageData {
            is_on_origin: &x_abs < &self.origin_threshold
                       && &y_abs < &self.origin_threshold,
            is_on_x_axis: &y_abs < &self.axis_threshold,
            is_on_y_axis: &x_abs < &self.axis_threshold,
            is_on_major_tick_x: is_near_grid_line(x, &self.major_spacing, &self.major_threshold)
                             && &y_abs < &self.major_tick_length,
            // ... remaining fields
            is_light_cell: is_light_cell_bf(x, y, &self.major_spacing),
        }
    }
}

/// Check if value is near a multiple of spacing.
fn is_near_grid_line(value: &BigFloat, spacing: &BigFloat, threshold: &BigFloat) -> bool {
    let remainder = value.rem_euclid(spacing);
    let distance = remainder.min(&(spacing.sub(&remainder)));
    &distance < threshold
}

/// Determine checkerboard cell color.
fn is_light_cell_bf(x: &BigFloat, y: &BigFloat, spacing: &BigFloat) -> bool {
    let cell_x = x.div(spacing).floor_to_i64();
    let cell_y = y.div(spacing).floor_to_i64();
    (cell_x + cell_y) % 2 == 0
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
ImagePointComputer::compute(&fx, &fy)
    │  (all comparisons in BigFloat)
    ▼
TestImageData { is_on_origin: bool, is_light_cell: bool, ... }
    │
    ▼
Colorizer(data) → [u8; 4]
    │
    ▼
Canvas ImageData
```

## Key Design Decisions

1. **No `.to_f64()` in compute layer:** All fractal-space math uses BigFloat.

2. **Bool-based data:** TestImageData contains bools from BigFloat comparisons. No floating-point values that could lose precision.

3. **Composed architecture:** `ImagePointComputer` computes single points; `render_viewport()` handles the pixel loop.

4. **Data types in core:** Keeps UI independent of compute; both import shared types.

5. **No `natural_bounds()` on Renderer:** `FractalConfig` handles default viewports.

6. **Typed colorizers:** Each data type has its own colorizer signature. The render loop matches on `ComputeData` to call the appropriate colorizer.

## Testing

- `TestImageComputer::compute()` returns correct bools for known BigFloat inputs
- `is_near_grid_line()` works at extreme precision
- `is_light_cell_bf()` produces consistent checkerboard
- Colorizer maps TestImageData to expected colors
- Round-trip: same visual output as current test_pattern.rs

## Migration

1. Create compute crate with traits and TestImageComputer
2. Create colorizers in UI
3. Update InteractiveCanvas to use compute pipeline
4. Verify visual output matches
5. Remove old test_pattern.rs code
