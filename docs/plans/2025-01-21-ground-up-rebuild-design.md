# Ground-Up Rebuild: Extreme-Precision Mandelbrot Explorer

**Date:** 2025-01-21
**Goal:** Build from scratch with extreme precision (10^2000+ zoom) as the foundation

## Core Principles

### 1. Strict Precision Enforcement
- ALL fractal-space coordinates use `BigFloat` with **explicit precision**
- ALL pixel-space coordinates use `f64`
- **NO implicit conversions or defaults anywhere**
- BigFloat internally uses f64 when `precision_bits ≤ 64`, FBig otherwise (transparent to external code)

### 2. Clean Separation of Concerns
- **Computation Layer**: Produces raw data (iteration counts, magnitudes) - no colors
- **Colorization Layer**: Transforms data → RGB values (main thread only)
- **Rendering Layer**: Pushes RGB to canvas pixels (main thread only)

### 3. Worker Architecture
- **Main thread**: UI, canvas interaction, colorization, rendering RGB to canvas
- **Web Workers**: Compute-heavy Mandelbrot iteration using BigFloat throughout
- **Message passing**: Serialized BigFloat values across worker boundary

## Architecture

### BigFloat Implementation

**Structure (enum-based for transparent f64/FBig switching):**
```rust
pub struct BigFloat {
    value: BigFloatValue,
    precision_bits: usize,
}

enum BigFloatValue {
    F64(f64),           // When precision_bits <= 64
    Arbitrary(FBig),    // When precision_bits > 64
}
```

**API - NO DEFAULTS:**
```rust
// Creation - ALWAYS requires explicit precision
pub fn zero(precision_bits: usize) -> Self;
pub fn one(precision_bits: usize) -> Self;
pub fn with_precision(value: f64, precision_bits: usize) -> Self;

// Arithmetic - preserves max precision from operands
pub fn add(&self, other: &Self) -> Self;
pub fn sub(&self, other: &Self) -> Self;
pub fn mul(&self, other: &Self) -> Self;
pub fn div(&self, other: &Self) -> Self;
pub fn sqrt(&self) -> Self;

// Comparison - PartialOrd trait
impl PartialOrd for BigFloat;

// Query
pub fn precision_bits(&self) -> usize;
pub fn to_f64(&self) -> f64;  // For display/colorization only

// Serialization for worker messages
impl Serialize + Deserialize for BigFloat;
```

**Critical rules:**
- NO `From<f64>` trait (forces explicit precision everywhere)
- NO `Default` trait
- Operations take max precision from operands
- Internally dispatches to f64 or FBig based on precision_bits

### Core Data Types

**Mandelbrot Computation Result:**
```rust
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
    pub z_magnitude: BigFloat,  // Full precision for extreme zooms
}
```

**Viewport (fractal-space coordinates):**
```rust
pub struct Viewport<T> {
    pub center: (T, T),      // Fractal coordinates
    pub width: T,            // Fractal-space width (can be ~10^-2000 for extreme zoom)
    pub height: T,           // Fractal-space height
}
```

**Why width/height instead of zoom factor?**
- At 10^2000 zoom, we're viewing a region of width ~10^-2000 in fractal space
- f64 cannot represent 10^2000 (max ~10^308), but BigFloat can represent 10^-2000
- Width/height directly describe the visible rectangle - no conversion needed
- Simplifies coordinate transformations (no need for `natural_bounds` parameter)

**Pixel Rectangle (pixel-space coordinates):**
```rust
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
```

### Coordinate Transformations

```rust
// Pixel (f64) → Fractal (BigFloat) - requires explicit precision
fn pixel_to_fractal(
    pixel_x: f64,
    pixel_y: f64,
    viewport: &Viewport<BigFloat>,
    canvas_size: (u32, u32),
    precision_bits: usize,  // EXPLICIT precision for result
) -> (BigFloat, BigFloat)

// Fractal (BigFloat) → Pixel (f64) - precision loss acceptable here
fn fractal_to_pixel(
    fractal_x: &BigFloat,
    fractal_y: &BigFloat,
    viewport: &Viewport<BigFloat>,
    canvas_size: (u32, u32),
) -> (f64, f64)

// Apply user interaction transform to viewport
// TransformResult comes from use_canvas_interaction hook
fn apply_pixel_transform_to_viewport(
    viewport: &Viewport<BigFloat>,
    transform: &TransformResult,  // Contains zoom_factor (relative), offsets
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> Viewport<BigFloat> {
    // Key transformations:
    // 1. new_width = old_width / transform.zoom_factor
    // 2. new_height = old_height / transform.zoom_factor
    // 3. Calculate new center from pixel offsets + zoom point
    //
    // Note: zoom_factor is RELATIVE (f64 is fine)
    //   zoom_factor = 2.0 means "zoom in 2x from current"
    //   Even at 10^2000 absolute zoom, the relative step fits in f64
}
```

**Why TransformResult.zoom_factor can be f64:**
- `zoom_factor` is RELATIVE to current zoom, not absolute
- Example: At 10^2000 zoom, user zooms 2x → `zoom_factor = 2.0` (fits in f64)
- We apply: `new_width = old_width / 2.0` where `old_width` is BigFloat
- The absolute zoom level (10^2000) is implicit in the width/height values

### Worker-Side Architecture

**PointComputer Trait (computes single fractal point):**
```rust
pub trait PointComputer {
    type Data: Clone + Send;
    type Config: Clone + Send;

    fn configure(&mut self, config: Self::Config);
    fn precision_bits(&self) -> usize;
    fn compute(&self, point: (BigFloat, BigFloat)) -> Self::Data;
}
```

**MandelbrotComputer:**
```rust
pub struct MandelbrotComputer {
    max_iterations: u32,
    precision_bits: usize,
}

pub struct MandelbrotConfig {
    pub max_iterations: u32,
    pub precision_bits: usize,
}

impl PointComputer for MandelbrotComputer {
    type Data = MandelbrotData;
    type Config = MandelbrotConfig;

    fn configure(&mut self, config: Self::Config) {
        self.max_iterations = config.max_iterations;
        self.precision_bits = config.precision_bits;
    }

    fn precision_bits(&self) -> usize {
        self.precision_bits
    }

    fn compute(&self, c: (BigFloat, BigFloat)) -> MandelbrotData {
        // z = 0
        let mut z = (
            BigFloat::zero(self.precision_bits),
            BigFloat::zero(self.precision_bits)
        );

        let mut iterations = 0;

        for _ in 0..self.max_iterations {
            // z = z^2 + c (complex arithmetic)
            let z_real = z.0.mul(&z.0)
                .sub(&z.1.mul(&z.1))
                .add(&c.0);

            let two = BigFloat::with_precision(2.0, self.precision_bits);
            let z_imag = two.mul(&z.0)
                .mul(&z.1)
                .add(&c.1);

            z = (z_real, z_imag);

            // Check escape: |z|² > 4
            let magnitude_squared = z.0.mul(&z.0).add(&z.1.mul(&z.1));
            let threshold = BigFloat::with_precision(4.0, self.precision_bits);

            if magnitude_squared > threshold {
                let magnitude = magnitude_squared.sqrt();
                return MandelbrotData {
                    iterations,
                    escaped: true,
                    z_magnitude: magnitude,
                };
            }

            iterations += 1;
        }

        // Didn't escape
        let magnitude_squared = z.0.mul(&z.0).add(&z.1.mul(&z.1));
        let magnitude = magnitude_squared.sqrt();

        MandelbrotData {
            iterations,
            escaped: false,
            z_magnitude: magnitude,
        }
    }
}
```

**PixelRenderer (loops over pixels, delegates to PointComputer):**
```rust
pub struct PixelRenderer<C: PointComputer> {
    computer: C,
}

impl<C: PointComputer> PixelRenderer<C> {
    pub fn configure(&mut self, config: C::Config) {
        self.computer.configure(config);
    }

    pub fn render(
        &self,
        viewport: &Viewport<BigFloat>,
        canvas_size: (u32, u32),
        pixel_rect: PixelRect,
    ) -> Vec<C::Data> {
        let mut results = Vec::new();

        for y in pixel_rect.y..(pixel_rect.y + pixel_rect.height) {
            for x in pixel_rect.x..(pixel_rect.x + pixel_rect.width) {
                let point = pixel_to_fractal(
                    x as f64,
                    y as f64,
                    viewport,
                    canvas_size,
                    self.computer.precision_bits(),
                );

                let data = self.computer.compute(point);
                results.push(data);
            }
        }

        results
    }
}
```

**Worker Messages:**
```rust
// Main → Worker
struct RenderRequest {
    viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    pixel_rect: PixelRect,
    config: MandelbrotConfig,  // max_iterations, precision_bits
}

// Worker → Main
struct RenderResponse {
    pixel_rect: PixelRect,
    data: Vec<MandelbrotData>,  // Contains BigFloat z_magnitude
}
```

**Worker Flow:**
1. Worker startup: Create `PixelRenderer<MandelbrotComputer>`
2. Receive `RenderRequest`: Configure computer, then render tile
3. Send back `RenderResponse` with serialized BigFloat values

### Main-Thread Architecture

**CanvasRenderer Trait (main thread only):**
```rust
pub trait CanvasRenderer {
    fn render(&self, viewport: &Viewport<BigFloat>, canvas: &HtmlCanvasElement);
    fn set_colorizer(&mut self, colorizer: Colorizer);
    fn cancel_render(&self);
    fn progress(&self) -> RwSignal<RenderProgress>;
}
```

**ParallelCanvasRenderer:**
- Manages worker pool
- Divides canvas into tiles (center-out ordering)
- Sends `RenderRequest` messages to workers
- Stores received `Vec<MandelbrotData>` in cache
- Applies colorizer: `MandelbrotData` → RGB
- Pushes RGB to canvas via `ImageData` (progressive rendering)

**Colorizer:**
```rust
pub type Colorizer = fn(&MandelbrotData) -> (u8, u8, u8, u8);

pub fn smooth_mandelbrot_colorizer(data: &MandelbrotData) -> (u8, u8, u8, u8) {
    if !data.escaped {
        return (0, 0, 0, 255);  // Black for inside set
    }

    // Smooth coloring: nsmooth = n + 1 - log(log(|z|)) / log(2)
    let magnitude_f64 = data.z_magnitude.to_f64();
    let smooth_value = data.iterations as f64
        + 1.0
        - magnitude_f64.ln().ln() / 2.0_f64.ln();

    // Map to HSV color space (cyclic)
    let hue = (smooth_value * 0.1) % 1.0;
    hsv_to_rgb(hue, 0.8, 0.9)
}
```

**InteractiveCanvas Component:**
- Owns `HtmlCanvasElement`
- Manages `Viewport<BigFloat>` state
- Uses `use_canvas_interaction` hook for pan/zoom gestures
- Owns `ParallelCanvasRenderer`
- On interaction end: triggers new render with updated viewport

## Project Structure

```
fractalwonder/
├── _archive/                    # All existing code moved here
├── Cargo.toml                   # Workspace definition
├── fractalwonder-core/          # Shared types
│   └── src/
│       ├── bigfloat.rs         # BigFloat with strict precision
│       ├── viewport.rs         # Viewport<BigFloat>
│       ├── pixel_rect.rs       # PixelRect
│       └── transforms.rs       # Coordinate conversions
│
├── fractalwonder-compute/       # Compute engine (workers)
│   └── src/
│       ├── point_computer.rs   # PointComputer trait
│       ├── pixel_renderer.rs   # PixelRenderer<C>
│       ├── mandelbrot.rs       # MandelbrotComputer
│       └── worker.rs           # Worker message loop
│
└── fractalwonder-ui/            # UI (main thread)
    └── src/
        ├── app.rs              # Root Leptos app
        ├── components/
        │   ├── interactive_canvas.rs
        │   └── ui.rs           # Reuse existing UI panel
        ├── hooks/
        │   ├── use_canvas_interaction.rs  # Reuse
        │   └── fullscreen.rs   # Reuse
        └── rendering/
            ├── canvas_renderer.rs
            ├── parallel_canvas_renderer.rs
            └── colorizers.rs
```

## Implementation Phases

### Stage 0: Basic Web App Skeleton
**Goal:** Validate basic plumbing works

- Leptos app with InteractiveCanvas component
- Reuse `use_canvas_interaction` hook
- Draw simple test pattern directly to canvas (no workers, no compute)
- On interaction end: redraw same test pattern (no transformations)
- **Validates:** Web app + Trunk + interaction hooks work

### Stage 1: Full Implementation
**Goal:** Complete extreme-precision Mandelbrot explorer

1. **BigFloat implementation** (fractalwonder-core)
   - Enum-based (F64/Arbitrary)
   - All methods require explicit precision
   - Serialization/deserialization for worker messages
   - Comprehensive tests for all operations
   - NO defaults anywhere

2. **Basic coordinate transforms** (fractalwonder-core)
   - `pixel_to_fractal()` with explicit precision parameter
   - `Viewport<BigFloat>` type
   - Tests for coordinate conversions

3. **Mandelbrot computation** (fractalwonder-compute)
   - `MandelbrotComputer` with `configure()`
   - `MandelbrotData` struct
   - Fixed iteration count (e.g., 256)
   - Tests for known points (inside/outside set)

4. **Worker-based rendering** (fractalwonder-ui + fractalwonder-compute)
   - Worker pool management
   - Message passing with serialized BigFloat
   - Tile-based rendering
   - ParallelCanvasRenderer with colorization
   - Progress indicator

5. **UI with interaction** (fractalwonder-ui)
   - Reuse existing UI panel components
   - Reuse `use_canvas_interaction` hook
   - Reuse fullscreen functionality
   - Basic smooth colorizer
   - Can pan/zoom and see Mandelbrot

**Critical test:** Send `Viewport<BigFloat>` and receive `Vec<MandelbrotData>` (with BigFloat z_magnitude) across worker boundary - validates entire serialization pipeline.

## Design Rationale

### Why Enum-Based BigFloat?
- Transparent optimization: f64 for low precision, FBig for high precision
- External code sees only `BigFloat` - no implementation leakage
- Performance where possible, precision where needed

### Why No Default Precision?
- Silent precision loss is the enemy of extreme zoom
- Every BigFloat creation point must explicitly consider precision needs
- Compiler enforces correctness via required parameters

### Why Separate Computation and Colorization?
- Re-color without re-computing (cache `Vec<MandelbrotData>`)
- Worker sends raw data, not RGB pixels (smaller messages)
- Colorizer runs on main thread (fast, no serialization)

### Why Configure Pattern for PointComputer?
- Different fractals need different parameters
- Workers long-lived, parameters change per render
- Extensible: Julia sets, other fractals with custom configs

### Why Store z_magnitude as BigFloat?
- At extreme zoom (10^-308+), f64 underflows to 0
- `log(0)` is undefined, breaks smooth coloring
- Convert to f64 only in colorizer (validated safe at that point)

## Testing Strategy

### Unit Tests
- BigFloat arithmetic at various precision levels
- Coordinate transformations (pixel ↔ fractal)
- Known Mandelbrot points (inside/outside set)
- Serialization round-trips for BigFloat

### Integration Tests
- Worker message passing with BigFloat
- Full render pipeline: viewport → tiles → data → colors → canvas
- Viewport transformations match user gestures

### Manual Testing
- Pan/zoom smoothness
- Progressive rendering updates
- Deep zoom (verify precision maintained)
- Color smoothness (no banding)

## Future Extensions

- Dynamic iteration count based on zoom level
- Dynamic precision_bits based on zoom level
- Perturbation theory for extreme zooms (10^100+)
- Multiple coloring schemes
- Julia set explorer
- URL sharing (serialize viewport)
- Save/load bookmarks
