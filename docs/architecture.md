# Fractal Wonder Architecture

This document describes the complete architecture for Fractal Wonder, a browser-based Mandelbrot set explorer
designed for extreme zoom levels (10^2000 and beyond).

## Core Design Principle

**All fractal-space computations use arbitrary precision arithmetic.** There is no "adaptive switching" between
f64 and BigFloat based on zoom level. The precision required is calculated from the zoom level, and all
computations use that precision from start to finish. BigFloat **automatically** uses f64 when precision bits ≤ 64,
but a core idea of this project is that precision bits must always be specified, to avoid accidental loss of precision.

## Crate Structure

```
fractalwonder/
├── fractalwonder-core/      # Mathematical types and coordinate transforms
├── fractalwonder-compute/   # Pure computation (runs in web workers)
└── fractalwonder-ui/        # Leptos app, canvas, worker orchestration
```

### Import Rules (Strictly Enforced)

| Crate | Can Import | Cannot Import |
|-------|------------|---------------|
| `fractalwonder-core` | (none) | compute, ui |
| `fractalwonder-compute` | core | ui |
| `fractalwonder-ui` | core | compute (see note) |

**Note:** The UI crate cannot directly call compute functions. All computation happens in web workers via
message passing. The UI imports only *types* from core (BigFloat, Viewport, etc.) for serialization.

---

## Coordinate Spaces

The system distinguishes two coordinate spaces:

| Space | Type | Description |
|-------|------|-------------|
| **Pixel Space** | `f64` | Canvas coordinates (0,0 = top-left). Used for UI events, canvas drawing. |
| **Fractal Space** | `BigFloat` | Mathematical coordinates in the fractal domain. Always arbitrary precision. |

### Key Types

```rust
// Pixel space (always f64 or u32)
pub struct PixelRect { x: u32, y: u32, width: u32, height: u32 }
pub struct PixelTransform { offset_x: f64, offset_y: f64, zoom_factor: f64, ... }

// Fractal space (always BigFloat)
pub struct Viewport {
    center: (BigFloat, BigFloat),
    width: BigFloat,
    height: BigFloat,
}
```

`Viewport` defines a rectangle in fractal space:
- `center`: The fractal-space coordinate at the rectangle's center
- `width`, `height`: The rectangle's dimensions in fractal space

**There is no "zoom" field.** At extreme depths, zoom factors like 10^2000 cannot fit in f64. Instead,
the width/height represent the visible region directly. Zooming in = dividing width/height.
At 10^2000× magnification, `width` might be `10^-2000` (stored as BigFloat).

---

## Precision Management

Precision is determined by the **pixel delta** - the smallest fractal-space distance between adjacent canvas pixels:

```
delta_x = viewport.width / canvas_width
delta_y = viewport.height / canvas_height
pixel_delta = min(delta_x, delta_y)
```

We need precision sufficient to distinguish adjacent pixels in **both** dimensions. The smaller delta is
harder to represent, so it determines the precision requirement.

At extreme zoom, this delta becomes extraordinarily small (e.g., 10^-2000). The precision must be sufficient
to represent and distinguish these tiny differences.

**Key insight:** There is no separate "zoom" value. The viewport's `width` and `height` (BigFloat) *are* the
zoom information. At 10^2000× magnification, the viewport dimensions might be `10^-2000` - far beyond f64 range.

### Precision Calculation (`precision.rs`)

The precision calculation function lives in `fractalwonder-core/src/precision.rs`:

```rust
/// Calculate required precision bits from viewport and canvas dimensions.
///
/// Returns the precision needed to distinguish adjacent pixels. This may be
/// higher than the viewport's current precision if the canvas is large.
pub fn calculate_required_precision(viewport: &Viewport, canvas_size: (u32, u32)) -> usize {
    // Pixel delta determines minimum distinguishable value
    // delta_x = viewport.width / canvas_width
    // delta_y = viewport.height / canvas_height
    // pixel_delta = min(delta_x, delta_y)
    //
    // For pixel_delta = 10^-N, we need ~N × 3.322 bits (log2(10) per decimal digit)
    // Plus safety margin for arithmetic operations
}
```

### Zoom Level Functions (`transforms.rs`)

The zoom level functions live in `fractalwonder-core/src/transforms.rs` alongside other
geometry/coordinate functions:

```rust
/// Calculate zoom level for UI display.
///
/// Compares current viewport width to a reference width (typically the fractal's
/// default viewport width). Returns (mantissa, exponent) where zoom ≈ mantissa × 10^exponent.
///
/// Example: At 10^2000× zoom, returns (1.0, 2000).
pub fn calculate_zoom_level(
    current_width: &BigFloat,
    reference_width: &BigFloat,
) -> (f64, i64) {
    // zoom = reference_width / current_width (BigFloat division)
    // Extract mantissa and base-10 exponent from result
}

/// Format zoom level as human-readable string for UI display.
///
/// Examples: "1×", "150×", "1.5 × 10^50", "10^2000"
pub fn format_zoom_level(
    current_width: &BigFloat,
    reference_width: &BigFloat,
) -> String {
    // Uses calculate_zoom_level internally
}
```

The viewport has its own precision (`viewport.precision_bits()`), but the **required** precision depends
on canvas size too. A larger canvas means more pixels to distinguish, potentially requiring higher precision
than the viewport currently uses. Before rendering, create a new viewport with the required precision.

| Pixel Delta | Approx. Precision Bits |
|-------------|------------------------|
| ~0.002 | 128 |
| ~10^-24 | 256 |
| ~10^-54 | 512 |
| ~10^-124 | 1024 |
| ~10^-2004 | 8192 |

**Critical:** The viewport width/height are BigFloat from the start. When the user zooms in, the width/height
shrink (division by zoom factor). Precision is recalculated as needed to maintain pixel-level accuracy.

---

## Core Module (`fractalwonder-core`)

Pure mathematical types with no rendering logic.

### BigFloat

Arbitrary precision floating point built on `dashu`:

```rust
pub struct BigFloat {
    value: BigFloatValue,      // F64 or Arbitrary(FBig)
    precision_bits: usize,
}

impl BigFloat {
    pub fn with_precision(val: f64, precision_bits: usize) -> Self;
    pub fn from_string(val: &str, precision_bits: usize) -> Result<Self, String>;
    pub fn zero(precision_bits: usize) -> Self;
    pub fn one(precision_bits: usize) -> Self;

    pub fn add(&self, other: &Self) -> Self;
    pub fn sub(&self, other: &Self) -> Self;
    pub fn mul(&self, other: &Self) -> Self;
    pub fn div(&self, other: &Self) -> Self;
    pub fn sqrt(&self) -> Self;

    pub fn to_f64(&self) -> f64;  // For display/colorization ONLY
    pub fn precision_bits(&self) -> usize;
}
```

**Important:** `BigFloat::to_f64()` is only used at the final colorization stage, never during computation.

### Viewport and Transforms

```rust
/// Viewport in fractal space - defines the visible region
pub struct Viewport {
    pub center: (BigFloat, BigFloat),
    pub width: BigFloat,
    pub height: BigFloat,
}

impl Viewport {
    pub fn with_bigfloat(center_x, center_y, width, height) -> Self;
    pub fn from_f64(center_x, center_y, width, height, precision_bits) -> Self;
    pub fn from_strings(center_x, center_y, width, height, precision_bits) -> Result<Self, String>;
    pub fn precision_bits(&self) -> usize;
}

/// Convert pixel coordinates to fractal coordinates
pub fn pixel_to_fractal(
    pixel_x: f64, pixel_y: f64,
    viewport: &Viewport,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> (BigFloat, BigFloat);

/// Convert fractal coordinates to pixel coordinates (for display only)
pub fn fractal_to_pixel(
    fractal_x: &BigFloat, fractal_y: &BigFloat,
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> (f64, f64);

/// Apply pixel-space transform to viewport (for interaction)
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport,
    transform: &PixelTransform,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> Viewport;
```

### Serialization

All core types implement `Serialize`/`Deserialize` for worker communication:

```rust
// BigFloat serializes as { value: String, precision_bits: usize }
// This preserves full precision through JSON
```

---

## Compute Module (`fractalwonder-compute`)

Pure computation logic that runs in web workers. Operates entirely in fractal space.

### Computation Data Types

Each fractal type produces its own data structure (NOT colors):

```rust
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
    // Future: smooth iteration count, orbit data, etc.
}

pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

/// Unified type for worker communication
pub enum ComputeData {
    Mandelbrot(MandelbrotData),
    TestImage(TestImageData),
}
```

### Renderer Trait

The core computation abstraction. Operates **purely in fractal space** - no pixel concepts:

```rust
pub trait Renderer: Clone + Send {
    type Data: Clone + Send + Serialize;

    /// Compute data for a grid of points in the given fractal-space region
    fn render(
        &self,
        region: &Viewport,       // Fractal-space rectangle to compute
        resolution: (u32, u32),  // Grid size: (width, height) points
    ) -> Vec<Self::Data>;
}
```

The renderer:
1. Receives a `region` (a Viewport defining a rectangle in fractal space)
2. Receives a `resolution` (how many points to compute: w × h)
3. Divides the region into a w × h grid of points
4. Computes each point
5. Returns w × h data values

The renderer knows nothing about pixels, canvases, or tiles. It just computes a grid of points
in fractal space. Each renderer implements its own computation strategy - simple renderers iterate
per-point; advanced renderers (perturbation theory) may compute reference orbits first.

### Renderer Registry

Workers are initialized with a renderer ID string. The compute layer has an internal registry
that maps IDs to renderer implementations:

```rust
/// Create a renderer by ID (called by worker on initialization)
pub fn create_renderer(renderer_id: &str) -> Option<Box<dyn Renderer<Data = ComputeData>>> {
    match renderer_id {
        "mandelbrot" => Some(Box::new(MandelbrotRenderer::new())),
        "test_image" => Some(Box::new(TestImageRenderer::new())),
        _ => None,
    }
}
```

### Renderer Implementations

**MandelbrotRenderer**
- Currently: simple per-pixel escape-time iteration
- Future: perturbation theory with reference orbits, series approximation for extreme zoom

**TestImageRenderer**
- Generates checkerboard pattern with circle for testing the rendering pipeline

### Worker Communication

Workers communicate with the main thread via serialized messages. Note that the compute layer
receives **pure fractal-space data** - all pixel-to-fractal conversion happens on the main thread.

```rust
/// Main thread → Worker
pub enum MainToWorker {
    Initialize { renderer_id: String },
    Render {
        render_id: u32,
        region: String,          // JSON-serialized Viewport (fractal-space region)
        resolution: (u32, u32),  // Grid size to compute
    },
    Cancel,
    Terminate,
}

/// Worker → Main thread
pub enum WorkerToMain {
    Ready,
    RenderComplete {
        render_id: u32,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    },
    Error { message: String },
}
```

The main thread is responsible for:
1. Dividing the canvas into tiles (PixelRects)
2. Converting each tile's pixel bounds to a fractal-space region (Viewport)
3. Sending the fractal region + resolution to workers
4. Receiving computed data and placing it at the correct canvas position

**Important:** The region is serialized as JSON string to preserve BigFloat precision. Workers deserialize
it with full precision intact.

---

## UI Module (`fractalwonder-ui`)

Leptos application running on the main thread.

### Colorizer

Converts computation data to RGBA pixels:

```rust
pub type Colorizer = fn(&ComputeData) -> (u8, u8, u8, u8);

pub struct ColorizerInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub colorizer: Colorizer,
}

/// Registry of colorizers per renderer type
pub static COLORIZERS: &[RendererColorizers] = &[
    RendererColorizers {
        renderer_id: "mandelbrot",
        colorizers: &[
            ColorizerInfo { id: "grayscale", ... },
            ColorizerInfo { id: "fire", ... },
            ColorizerInfo { id: "ocean", ... },
        ],
    },
    // ...
];
```

**Key insight:** Colorization happens on the main thread after computation completes. This allows:
- Re-colorizing cached data without recomputation
- Interactive color adjustment
- Separation of computation (expensive) from display (cheap)

### CanvasRenderer Trait

Abstraction for rendering computed data to an HTML canvas:

```rust
pub trait CanvasRenderer {
    /// Start rendering the viewport to the canvas
    fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement);

    /// Cancel in-progress rendering
    fn cancel(&self);

    /// Set colorizer (may re-colorize from cache without recomputing)
    fn set_colorizer(&mut self, colorizer: Colorizer);

    /// Progress signal for UI
    fn progress(&self) -> RwSignal<RenderProgress>;
}

pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}
```

### CanvasRenderer Implementations

Multiple implementations of `CanvasRenderer` exist for different orchestration strategies.
The `OrchestrationType` in `FractalConfig` determines which implementation is used.

#### SimpleTilingRenderer

For fractals that don't need advanced orchestration (test images, simple fractals):

```rust
pub struct SimpleTilingRenderer {
    renderer_id: String,
    worker_pool: WorkerPool,
    cache: RenderCache,
    colorizer: Colorizer,
    progress: RwSignal<RenderProgress>,
}
```

**Responsibilities:**
1. **Precision calculation:** Determine required precision bits from viewport and canvas size
2. **Tiling:** Divide canvas into tiles, order center-out for progressive display
3. **Coordinate conversion:** Convert pixel tiles to fractal-space regions for workers
4. **Worker dispatch:** Send regions to workers, collect results
5. **Caching:** Store computed data per tile for re-colorization
6. **Cancellation:** Terminate workers on user interaction

Tile size adapts to zoom level:
- Low zoom (< 10^10): 128×128 px tiles
- High zoom (≥ 10^10): 64×64 px tiles (more granular progress)

#### PerturbationRenderer

For deep Mandelbrot zoom using perturbation theory:

```rust
pub struct PerturbationRenderer {
    renderer_id: String,
    worker_pool: WorkerPool,
    reference_orbit_cache: ReferenceOrbitCache,
    tile_cache: RenderCache,
    colorizer: Colorizer,
    progress: RwSignal<RenderProgress>,
}
```

**Additional responsibilities beyond SimpleTilingRenderer:**
1. **Reference orbit computation:** Calculate high-precision reference orbits before tile dispatch
2. **Reference data distribution:** Send reference orbit data to workers along with tile regions
3. **Glitch detection:** Analyze returned results for perturbation failures
4. **Subdivision:** Split problem regions and compute additional reference points
5. **Precision management:** Handle different precision requirements for reference vs. delta calculations

The compute layer remains simple: workers receive a region (plus reference data), compute, return
results. All orchestration complexity lives in the renderer implementation.

### use_canvas_interaction Hook

Handles user interaction with zero-latency preview. **NOTE**: The current implementation is CORRECT already and does
NOT need further work.

```rust
pub fn use_canvas_interaction(
    canvas_ref: NodeRef<Canvas>,
    on_interaction_end: impl Fn(PixelTransform) + 'static,
) -> InteractionHandle;

pub struct InteractionHandle {
    pub is_interacting: Signal<bool>,
}
```

**Interaction flow:**
1. User starts drag/pinch → capture canvas `ImageData` snapshot
2. Build transformation sequence as `Vec<AffinePrimitive>` (Translate/Scale operations)
3. During interaction → render preview using Canvas2D `setTransform()` API with composed matrix
4. After 1.5s idle → compose transforms into `PixelMat3`, extract `PixelTransform`, fire callback
5. `on_interaction_end` calls `apply_pixel_transform_to_viewport()` to update the viewport

**Key types for interaction:**
```rust
/// Primitive 2D affine operations (pixel space)
pub enum AffinePrimitive {
    Translate { dx: f64, dy: f64 },
    Scale { factor: f64, center_x: f64, center_y: f64 },
}

/// 3x3 homogeneous transformation matrix
pub struct PixelMat3 { data: [[f64; 3]; 3] }

/// Final transform result with center-relative offsets
pub struct PixelTransform {
    pub offset_x: f64,      // Center-relative horizontal offset
    pub offset_y: f64,      // Center-relative vertical offset
    pub zoom_factor: f64,   // Cumulative zoom (1.0 = no change)
    pub matrix: [[f64; 3]; 3],
}
```

The preview rendering uses `context.setTransform()` to apply the composed matrix to the Canvas2D
context, then draws the captured `ImageData` through that transform. This provides instant visual
feedback without recomputing pixels.

### InteractiveCanvas Component

Combines canvas, interaction hook, and renderer:

```rust
#[component]
pub fn InteractiveCanvas(
    renderer: Box<dyn CanvasRenderer>,
    viewport: RwSignal<Viewport<BigFloat>>,
) -> impl IntoView;
```

Responsibilities:
- Mount HTML canvas element
- Attach interaction hook
- Cancel render on interaction start
- Apply viewport transform on interaction end
- Trigger re-render when viewport changes

### App Component

Top-level application state and UI:

```rust
#[component]
pub fn App() -> impl IntoView;
```

State managed:
- Current fractal type (TestImage, Mandelbrot)
- Current colorizer
- Viewport (persisted to localStorage)
- UI visibility (auto-hide during interaction)

---

## Configuration

Configuration lives in the **UI layer** and connects to renderers by **string ID**. The UI never
imports or references compute types directly.

Two independent axes determine rendering behavior:
- **`renderer_id`**: Tells workers *what* to compute (sent to compute layer)
- **`orchestration`**: Tells main thread *how* to orchestrate (selects CanvasRenderer implementation)

```rust
/// UI-layer configuration for a fractal type
pub struct FractalConfig {
    pub id: &'static str,              // Must match renderer registry in compute layer
    pub display_name: &'static str,
    pub description: &'static str,
    pub orchestration: OrchestrationType,
    /// Default viewport center and dimensions as string literals.
    /// Strings preserve precision and are parsed to BigFloat when creating Viewport.
    /// This avoids f64 precision loss for fractal-space coordinates.
    pub default_center: (&'static str, &'static str),
    pub default_width: &'static str,
    pub default_height: &'static str,
    pub colorizers: &'static [ColorizerInfo],
}

impl FractalConfig {
    /// Create the default Viewport for this fractal type at the given precision.
    pub fn default_viewport(&self, precision_bits: usize) -> Viewport {
        Viewport::from_strings(
            self.default_center.0,
            self.default_center.1,
            self.default_width,
            self.default_height,
            precision_bits,
        ).expect("Invalid default viewport coordinates in FractalConfig")
    }
}

/// Determines which CanvasRenderer implementation to use
pub enum OrchestrationType {
    /// Simple tile queue: dispatch tiles, collect results, colorize, draw.
    /// Used for test images and simple fractals.
    SimpleTiling,

    /// Perturbation theory: compute reference orbits, dispatch tiles with reference
    /// data, detect glitches, subdivide problem regions, retry.
    /// Used for deep Mandelbrot zoom.
    Perturbation,
}

pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        description: "Classic Mandelbrot fractal",
        orchestration: OrchestrationType::Perturbation,
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "3.0",
        colorizers: &[/* grayscale, fire, ocean, ... */],
    },
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        description: "Checkerboard with circle for testing",
        orchestration: OrchestrationType::SimpleTiling,
        default_center: ("0.0", "0.0"),
        default_width: "2.0",
        default_height: "1.5",
        colorizers: &[/* default, pastel, ... */],
    },
];
```

### CanvasRenderer Factory

When the user selects a fractal type, the UI creates the appropriate `CanvasRenderer`:

```rust
pub fn create_canvas_renderer(
    config: &FractalConfig,
    colorizer: Colorizer,
) -> Box<dyn CanvasRenderer> {
    match config.orchestration {
        OrchestrationType::SimpleTiling => {
            Box::new(SimpleTilingRenderer::new(config.id, colorizer))
        }
        OrchestrationType::Perturbation => {
            Box::new(PerturbationRenderer::new(config.id, colorizer))
        }
    }
}
```

The `CanvasRenderer` trait provides a stable interface; implementations vary based on orchestration
needs. `SimpleTilingRenderer` just queues tiles and dispatches. `PerturbationRenderer` handles
reference orbits, glitch detection, and subdivision.

**Key point:** The `id` field is the contract between UI and compute layers. The UI sends this ID
when initializing workers; the compute layer's `create_renderer(id)` returns the corresponding
renderer implementation.

---

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ MAIN THREAD (fractalwonder-ui)                                              │
│                                                                             │
│  User Interaction (drag/pinch/scroll)                                       │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────┐                                                    │
│  │ use_canvas_interaction │                                                 │
│  │  • Capture ImageData │                                                   │
│  │  • Build Vec<AffinePrimitive>                                            │
│  │  • Preview via Canvas2D setTransform()                                   │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       │ on_interaction_end(PixelTransform)                                  │
│       ▼                                                                     │
│  ┌─────────────────────┐                                                    │
│  │ apply_pixel_transform │──► Viewport (full precision BigFloat)            │
│  │ _to_viewport         │                                                   │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────┐               │
│  │ TilingCanvasRenderer                                    │               │
│  │  1. Divide canvas into tiles (PixelRects)               │               │
│  │  2. For each tile: pixel_rect_to_viewport() ──►         │               │
│  │     fractal-space region (Viewport) + resolution        │               │
│  │  3. Send to WorkerPool                                  │               │
│  └─────────────────────────────────────────────────────────┘               │
│       │                                                                     │
│       │ MainToWorker::Render { region, resolution }                         │
│       │ (region as JSON string - pure fractal space)                        │
└───────│─────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ WEB WORKERS (fractalwonder-compute) - PURE FRACTAL SPACE                    │
│                                                                             │
│  ┌─────────────────────┐                                                    │
│  │ Deserialize region  │                                                    │
│  │ (Viewport)          │                                                    │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────┐                                                    │
│  │ Renderer.render()   │                                                    │
│  │  • region: Viewport │                                                    │
│  │  • resolution: (w,h)│                                                    │
│  │  • Divide region    │                                                    │
│  │    into w×h grid    │                                                    │
│  │  • Compute each pt  │                                                    │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       │ WorkerToMain::RenderComplete { data: Vec<ComputeData> }             │
└───────│─────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ MAIN THREAD (continued)                                                     │
│                                                                             │
│  ┌─────────────────────┐                                                    │
│  │ Cache data for tile │  (knows which PixelRect this data belongs to)      │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────┐                                                    │
│  │ Colorizer           │──► (u8, u8, u8, u8) per point                      │
│  │ fn(&Data) → RGBA    │                                                    │
│  └─────────────────────┘                                                    │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────┐                                                    │
│  │ canvas.putImageData │──► Pixels on screen at correct tile position       │
│  └─────────────────────┘                                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Archived Code Reference

The `_archive/` directory contains a previous implementation with valuable patterns but flawed precision
handling. Use it for inspiration, but do not copy code without verifying precision correctness.

| Archive Location | Pattern to Reuse | Precision Issue |
|------------------|------------------|-----------------|
| `_archive/fractalwonder-ui/src/hooks/use_canvas_interaction.rs` | Transform composition, gesture handling | None (pixel-space only) |
| `_archive/fractalwonder-compute/src/pixel_renderer.rs` | Tile iteration pattern | Uses generic Scalar, needs BigFloat enforcement |
| `_archive/fractalwonder-compute/src/precision.rs` | Precision calculation formula | Good, keep as-is |
| `_archive/fractalwonder-ui/src/rendering/colorizers.rs` | Colorizer registry pattern | None (operates on computed data) |
| `_archive/fractalwonder-compute/src/messages.rs` | Worker message protocol | Good, viewport now uses JSON string |
| `_archive/fractalwonder-core/src/transforms.rs` | Coordinate conversion math | Needs review for BigFloat usage |

---

## Implementation Order

1. **Core types** - BigFloat (done), Viewport, Point, Rect, PixelRect
2. **Coordinate transforms** - pixel_to_fractal, fractal_to_pixel (with BigFloat)
3. **PointComputer trait** - Define interface
4. **TestImageComputer** - Simple implementation for testing
5. **Worker infrastructure** - Messages, worker entry point
6. **TileRenderer** - PixelTileRenderer wrapping PointComputer
7. **WorkerPool** - Thread management, message routing
8. **CanvasRenderer** - TilingCanvasRenderer with cache
9. **use_canvas_interaction** - Interaction handling
10. **InteractiveCanvas** - Component assembly
11. **App** - Full application
12. **MandelbrotComputer** - Production fractal computation

---

## Testing Strategy

| Layer | Test Type | What to Verify |
|-------|-----------|----------------|
| BigFloat | Unit | Arithmetic at extreme precision (10^500) |
| Coordinate transforms | Unit | Round-trip: pixel → fractal → pixel |
| PointComputer | Unit | Known values (e.g., origin is in Mandelbrot set) |
| TileRenderer | Unit | Correct pixel count, data ordering |
| Worker messages | Unit | Serialization round-trip preserves precision |
| CanvasRenderer | Integration | Correct pixels rendered to canvas |
| Interaction | E2E | Pan/zoom produces correct viewport |
