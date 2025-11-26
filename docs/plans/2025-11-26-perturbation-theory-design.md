# Perturbation Theory Renderer - Design

**Date:** 2025-11-26
**Iteration:** 12
**Status:** Draft

## Overview

Iteration 12 adds a `ParallelPerturbationRenderer` to enable extreme deep zoom (10^1000+) with practical render times.

**The problem:** The current `MandelbrotRenderer` uses BigFloat arithmetic for every pixel. At 10^1000 zoom, BigFloat operations with ~3300 bits of precision are extremely slow. Rendering a single frame could take hours.

**The solution:** Perturbation theory computes ONE high-precision reference orbit, then uses fast f64 arithmetic to compute deltas for all other pixels. This reduces render time by 50-100x.

**Key components:**
- `ParallelPerturbationRenderer` - New renderer in UI layer, orchestrates the two-phase render
- `ComputeReferenceOrbit` / `RenderTilePerturbation` - New worker message types
- Delta iteration algorithm - f64 computation with per-pixel rebasing
- Reference orbit caching - Workers store orbit locally by ID

**Render flow:**
1. Compute 1 reference orbit (viewport center) using BigFloat
2. Broadcast orbit data (as f64) to all workers
3. Workers compute pixels using f64 delta iterations
4. Colorize and draw (unchanged)

---

## Message Protocol

New message types added to `MainToWorker` and `WorkerToMain`:

```rust
// Main -> Worker
enum MainToWorker {
    // ... existing messages ...

    /// Compute a reference orbit at high precision
    ComputeReferenceOrbit {
        render_id: u32,
        orbit_id: u32,
        c_ref_json: String,  // BigFloat coordinates as JSON
        max_iterations: u32,
    },

    /// Store a reference orbit for use in tile rendering
    StoreReferenceOrbit {
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    },

    /// Render a tile using perturbation
    RenderTilePerturbation {
        render_id: u32,
        tile: PixelRect,
        orbit_id: u32,
        delta_c_origin: (f64, f64),
        delta_c_step: (f64, f64),
        max_iterations: u32,
    },

    /// Discard a cached orbit
    DiscardOrbit { orbit_id: u32 },
}

// Worker -> Main
enum WorkerToMain {
    // ... existing messages ...

    /// Reference orbit computation complete
    ReferenceOrbitComplete {
        render_id: u32,
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    },

    /// Orbit stored and ready
    OrbitStored { orbit_id: u32 },
}
```

---

## Render Flow

```
1. User triggers render (viewport change)
   |
2. Main thread: Compute reference point location
   |  - c_ref = viewport.center (BigFloat)
   |  - max_iterations = 50 * zoom_exp^1.25
   |
3. Main thread: Queue orbit computation
   |  - Send ComputeReferenceOrbit to first available worker
   |  - Worker computes orbit using BigFloat
   |  - Worker returns ReferenceOrbitComplete with f64 orbit data
   |
4. Main thread: Broadcast orbit to all workers
   |  - Send StoreReferenceOrbit to ALL workers
   |  - Workers cache orbit locally, reply OrbitStored
   |  - Wait for all workers to confirm storage
   |
5. Main thread: Queue tile jobs
   |  - Generate tiles (same as current ParallelRenderer)
   |  - For each tile: compute delta_c_origin and delta_c_step
   |  - Send RenderTilePerturbation messages
   |
6. Workers: Render tiles using perturbation
   |  - Look up cached orbit
   |  - Run f64 delta iteration with rebasing
   |  - Return TileComplete (same as current)
   |
7. Main thread: Colorize and draw (unchanged)
```

**Cancellation:** If user interacts mid-render:
- Cancel aborts at any phase
- Orbit computation can be interrupted
- Cached orbits discarded on next render (new orbit_id)

---

## Delta Iteration Algorithm

The core computation in workers for perturbation rendering:

```rust
fn compute_pixel_perturbation(
    orbit: &[(f64, f64)],      // Pre-computed X_n values (f64)
    c_ref: (f64, f64),         // Reference point for on-the-fly computation
    delta_c: (f64, f64),       // Pixel offset from reference
    max_iter: u32,
    escaped_at: Option<u32>,   // When reference orbit escaped
) -> MandelbrotData {
    let (mut dx, mut dy) = (0.0, 0.0);  // delta starts at 0
    let (mut x, mut y) = (0.0, 0.0);     // X_n for on-the-fly mode
    let mut on_the_fly = false;

    for n in 0..max_iter {
        // Get X_n from orbit or compute on-the-fly
        let (xn, yn) = if !on_the_fly && (escaped_at.is_none() || n < escaped_at.unwrap()) {
            orbit[n as usize]
        } else {
            on_the_fly = true;
            let new_x = x * x - y * y + c_ref.0;
            let new_y = 2.0 * x * y + c_ref.1;
            x = new_x; y = new_y;
            (x, y)
        };

        // Escape check: |X_n + delta_n|^2 > 4
        let zx = xn + dx;
        let zy = yn + dy;
        if zx * zx + zy * zy > 4.0 {
            return MandelbrotData { iterations: n, escaped: true, max_iterations: max_iter };
        }

        // Rebase check: |delta|^2 > 0.25 * |X|^2
        if !on_the_fly && (dx * dx + dy * dy) > 0.25 * (xn * xn + yn * yn) {
            // Switch to on-the-fly: X becomes Z, delta resets
            x = zx; y = zy;
            dx = 0.0; dy = 0.0;
            on_the_fly = true;
            continue;
        }

        // Delta iteration: delta_{n+1} = 2*X_n*delta_n + delta_n^2 + delta_c
        let new_dx = 2.0 * (xn * dx - yn * dy) + dx * dx - dy * dy + delta_c.0;
        let new_dy = 2.0 * (xn * dy + yn * dx) + 2.0 * dx * dy + delta_c.1;
        dx = new_dx;
        dy = new_dy;
    }

    MandelbrotData { iterations: max_iter, escaped: false, max_iterations: max_iter }
}
```

**Key points:**
- All arithmetic is f64 (fast)
- Rebasing switches to on-the-fly mode (no return to orbit)
- Reference escape also triggers on-the-fly mode
- Same code path handles both cases

---

## Config Integration

Add `renderer_type` field to `FractalConfig`:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RendererType {
    Simple,       // Current approach (BigFloat per pixel)
    Perturbation, // Perturbation theory (f64 deltas)
}

pub struct FractalConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_center: (&'static str, &'static str),
    pub default_width: &'static str,
    pub default_height: &'static str,
    pub renderer_type: RendererType,
}

pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        renderer_type: RendererType::Simple,
        // ...
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        renderer_type: RendererType::Perturbation,
        // ...
    },
];
```

**Max iterations computed from zoom:**

```rust
pub fn max_iterations_for_zoom(zoom_exponent: f64) -> u32 {
    let iterations = 50.0 * zoom_exponent.powf(1.25);
    iterations.max(1000.0).min(10_000_000.0) as u32
}
```

**Renderer selection:**

The UI layer selects renderer based on `config.renderer_type`:
- `Simple` -> `ParallelRenderer` (current)
- `Perturbation` -> `ParallelPerturbationRenderer` (new)

---

## Worker State

Workers become stateful to cache reference orbits:

```rust
// In worker.rs
struct WorkerState {
    renderer_id: String,
    cached_orbits: HashMap<u32, CachedOrbit>,
}

struct CachedOrbit {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
}

impl WorkerState {
    fn handle_message(&mut self, msg: MainToWorker) -> Option<WorkerToMain> {
        match msg {
            MainToWorker::StoreReferenceOrbit { orbit_id, c_ref, orbit, escaped_at } => {
                self.cached_orbits.insert(orbit_id, CachedOrbit { c_ref, orbit, escaped_at });
                Some(WorkerToMain::OrbitStored { orbit_id })
            }

            MainToWorker::DiscardOrbit { orbit_id } => {
                self.cached_orbits.remove(&orbit_id);
                None
            }

            MainToWorker::RenderTilePerturbation { orbit_id, tile, .. } => {
                let orbit = self.cached_orbits.get(&orbit_id)
                    .expect("Orbit must be cached before rendering");
                // ... render tile using cached orbit
            }

            // ... other messages
        }
    }
}
```

**Orbit lifecycle:**
1. `ComputeReferenceOrbit` -> worker computes, returns data
2. `StoreReferenceOrbit` -> all workers cache it
3. `RenderTilePerturbation` -> workers use cached orbit
4. `DiscardOrbit` -> workers free memory (optional cleanup)

**Memory management:**
- New render -> new orbit_id -> old orbits can be discarded
- Workers can hold ~160MB per orbit (10M iterations * 16 bytes)
- With single-orbit strategy, memory is bounded

---

## Testing Strategy

**Unit tests (fractalwonder-compute):**

```rust
// Delta iteration correctness
#[test]
fn perturbation_matches_direct_computation() {
    // Compute a point using full BigFloat
    // Compute same point using perturbation from nearby reference
    // Results should match within f64 tolerance
}

#[test]
fn rebasing_triggers_at_threshold() {
    // Create scenario where |delta| > 0.5 * |X|
    // Verify rebasing activates and computation continues correctly
}

#[test]
fn on_the_fly_continues_after_reference_escapes() {
    // Reference escapes at iteration 100
    // Pixel needs 200 iterations
    // Verify correct result
}
```

**Integration tests (fractalwonder-ui):**

```rust
#[test]
fn reference_orbit_roundtrip() {
    // Serialize ComputeReferenceOrbit message
    // Deserialize, verify BigFloat precision preserved
}

#[test]
fn orbit_broadcast_to_all_workers() {
    // Start render, verify all workers receive StoreReferenceOrbit
}
```

**Browser tests:**

- Render at zoom 10^10, 10^100, 10^1000
- Compare against known reference images (visual regression)
- Verify no glitchy artifacts
- Measure render time improvement vs Simple renderer

---

## Future Enhancements

These are explicitly out of scope for v1 but documented for future reference:

1. **Adaptive reference placement** - Add more reference orbits in regions with excessive rebasing
2. **Series approximation** - Skip early iterations using Taylor series (BLA algorithm)
3. **Glitch detection heuristics** - Detect visual artifacts and automatically recompute
4. **SharedArrayBuffer** - Share orbit data across workers without copying (requires nightly Rust)

---

## Summary of Design Decisions

| Topic | Decision |
|-------|----------|
| Renderer | New `ParallelPerturbationRenderer` |
| Reference orbits | 1 orbit (viewport center), add more later if needed |
| Orbit computation | Any worker, just another message type |
| Orbit storage | Workers cache locally (broadcast to all) |
| Orbit data | `c_ref: (f64, f64)`, `orbit: Vec<(f64, f64)>`, `escaped_at` |
| Glitch handling | Per-pixel rebasing, adaptive placement later if needed |
| Reference escape | On-the-fly computation (same code path as rebasing) |
| Message protocol | Separate types (`RenderTilePerturbation` vs `RenderTile`) |
| Delta format | `delta_c_origin + delta_c_step` |
| Config | Add `renderer_type: RendererType` field |
| Max iterations | Computed from zoom: `50 * zoom_exp^1.25`, clamped to [1000, 10M] |
