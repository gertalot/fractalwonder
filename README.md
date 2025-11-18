# Fractal Wonder

A high-performance, browser-based Mandelbrot set explorer capable of rendering at extreme zoom levels (up to 10^100 and
beyond) with interactive real-time exploration. Built entirely in Rust using Leptos and WebAssembly.

## Features

- **Progressive Rendering**: Tiles appear incrementally during long renders
- **Responsive UI**: Interact with controls while rendering (pan, zoom, change settings)
- **Immediate Cancellation**: Pan/zoom instantly stops current render and starts new one
- **Extreme Precision**: Arbitrary precision support for zoom levels up to 10^100 and beyond
- **Pure Rust/WASM**: 100% Rust codebase compiled to WebAssembly

## Table of Contents

- [Prerequisites](#prerequisites)
  - [Option 1: Development Container (Recommended)](#option-1-development-container-recommended)
  - [Option 2: Local Installation](#option-2-local-installation)
- [Development](#development)
  - [Clone and Run](#clone-and-run)
  - [Development Modes](#development-modes)
- [Testing](#testing)
- [Building](#building)
  - [Production Build](#production-build)
  - [Serving Production Build Locally](#serving-production-build-locally)
- [Development Container](#development-container)
  - [What's Included](#whats-included)
  - [Recommended Workflow: Hybrid Setup](#recommended-workflow-hybrid-setup)
  - [Setup Option A: Using VS Code (Easier)](#setup-option-a-using-vs-code-easier)
  - [Setup Option B: Using Command Line (Without VS Code)](#setup-option-b-using-command-line-without-vs-code)
  - [Alternative: Fully Containerized Workflow](#alternative-fully-containerized-workflow)
  - [Features](#features)
  - [Troubleshooting](#troubleshooting)
  - [Container Configuration](#container-configuration)
- [Architecture](#architecture)
  - [Coordinate Spaces](#coordinate-spaces)
  - [Core Traits](#core-traits)
  - [Renderer Implementations](#renderer-implementations)
  - [Generic Coordinate System](#generic-coordinate-system)
  - [Coordinate Transformations](#coordinate-transformations)
  - [Rendering Pipeline](#rendering-pipeline)
  - [Precision Handling for Extreme Zoom](#precision-handling-for-extreme-zoom)
  - [Key Architectural Patterns](#key-architectural-patterns)
  - [Key Files](#key-files)
- [Project Structure](#project-structure)
- [Technology Stack](#technology-stack)
- [Development Notes](#development-notes)
- [Contributing](#contributing)
- [License](#license)

## Prerequisites

You can develop FractalWonder in two ways:

### Option 1: Development Container (Recommended)

Use VS Code's devcontainer feature for an isolated, fully-configured environment with all tools pre-installed.

**Requirements:**
- [Docker Desktop](https://www.docker.com/products/docker-desktop)
- [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

**Setup:**
1. Open the project in VS Code
2. When prompted, click "Reopen in Container" (or use Command Palette: "Dev Containers: Reopen in Container")
3. Wait for the container to build (5-10 minutes first time, cached afterward)
4. All dependencies are pre-installed and ready to use

See the [Development Container](#development-container) section below for details.

### Option 2: Local Installation

Install dependencies directly on your system:

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

## Development Container

FractalWonder includes a fully-configured development container for isolated, reproducible development environments. The container is designed to run **Claude Code in isolation** while your normal development tools (trunk, Chrome) run on the host.

### What's Included

The devcontainer provides:
- **Full Rust toolchain**: rustc, cargo, clippy, rustfmt (stable channel)
- **WASM tools**: wasm32-unknown-unknown target, wasm-pack, Trunk
- **Node.js 20**: For Tailwind CSS and npm tooling
- **Chrome**: Headless Chrome for browser testing
- **Git & GitHub CLI**: Full version control integration
- **VS Code extensions**: rust-analyzer, Claude Code, Tailwind CSS IntelliSense
- **Shared credentials**: Your host `~/.claude` folder is automatically mounted for authentication

### Recommended Workflow: Hybrid Setup

The most practical setup is to run Claude Code in the container while keeping your development tools on the host:

**On Host (your normal machine):**
```bash
# Terminal 1: Start the dev server
trunk serve
# Access at http://localhost:8080

# Terminal 2: Run Chrome with remote debugging for Claude's chrome-devtools MCP
google-chrome --remote-debugging-port=9222
# (On Windows: chrome.exe --remote-debugging-port=9222)

# Terminal 3 (optional): Run tests, format, lint as usual
cargo test
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

**In Container (via VS Code):**
```bash
# Open project in VS Code → "Reopen in Container"
# Once inside container, run Claude Code with full permissions
claude --dangerously-skip-permissions
```

**How it works:**
- Claude Code runs isolated in the container (safe to use `--dangerously-skip-permissions`)
- Claude edits files via mounted workspace → host `trunk serve` detects changes → hot reload
- Claude controls host Chrome via `localhost:9222` (socat proxy forwards to host automatically)
- All test/format/lint commands work normally on host
- Container shares your `~/.claude` credentials (no separate login needed)

> **Note:** The container uses `socat` to forward `localhost:9222` inside the container to Chrome running on the host. Your `.mcp.json` can use `http://localhost:9222` both on the host and in the container - it works the same everywhere.

### Setup Option A: Using VS Code (Easier)

1. **Install prerequisites**:
   - [Docker Desktop](https://www.docker.com/products/docker-desktop)
   - [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
   - Rust toolchain on host (for running cargo/trunk locally - see [Prerequisites](#prerequisites))

2. **Open in container**:
   - Open FractalWonder in VS Code
   - Click "Reopen in Container" when prompted
   - Or use Command Palette: `Dev Containers: Reopen in Container`

3. **First build** (one-time, 5-10 minutes):
   - Container downloads Node.js base image
   - Installs Rust toolchain, WASM tools, Chrome
   - Subsequent starts use cached image (~30 seconds)

4. **Start developing**:
   - Run `trunk serve` on **host** (in a separate terminal outside VS Code)
   - Run `google-chrome --remote-debugging-port=9222` on **host**
   - Open VS Code terminal inside container and run `claude --dangerously-skip-permissions`
   - Access application at `http://localhost:8080` in your host browser

### Setup Option B: Using Command Line (Without VS Code)

If you prefer to run Claude in the container from the terminal without VS Code:

1. **Use the provided script** (easiest):
   ```bash
   # Run Claude in container with any arguments
   ./scripts/claude --dangerously-skip-permissions

   # The script automatically builds the container image if needed
   # and passes all arguments to Claude Code
   ```

2. **On host** (in separate terminals):
   ```bash
   # Terminal 1: Dev server
   trunk serve

   # Terminal 2: Chrome with remote debugging
   google-chrome --remote-debugging-port=9222
   ```

**Alternative: Manual Docker command:**

If you prefer not to use the script:

1. **Build the container image** (one-time):
   ```bash
   docker build -t fractalwonder-dev .devcontainer/
   ```

2. **Run Claude in the container**:
   ```bash
   docker run -it --rm \
     --add-host=host.docker.internal:host-gateway \
     -v ~/.claude:/home/node/.claude \
     -v "$(pwd)":/workspaces/fractalwonder \
     -w /workspaces/fractalwonder \
     -u node \
     fractalwonder-dev \
     claude --dangerously-skip-permissions
   ```

   **What this does:**
   - Mounts your `~/.claude` credentials folder
   - Mounts current directory as workspace
   - Enables `host.docker.internal` for Chrome DevTools bridge
   - Runs as non-root user (`node`)
   - Launches Claude Code with full permissions inside container

### Alternative: Fully Containerized Workflow

You can also run everything inside the container:

```bash
# Inside container (VS Code terminal)
trunk serve    # Runs on port 8080, auto-forwarded to host
cargo test
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

This is slower due to Docker filesystem overhead but ensures identical environment across machines.

### Features

- **Shared credentials**: `~/.claude` folder mounted from host - Claude Code in container uses your existing authentication
- **Workspace mounting**: All file changes sync between container and host instantly
- **Port forwarding**: Port 8080 automatically forwarded to host
- **Chrome DevTools bridge**: Container can control host Chrome at `http://host.docker.internal:9222`
- **Isolated environment**: Safe to run Claude Code with `--dangerously-skip-permissions`
- **Full network access**: WebSearch, WebFetch, package managers all work normally

#### Troubleshooting

**Container build fails:**
- Ensure Docker Desktop is running
- Check Docker has sufficient memory allocated (4GB+ recommended)
- Try rebuilding: Command Palette → `Dev Containers: Rebuild Container`

**Port 8080 already in use:**
- Stop any local `trunk serve` processes on the host
- Or change port in `.devcontainer/devcontainer.json`

**File permission issues:**
- Files created in container should match host user automatically
- If issues occur, check Docker Desktop file sharing settings

**Slow builds:**
- First Rust build is always slow (downloading all crates)
- Subsequent builds use cargo cache
- Consider adding cargo registry cache mount for faster dependency downloads

### Container Configuration

Configuration files are in `.devcontainer/`:
- `Dockerfile`: Container image definition (Rust, WASM, Chrome installation)
- `devcontainer.json`: VS Code settings, extensions, mounts, port forwarding

To customize the environment, edit these files and rebuild the container.

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

Fractal Wonder uses a Cargo workspace with three crates:

- **fractalwonder-core**: Shared types and utilities (no DOM dependencies)
  - Geometric types: `Point`, `Rect`, `Viewport`, `PixelRect`
  - Numeric types: `BigFloat`, `ToF64`
  - Coordinate transforms
  - Data types: `AppData`, `MandelbrotData`, `TestImageData`

- **fractalwonder-compute**: Computation engine (no DOM dependencies)
  - Renderer trait and implementations
  - Fractal computers (Mandelbrot, TestImage)
  - Pixel rendering logic
  - Designed to run in Web Workers

- **fractalwonder-ui**: UI/presentation layer (has DOM dependencies)
  - Leptos components and hooks
  - Canvas rendering utilities
  - Colorizers
  - Application state management

Dependency chain: `fractalwonder-ui` → `fractalwonder-compute` → `fractalwonder-core`

This separation enables future Web Worker parallelization for multi-core rendering.

```txt
fractalwonder/
├── .devcontainer/          # Development container configuration
│   ├── Dockerfile          # Container image definition
│   └── devcontainer.json   # VS Code devcontainer settings
├── scripts/
│   └── claude              # Helper script to run Claude Code in container
├── fractalwonder-core/     # Shared types (no DOM)
├── fractalwonder-compute/  # Computation engine (no DOM)
├── fractalwonder-ui/       # UI layer (with DOM)
├── tests/                  # Integration tests
├── docs/                   # Documentation
├── index.html              # HTML entry point for Trunk
├── input.css               # Tailwind CSS source
├── tailwind.config.js      # Tailwind configuration
├── Trunk.toml              # Trunk build configuration
├── Cargo.toml              # Workspace manifest
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
