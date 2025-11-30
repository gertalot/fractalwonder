# GPU Increment 1: UI Integration Design

> Design document for integrating GPU renderer into the UI layer.

**Date:** 2025-11-29
**Status:** Approved
**Depends on:** GPU crate (tasks 1-9 complete)

---

## Overview

Integrate `fractalwonder-gpu` into `fractalwonder-ui` so GPU rendering is visible in the browser. GPU renders all pixels in one dispatch, returns `Vec<ComputeData>` matching CPU output, and draws full frame to canvas.

**Scope:**
- Lazy GPU initialization in `ParallelRenderer`
- Hook GPU render into `ReferenceOrbitComplete` handler
- GPU returns `Vec<ComputeData>` (same as CPU)
- Draw full frame to canvas
- Silent fallback to CPU on GPU failure (log to console)

**Out of scope (future increments):**
- Multi-pass progressive rendering (Increment 2)
- Automatic glitch correction (glitches marked, not re-rendered)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      ParallelRenderer                            │
│                                                                  │
│  ┌─────────────────┐      ┌─────────────────────────────────┐   │
│  │   WorkerPool    │      │  GpuRenderer (Option)           │   │
│  │                 │      │  - Lazily initialized           │   │
│  │  - Computes     │      │  - Renders all pixels at once   │   │
│  │    reference    │      │  - Returns Vec<ComputeData>     │   │
│  │    orbit        │      │                                 │   │
│  │  - Fallback for │      └─────────────────────────────────┘   │
│  │    CPU tiles    │                                            │
│  └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
```

**Key decisions:**
- `GpuRenderer` lives inside `ParallelRenderer` as `Option<GpuRenderer>`
- Initialized lazily on first render when `config.gpu_enabled == true`
- GPU failure → silent fallback to CPU (log to console)
- GPU returns `Vec<ComputeData>` matching CPU output exactly

---

## Render Flow

When `render()` is called with `gpu_enabled: true`:

```
1. start_perturbation_render()
   └── Send ComputeReferenceOrbit to worker

2. Worker returns ReferenceOrbitComplete
   ├── Store orbit in perturbation.pending_orbit
   │
   ├── IF gpu_renderer exists AND gpu_enabled:
   │   ├── Spawn async GPU render
   │   ├── GPU returns Vec<ComputeData> for all pixels
   │   ├── Colorize all pixels (reuse existing colorize())
   │   ├── Draw full frame to canvas (single putImageData)
   │   ├── IF GPU fails:
   │   │   └── Fall back to CPU tile dispatch
   │   └── DONE
   │
   └── ELSE (no GPU or GPU disabled):
       ├── Broadcast orbit to all workers
       ├── Dispatch tiles to workers (existing flow)
       └── Tiles colorized and drawn as they complete
```

**Critical:** CPU tile dispatch only happens if GPU is unavailable or fails. No redundant work.

---

## GPU Result Conversion

**GpuRenderResult changes:**

```rust
// Before (raw data)
pub struct GpuRenderResult {
    pub iterations: Vec<u32>,
    pub glitch_flags: Vec<bool>,
    pub compute_time_ms: f64,
}

// After (matches CPU output)
pub struct GpuRenderResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}
```

**Conversion inside `GpuRenderer::render()`:**

```rust
let data: Vec<ComputeData> = iterations
    .iter()
    .zip(glitch_flags.iter())
    .map(|(&iter, &glitched)| {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: iter,
            max_iterations,
            escaped: iter < max_iterations,
            glitched,
        })
    })
    .collect();
```

---

## ParallelRenderer Changes

**New fields:**

```rust
pub struct ParallelRenderer {
    // ... existing fields ...

    /// GPU renderer, lazily initialized when gpu_enabled
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,

    /// Whether GPU init has been attempted (avoid retry on failure)
    gpu_init_attempted: Rc<Cell<bool>>,
}
```

**Async GPU render in orbit complete handler:**

```rust
if self.config.gpu_enabled && self.gpu_renderer.borrow().is_some() {
    wasm_bindgen_futures::spawn_local(async move {
        match try_gpu_render(/* params */).await {
            Ok(gpu_data) => {
                // Colorize and draw full frame
                let pixels: Vec<u8> = gpu_data
                    .iter()
                    .flat_map(|d| colorize(d, xray))
                    .collect();
                draw_full_frame(&ctx, &pixels, width, height);
            }
            Err(e) => {
                log::warn!("GPU render failed: {e}");
                // Fall back to CPU tile dispatch
                dispatch_cpu_tiles(/* ... */);
            }
        }
    });
} else {
    // No GPU - use existing CPU tile dispatch
    broadcast_orbit_and_dispatch_tiles(/* ... */);
}
```

---

## Full Frame Drawing

**New function in `canvas_utils.rs`:**

```rust
/// Draw an entire frame to canvas (for GPU results).
pub fn draw_full_frame(
    ctx: &CanvasRenderingContext2d,
    pixels: &[u8],  // RGBA, length = width * height * 4
    width: u32,
    height: u32,
) -> Result<(), JsValue> {
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(pixels),
        width,
        height,
    )?;

    ctx.put_image_data(&image_data, 0.0, 0.0)
}
```

---

## Files to Modify

| File | Change |
|------|--------|
| `fractalwonder-gpu/Cargo.toml` | Add `fractalwonder-core` dependency |
| `fractalwonder-gpu/src/renderer.rs` | Return `Vec<ComputeData>` instead of raw vecs |
| `fractalwonder-gpu/src/lib.rs` | Update exports |
| `fractalwonder-ui/Cargo.toml` | Add `fractalwonder-gpu` dependency |
| `fractalwonder-ui/src/rendering/parallel_renderer.rs` | Add `GpuRenderer`, hook into orbit complete |
| `fractalwonder-ui/src/rendering/canvas_utils.rs` | Add `draw_full_frame()` |
| `fractalwonder-ui/src/workers/worker_pool.rs` | Expose orbit data for GPU consumption |

**New dependencies:**
```toml
# fractalwonder-gpu/Cargo.toml
fractalwonder-core = { path = "../fractalwonder-core" }

# fractalwonder-ui/Cargo.toml
fractalwonder-gpu = { path = "../fractalwonder-gpu" }
```

---

## Error Handling

- GPU initialization failure → log to console, set `gpu_init_attempted = true`, use CPU
- GPU render failure → log to console, fall back to CPU tile dispatch
- No user-visible errors (silent fallback)

---

## Testing

GPU integration tested by:
1. Existing GPU unit tests (tasks 1-9) verify GPU correctness
2. Visual verification in browser: GPU renders same image as CPU
3. Console logs confirm GPU path is taken when enabled
4. Disabling GPU in config falls back to CPU correctly

---

## Acceptance Criteria

- [ ] GPU renders full frame when `gpu_enabled: true`
- [ ] Output matches CPU rendering (same colors, same glitch markers)
- [ ] GPU failure falls back to CPU silently (console log only)
- [ ] No redundant CPU work when GPU succeeds
- [ ] Existing CPU path unchanged when GPU disabled
