# Workspace Structure

## Overview

Fractal Wonder is organized as a Cargo workspace with three crates, separated by DOM dependencies to enable Web Worker parallelization.

## Crate Dependency Graph

```
fractalwonder-ui (DOM-dependent)
    ↓
fractalwonder-compute (pure computation, no DOM)
    ↓
fractalwonder-core (shared types, no DOM)
```

## fractalwonder-core

**Purpose:** Shared types used by both compute and UI layers.

**Key modules:**
- `points`: `Point<T>`, `Rect<T>` geometric primitives
- `viewport`: `Viewport<T>` for view transformations
- `pixel_rect`: `PixelRect` for pixel-space rectangles
- `numeric`: `BigFloat`, `ToF64` trait for arbitrary precision
- `transforms`: Coordinate transformation utilities
- `app_data`: `AppData` enum for renderer output data

**Dependencies:** Only `dashu`, `serde`, `dyn-clone` (no DOM)

## fractalwonder-compute

**Purpose:** Computation engine for fractal rendering.

**Key modules:**
- `renderer_trait`: `Renderer` trait for computation strategies
- `computers/`: Fractal algorithms (`MandelbrotComputer`, `TestImageComputer`)
- `pixel_renderer`: Maps pixels to computed data
- `adaptive_mandelbrot_renderer`: Adaptive precision rendering
- `render_config`: Configuration and color schemes
- `precision`: Arbitrary precision calculator

**Dependencies:** `fractalwonder-core` + `dashu` (no DOM)

**Design:** Can run in Web Workers for multi-core parallelism.

## fractalwonder-ui

**Purpose:** User interface and presentation layer.

**Key modules:**
- `app`: Main Leptos application
- `components/`: Reusable UI components
- `hooks/`: Custom Leptos hooks
- `state/`: Application state management
- `rendering/canvas_renderer`: Canvas rendering trait
- `rendering/tiling_canvas_renderer`: Progressive tiled rendering
- `rendering/canvas_utils`: Canvas rendering utilities
- `rendering/colorizers`: Data-to-RGBA color functions

**Dependencies:** `fractalwonder-compute`, `fractalwonder-core`, `leptos`, `web-sys` (has DOM)

**Design:** Main thread only, handles UI and canvas rendering.

## Why This Structure?

**Enables Web Workers:**
- Web Workers cannot access DOM
- `fractalwonder-compute` has no DOM dependencies → can run in workers
- `fractalwonder-ui` handles all DOM interaction on main thread

**Clean Separation:**
- Core types shared between layers
- Computation logic isolated from presentation
- Testable in isolation

**Future-Proof:**
- Workers can be added without refactoring computation code
- GPU acceleration can replace compute layer
- Core types remain stable
