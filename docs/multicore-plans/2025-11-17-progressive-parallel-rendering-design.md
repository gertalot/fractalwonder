# Progressive Parallel Rendering Architecture Design

**Date:** 2025-11-17
**Status:** Design Phase
**Context:** Architecture for 30-minute deep zoom renders with progressive display and multi-core parallelism

---

## Executive Summary

This design implements progressive rendering with Web Workers and perturbation theory for extreme zoom levels (10^100+) in Fractal Wonder. We build the architecture incrementally across 7 testable iterations, each delivering shippable value.

**Key architectural decisions:**
- Workspace separation based on DOM dependencies (enables workers)
- Manual Web Workers with atomic work distribution (proven, documented approach)
- Size-based + stability-based tile subdivision (progressive UX + numerical correctness)
- Single reference orbit → multiple references (incremental perturbation complexity)

**Note:** Originally planned to use wasm-bindgen-rayon, but research showed this is not documented/proven with Trunk. We use manual workers instead - simpler, proven for fractals, works with existing tooling.

---

## Overall Architecture

### Layer Separation

**Main Thread (UI Layer)**
- Leptos reactive UI (pan/zoom controls, progress indicators)
- Canvas rendering (colorization + drawing)
- User interaction handling (immediate cancel on interaction)
- Progressive render loop (polls SharedArrayBuffer via `requestAnimationFrame`)
- **Never blocks** - all computation off-thread

**Worker Threads (N workers, typically = CPU cores)**
- Each loads fractalwonder-compute WASM independently
- Share memory via SharedArrayBuffer
- Compete for work using atomic counter (work-stealing pattern)
- Sequential phases:
  - Phase 1: Reference orbit computation (arbitrary precision, computed once by coordinator)
  - Phase 2: Adaptive subdivision (computed once by coordinator)
  - Phase 3: Parallel tile computation (workers atomically grab next tile index)
- Each worker writes results directly to SharedArrayBuffer
- No coordination between workers needed (fully independent)

**Key principle:** Separation of concerns. Main thread = presentation, Worker = computation.

---

## Iteration Roadmap

### Iteration 1: Workspace Restructure

**Goal:** Separate code by DOM dependencies to enable workers

**Workspace structure:**

**fractalwonder-core** - Shared types (NO DOM)
- Geometric types: Point, Rect, Viewport, PixelRect
- Numeric types/traits: ToF64, BigFloat
- Coordinate transforms
- Types used by BOTH compute and UI

**fractalwonder-compute** - Computation engine (NO DOM)
- Renderer trait, ImagePointComputer trait
- MandelbrotComputer, PixelRenderer, AdaptiveMandelbrotRenderer
- All computation implementations from `src/rendering/` except TilingCanvasRenderer
- This is the fractal computation engine

**fractalwonder-ui** - UI/Presentation layer (HAS DOM)
- TilingCanvasRenderer (uses HtmlCanvasElement)
- Colorizer type + implementations (mandelbrot_fire_colorizer, etc.)
- Leptos components, hooks, state
- hydrate()

**Dependency chain:** `ui → compute → core`

**Migration:**
- Move `src/rendering/` → `fractalwonder-compute/src/` (except TilingCanvasRenderer)
- Move TilingCanvasRenderer, colorizers → `fractalwonder-ui/src/`
- Move Leptos app → `fractalwonder-ui/src/`
- Create shared types → `fractalwonder-core/src/`

**Validation:** App works identically, all tests pass, clean build

---

### Iteration 2: Progressive Rendering (Single-Threaded)

**Goal:** Build true progressive architecture before adding workers

**Problem with current TilingCanvasRenderer:**
- Synchronous blocking loop never yields to browser
- Blocks main thread until all tiles complete
- Renders tiles but provides no progressive display

**New architecture:**
- Async tile scheduling (requestAnimationFrame or setTimeout between tiles)
- Main thread yields between tiles → stays responsive
- Computation still on main thread (single-threaded)
- User sees tiles appear progressively AND can interact

**Implementation:**
- Replace TilingCanvasRenderer with async progressive renderer
- Tile subdivision based on maximum tile size (e.g., 256×256 pixels)
- Compute tiles asynchronously with yields
- Immediate cancellation support (render ID checking)

**Validation:**
- Tiles appear one by one during render
- UI responds to clicks and keypresses while rendering
- Pan or zoom stops current render within 100ms

---

### Iteration 3: Manual Web Workers Setup

**Goal:** Add multi-core parallelism using manual Web Workers with atomic work distribution

**Architecture:**

```
Main Thread (UI - never blocks)
    ↓ Creates SharedArrayBuffer
    ↓ Spawns N workers (navigator.hardwareConcurrency)
    ↓ postMessage(viewport, render_id, shared_buffer)
    ↑ Polls SharedArrayBuffer for results

Worker Threads (N = CPU cores)
    - Each loads fractalwonder-compute WASM
    - Share same memory via SharedArrayBuffer
    - Loop: atomically increment counter → get tile → compute → write results
    - No coordination needed between workers
```

**Key insight:** Work-stealing via atomic counter. Each worker competes for next tile index.

**Implementation:**
- Create worker entry points in fractalwonder-compute
- Spawn N workers from main thread using web_sys::Worker
- Implement SharedArrayBuffer with atomic counter at offset 0
- Workers use Atomics.add() to get next tile index
- Workers write tile results directly to shared buffer
- Main thread polls buffer, deserializes, colorizes, displays

**Validation:**
- CPU utilization shows multi-core usage
- Render time decreases vs. single-threaded
- Progressive display still works

---

### Iteration 4: Responsive Cancellation

**Goal:** Pan/zoom immediately stops render, UI never freezes

**The Problem:**
At extreme zoom levels, individual tiles can take **minutes** to compute. When a user interacts during a long render:
- Workers are busy computing expensive tiles for the old viewport
- Even with render_id checking, workers cannot process cancellation messages until they finish their current tile
- Result: User experiences **unpredictable lag** (seconds to minutes) before the new render starts

**The Solution: Terminate and Recreate Workers**

Instead of cooperative cancellation (render_id checking), forcefully terminate and recreate workers:

```rust
pub fn cancel_and_restart(&mut self) {
    // 1. Immediately kill all workers (stops CPU instantly)
    for worker in &self.workers {
        worker.terminate();  // Web Workers API - hard stop
    }

    // 2. Recreate worker pool (80-400ms)
    self.workers = create_workers(worker_count);

    // 3. Start new render with fresh workers
    self.start_render(new_viewport, ...);
}
```

**Why This Works:**
- `worker.terminate()` **immediately stops** JavaScript execution in the worker thread
- CPU resources freed instantly (no waiting for expensive tile to finish)
- Worker recreation takes **predictable 80-400ms** (measured in codebase)
- New render guaranteed to start in < 0.5 seconds regardless of tile complexity

**Tradeoff Analysis:**

| Scenario | Render ID (cooperative) | Terminate & Recreate |
|----------|-------------------------|----------------------|
| Simple tiles (10ms) | ~10ms lag | ~300ms lag |
| Complex tiles (30 sec) | **~30 second lag** | ~300ms lag ✅ |
| Extreme tiles (5 min) | **~5 minute lag** | ~300ms lag ✅ |

**Key Insight:** Predictable bounded latency (< 0.5s) is better than unpredictable unbounded latency (seconds to minutes).

**Validation:**
- Pan/zoom during long render → new render starts within 500ms
- CPU usage drops to zero immediately when workers terminated
- No accumulation of stale worker threads
- UI stays responsive throughout

---

### Iteration 5: Optimize Tile Scheduling/Sizing

**Goal:** Tune the working manual worker architecture

**Optimizations:**
- Experiment with tile sizes for optimal progressive display
- Tile ordering strategies (visible-first, spiral-out, etc.)
- Memory management, buffer reuse
- Performance profiling and tuning
- Consider dynamic tile sizing based on complexity

**Validation:** Benchmark improvements, smooth progressive display

---

### Iteration 6: Perturbation Theory (Single Reference)

**Goal:** Enable deep zoom (10^50+) with fast rendering

**Perturbation theory components:**
1. Compute ONE reference orbit (arbitrary precision, slow, computed once)
2. Compute all pixels as deltas from reference (f64, fast, parallel)

**Worker phases:**
```rust
// Phase 1: Compute reference orbit (single-threaded, arbitrary precision)
let reference = compute_reference_orbit_arbitrary_precision(
    viewport.center,
    max_iterations
);

// Phase 2: Perturbation-based tile computation (f64, parallel via workers)
// Each worker loops:
loop {
    let tile_index = atomic_fetch_add(counter, 1);
    if tile_index >= total_tiles { break; }

    let tile = tiles[tile_index];
    for pixel in tile.pixels() {
        let delta_c = pixel.coords - reference.center;  // f64
        let result = compute_perturbation_f64(delta_c, &reference);  // f64
        write_to_shared_buffer(result);
    }
}
```

**Key benefit:** 99%+ of computation uses fast f64, only reference uses slow arbitrary precision

**Validation:**
- Zoom to 10^50+ levels
- Renders remain fast
- Image quality maintained

---

### Iteration 7: Adaptive Quadtree (Multiple References)

**Goal:** Extreme zoom (10^100+) with numerical stability

**Two subdivision criteria:**

**1. Size-based subdivision (for progressive rendering UX)**
- Maximum tile size (e.g., 256×256 pixels)
- Ensures multiple tiles even when stable
- User sees progress immediately

**2. Stability-based subdivision (for perturbation correctness)**
- Subdivides tiles where perturbation becomes unstable
- Computes new reference orbit for unstable regions
- Can subdivide below max tile size if needed
- Creates many small tiles in chaotic regions

**Combined algorithm:**
```rust
fn subdivide_viewport(viewport) -> Vec<Tile> {
    // First: enforce max size for progressive rendering
    let tiles = subdivide_to_max_tile_size(viewport);

    // Second: stability-driven refinement
    for tile in tiles {
        if !is_stable(tile, reference) {
            subtiles = subdivide_into_4(tile);
            for subtile where !is_stable(subtile) {
                subtile.reference = compute_new_reference(subtile.center);
            }
            tiles.extend(subtiles);
        }
    }

    return tiles;
}
```

**Validation:**
- Zoom to 10^100+ levels
- Image artifact-free
- Automatic subdivision in chaotic regions

---

## Data Structures

### SharedArrayBuffer (Conceptual)

**Contains:**
- Metadata (cancel flag, completion counters) - atomic operations
- Serialized AppData for each pixel
- Tile completion tracking

**AppData serialization:**
- Current: MandelbrotData (iterations: u32, escaped: bool)
- Future: Will be extended with magnitude, distance estimation, etc.
- Exact format TBD, will evolve as needed

Workers write serialized AppData, main thread reads and deserializes for colorization.

### Message Passing

**Main Thread → Worker:**
- Render requests (viewport, canvas size, render ID)
- Cancel requests

**Worker → Main Thread:**
- Progress notifications (tile complete)
- Render complete notifications

Exact message formats will be defined during implementation.

---

## Testing Strategy

**Iteration 1 (Workspace):**
- All existing tests still pass
- Build succeeds for all three crates
- App runs identically to before

**Iteration 2 (Progressive Rendering):**
- Manual: Watch tiles appear progressively
- Verify: UI responsive during render
- Test: Cancel mid-render works

**Iterations 3-5 (Parallelism):**
- Measure: CPU utilization across cores
- Benchmark: Render time improvements
- Test: Cancellation responsiveness
- Verify: Correct results vs. single-threaded

**Iterations 6-7 (Perturbation):**
- Test: Zoom to 10^50, 10^100 depths
- Verify: Image correctness vs. known reference images
- Benchmark: Render time at extreme zoom
- Test: Adaptive subdivision behavior in different regions

---

## Key Architectural Decisions

### Why workspace separation by DOM dependencies?

Workers cannot access DOM. Current single-crate architecture has `hydrate()` (DOM dependency) in the same crate as computation logic. Workers fail to load. Solution: Separate by DOM dependencies - compute has no DOM, UI has DOM.

### Why manual workers instead of wasm-bindgen-rayon?

**Research showed wasm-bindgen-rayon + Trunk is not documented/proven:**
- Official rayon demo uses Webpack, not Trunk
- No confirmed working examples with Trunk exist
- Requires nightly Rust, complex build configuration
- Higher risk of getting stuck (as happened in previous session)

**Manual workers are proven for fractal rendering:**
- Multiple working examples (sgasse/wasm_worker_interaction, ScottLogic, webfractals)
- Works with existing Trunk + Leptos stack
- Simpler to debug and understand
- No nightly Rust or special build flags needed
- Atomic work distribution achieves same performance

### Why N independent workers instead of coordinator pattern?

Fractal tile rendering is embarrassingly parallel. Each tile is independent, so workers can compete for work via atomic counter without needing coordination. This is simpler than a coordinator distributing work.

### Why separate size-based and stability-based subdivision?

Size-based ensures progressive UX (always have multiple tiles). Stability-based ensures numerical correctness (accurate perturbation). Two orthogonal concerns that work together.

### Why single reference first, then multiple references?

Validates perturbation theory works before adding adaptive complexity. Single reference (Iteration 6) proves the technique. Multiple references (Iteration 7) adds adaptive subdivision for extreme zoom.

---

## Future Considerations

**GPU acceleration (beyond this design):**
- Perturbation calculations (f64 operations) are GPU-friendly
- Reference orbits stay on CPU (require arbitrary precision)
- WebGPU compute shaders could replace manual workers
- Design allows swapping compute backend without changing architecture

**Series approximation:**
- Skip early iterations for faster rendering
- Can be added to perturbation calculation without architectural changes

**Caching:**
- Cache reference orbits for repeated renders at same location
- Cache tiles for smooth panning
- Can be added incrementally

---

## Success Criteria

**Iteration 1:** Workspace builds, app works identically
**Iteration 2:** Progressive display visible, UI responsive
**Iteration 3:** Multi-core CPU utilization, faster renders
**Iteration 4:** Instant cancellation on interaction
**Iteration 5:** Optimized performance, smooth UX
**Iteration 6:** 10^50+ zoom functional, fast renders
**Iteration 7:** 10^100+ zoom artifact-free, automatic subdivision

Each iteration delivers testable, shippable value. Architecture supports future enhancements (GPU, caching, series approximation) without major rework.
