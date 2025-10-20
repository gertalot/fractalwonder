# Fractal Wonder

A high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels (up to 10^100 and
beyond) with interactive real-time exploration. Built entirely in Rust using Leptos and WebAssembly.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.80 or later**: Install via [rustup](https://rustup.rs/)

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **wasm32-unknown-unknown target**: Required for WebAssembly compilation

  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- **Trunk**: Build tool for Rust WASM applications

  ```bash
  cargo install trunk
  ```

- **Node.js and npm** (optional): Only needed if you want to modify Tailwind CSS configuration
  - Trunk has built-in Tailwind CSS support via the standalone CLI

## Development

### Clone and Run

```bash
# Clone the repository
git clone <repository-url>
cd fractalwonder

# Start the development server with hot-reload
trunk serve

# Or start and automatically open in browser
trunk serve --open
```

The development server runs on `http://localhost:8080` with automatic hot-reload enabled. Changes to Rust files will
trigger automatic recompilation and browser refresh.

### Development Modes

```bash
# Development mode (default, with debug info)
trunk serve

# Development with optimized builds (faster runtime, slower compilation)
trunk serve --release
```

## Testing

```bash
# format code
cargo fmt --all

# lint code
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all

# check for compile/build errors
cargo check --workspace --all-targets --all-features

# Run all unit tests
cargo test

# Run tests with console output visible
cargo test -- --nocapture

# Run tests for a specific module
cargo test components::

# WASM-specific browser tests (when available)
wasm-pack test --headless --chrome
```

## Building

### Production Build

```bash
# Build optimized release version
trunk build --release
```

Output files are generated in the `dist/` directory and are ready to deploy to any static hosting service.

### Serving Production Build Locally

The production build requires specific HTTP headers for SharedArrayBuffer support (needed for multi-threading):

```bash
# Example using Python's http.server with custom headers
cd dist
python3 -m http.server 8080
```

**Note**: For proper SharedArrayBuffer support in production, your server must send:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`

Trunk's dev server includes these headers automatically.

## Architecture

Fractal Wonder's rendering architecture is built on **composable, trait-based abstractions** that enable extreme precision and clean separation of concerns.

### Coordinate Spaces

The system distinguishes between two coordinate spaces:

- **Pixel Space**: Screen coordinates in `u32` units representing actual canvas pixels
- **Image Space**: Mathematical coordinates using a generic type `T` (can be `f64`, `rug::Float`, or any numeric type)

This separation enables arbitrary precision rendering at extreme zoom levels (10^100+) by simply swapping the coordinate type without changing the rendering logic.

### Core Traits

The rendering architecture is built from composable traits that form a hierarchy of abstractions:

#### `Renderer<T>` - The Core Abstraction
The fundamental trait for rendering pixel data given a viewport and pixel-space dimensions.

```rust
pub trait Renderer {
    type Coord;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
    fn render(&self, viewport: &Viewport<Self::Coord>,
              pixel_rect: PixelRect,
              canvas_size: (u32, u32)) -> Vec<u8>;
}
```

- Generic over image-space coordinate type (`Coord`)
- Takes a viewport (what image region to show), a pixel rectangle to render, and canvas dimensions
- Returns RGBA pixel data as a `Vec<u8>`
- Can be implemented in any way - pixel iteration, GPU compute, lookup tables, etc.

#### `ImagePointComputer<T>` - Point-Based Computation
A specialized abstraction for algorithms that compute color one point at a time.

```rust
pub trait ImagePointComputer {
    type Coord;
    fn natural_bounds(&self) -> Rect<Self::Coord>;
    fn compute(&self, coord: Point<Self::Coord>) -> (u8, u8, u8, u8);
}
```

- Pure computation: takes a coordinate, returns RGBA color
- Stateless and generic over coordinate type
- **Note**: This is NOT a `Renderer` - it only knows how to color individual points
- Designed to be wrapped by `PixelRenderer` which adds pixel iteration logic

#### `RendererInfo<T>` - UI Integration
Optional trait for exposing renderer state to the UI.

```rust
pub trait RendererInfo {
    type Coord;
    fn info(&self, viewport: &Viewport<Self::Coord>) -> RendererInfoData;
}
```

- Provides display-formatted information (name, center, zoom, custom parameters)
- Implemented by both `ImagePointComputer` and `Renderer` implementations

### Renderer Implementations

The architecture provides composable renderers that can wrap each other:

#### `PixelRenderer<C: ImagePointComputer>`
A generic renderer that wraps any `ImagePointComputer` to add pixel iteration logic.

- Implements the `Renderer` trait
- Iterates over each pixel in the requested rectangle
- Converts pixel coordinates to image coordinates using `pixel_to_image()`
- Calls the wrapped `ImagePointComputer::compute()` for each pixel
- This is the bridge that turns point-based computation into a full `Renderer`
- One of many possible ways to implement `Renderer` - others might use GPU compute, parallel algorithms, or caching strategies

#### `TiledRenderer<R: Renderer>`
A wrapper that adds tiling to **any** `Renderer` implementation.

```rust
let tiled = TiledRenderer::new(renderer, 256);
```

- Implements `Renderer` by wrapping another `Renderer`
- Splits rendering into tiles (e.g., 256×256 pixels)
- Delegates each tile to the inner renderer
- Enables parallelization, progress tracking, and better memory locality
- Works with any renderer, not just `PixelRenderer`

#### Composition Example

```rust
// Implement ImagePointComputer for your algorithm
struct MyAlgorithm { /* ... */ }
impl ImagePointComputer for MyAlgorithm { /* ... */ }

// Wrap in PixelRenderer to create a Renderer
let pixel_renderer = PixelRenderer::new(MyAlgorithm::new());

// Optional: wrap in TiledRenderer for performance
let tiled_renderer = TiledRenderer::new(pixel_renderer, 256);

// Use with InteractiveCanvas
InteractiveCanvas(tiled_renderer)
```

### Generic Coordinate System

All spatial types are generic over the coordinate type `T`:

```rust
Point<T>      // (x, y) in any numeric type
Rect<T>       // min/max points defining a rectangle
Viewport<T>   // center and zoom level
```

The `Viewport` holds only navigation state:
- `center`: The point in image space that appears at the canvas center
- `zoom`: Magnification level (1.0 = show entire natural bounds, 2.0 = show half the area)

The natural bounds are provided by the `Renderer` via its `natural_bounds()` method and passed to transform functions as needed. This avoids duplicating renderer-specific information in the viewport.

**Critical Design Principle**: Image-space calculations **must always use** these generic types and **never hardcode** `f64`. This enables seamless transition to arbitrary precision types for extreme zoom levels.

### Coordinate Transformations

The `transforms` module provides generic functions for converting between coordinate spaces:

- `calculate_visible_bounds<T>(viewport, natural_bounds, ...)`: Converts `Viewport<T>` and natural bounds to visible `Rect<T>` accounting for zoom and aspect ratio
- `pixel_to_image<T>()`: Maps pixel coordinates (`f64`) to image coordinates (generic `T`)
- `image_to_pixel<T>()`: Inverse mapping for UI purposes
- `pan_viewport<T>()`: Translates viewport in image space
- `zoom_viewport_at_point<T>(viewport, natural_bounds, ...)`: Zooms while keeping a point fixed under cursor

Transform functions that need natural bounds accept it as a parameter, obtained from `Renderer::natural_bounds()`. These functions work with any numeric type `T` that supports the required arithmetic operations.

### Rendering Pipeline

1. **User Interaction**: Mouse drag/zoom events captured on canvas
2. **Transform Application**: Pixel-space transforms converted to `Viewport<T>` updates via transform functions
3. **Rendering** (example with `TiledRenderer(PixelRenderer(ImagePointComputer))`):
   - `TiledRenderer` splits canvas into tiles
   - Each tile delegated to `PixelRenderer`
   - `PixelRenderer` converts each pixel to image coordinates
   - `ImagePointComputer` computes RGBA color for that point
4. **Display**: RGBA buffer written to canvas element via `canvas_utils`

### Precision Handling for Extreme Zoom

To render at extreme zoom levels (10^100+):

1. Replace `f64` with high-precision type (e.g., `rug::Float`)
2. Implement your algorithm as `ImagePointComputer<Coord = rug::Float>`
3. Wrap in `PixelRenderer` to create a `Renderer`
4. All generic transform functions automatically work with the new type

The architecture ensures:
- Pixel coordinates remain `f64` (sufficient resolution for screen)
- Image-space calculations use arbitrary precision
- No changes needed to rendering infrastructure

### Key Architectural Patterns

- **Trait Composition**: `Renderer` can wrap `Renderer`, `PixelRenderer` wraps `ImagePointComputer`
- **Generic-First Design**: Coordinate types parameterized throughout, never hardcoded
- **Separation of Concerns**: Point computation, pixel iteration, tiling, and UI binding are independent
- **Type Safety**: Generic constraints ensure compile-time verification of mathematical operations
- **Flexibility**: `Renderer` can be implemented in any way - pixel iteration is just one approach

### Key Files

| Path | Purpose |
|------|---------|
| `src/rendering/renderer_trait.rs` | Core `Renderer` trait definition |
| `src/rendering/point_compute.rs` | `ImagePointComputer` trait for point-based algorithms |
| `src/rendering/pixel_renderer.rs` | Wraps `ImagePointComputer` to create a `Renderer` |
| `src/rendering/tiled_renderer.rs` | Wraps any `Renderer` to add tiling |
| `src/rendering/points.rs` | Generic `Point<T>` and `Rect<T>` types |
| `src/rendering/viewport.rs` | `Viewport<T>` state management |
| `src/rendering/transforms.rs` | Generic coordinate transformations |
| `src/components/interactive_canvas.rs` | Leptos component binding UI to any `Renderer` |

## Project Structure

```txt
fractalwonder/
├── src/
│   ├── lib.rs              # WASM entry point
│   ├── main.rs             # Empty main (for test compatibility)
│   ├── app.rs              # Main App component
│   └── components/         # UI components
│       ├── mod.rs
│       ├── canvas.rs       # Full-screen canvas rendering
│       ├── ui.rs           # Bottom UI bar
│       └── ui_visibility.rs # Auto-hide/show logic
├── index.html              # HTML entry point for Trunk
├── input.css               # Tailwind CSS source
├── tailwind.config.js      # Tailwind configuration
├── Trunk.toml              # Trunk build configuration
├── Cargo.toml              # Rust dependencies
└── README.md               # This file
```

## Technology Stack

- **Language**: Rust (100% Rust, no TypeScript/JavaScript)
- **Frontend Framework**: Leptos 0.6+ (compiled to WebAssembly)
- **Build Tool**: Trunk (bundling and dev server)
- **Styling**: Tailwind CSS (processed by Trunk)
- **Testing**: cargo test, wasm-bindgen-test

## Development Notes

- The checkerboard pattern on canvas is a placeholder for future fractal rendering
- UI auto-hides after 4 seconds of mouse inactivity (UX requirement)
- The 300ms fade animation duration follows the design specification
- Browser testing will be expanded in future development iterations

## Contributing

This is a personal project, but suggestions and bug reports are welcome via GitHub issues.

## License

MIT
