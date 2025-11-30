# GPU Increment 2: Progressive Multi-Pass Rendering

> Design document for responsive GPU rendering with blocky→sharp visual refinement.

**Date:** 2025-11-29
**Status:** Draft
**Depends on:** GPU Increment 1 (complete)

---

## Overview

GPU rendering loses the tile-by-tile progress feedback of CPU workers. Progressive multi-pass rendering restores visual responsiveness by rendering at increasing resolutions, displaying each pass immediately.

**Fixed 4-pass schedule:**

| Pass | Resolution | Max Iterations | Pixels (4K) | Relative Work |
|------|------------|----------------|-------------|---------------|
| 1 | 1/16 | 1/16 | ~32K | Tiny |
| 2 | 1/8 | 1/8 | ~130K | Small |
| 3 | 1/4 | 1/4 | ~520K | Medium |
| 4 | Full | Full | ~8.3M | Full |

Passes 1-3 combined are <10% of pass 4's compute work.

---

## High-Level Flow

```
1. User triggers render
2. Increment generation counter
3. Compute reference orbit (once, full max_iter)
4. For each pass:
   a. Check generation counter (abort if stale)
   b. GPU renders at pass resolution/iterations
   c. Read back small ComputeData buffer
   d. Stretch ComputeData to full canvas size
   e. Store stretched ComputeData
   f. Colorizer runs → RGBA → canvas
5. Pass 4 enables glitch detection
```

**User experience:**
- Shallow zoom: All passes complete instantly, preview barely visible
- Deep zoom: Blocky preview in ~16ms, sharpens over 1-10s

---

## Interruption Handling

Navigation (zoom/pan) during render must cancel in-progress passes cleanly.

**Mechanism: Generation counter**

```rust
struct ParallelRenderer {
    render_generation: Rc<Cell<u32>>,
    // ...
}

fn start_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
    let gen = self.render_generation.get() + 1;
    self.render_generation.set(gen);

    // ... compute orbit ...

    spawn_local(async move {
        for pass in Pass::all() {
            if generation.get() != gen {
                return; // Interrupted - abort silently
            }

            render_pass(pass).await;
        }
    });
}
```

**Characteristics:**
- Lock-free: no awaiting previous render
- Stale passes exit cleanly without coordination
- No resource leaks: GPU buffers are reused, not abandoned

---

## ComputeData Stretching

Each pass renders to a small buffer, then stretches ComputeData to full canvas size before the colorizer runs.

**Why stretch ComputeData, not RGBA:**
- Colorizer is a swappable function (user can change anytime)
- Stored ComputeData enables instant recolorization without recompute
- Architecture unchanged: Render → Store → Colorize → Canvas

**Stretch function:**

```rust
fn stretch_compute_data(
    small: &[ComputeData],
    small_w: u32,
    small_h: u32,
    scale: u32,
) -> Vec<ComputeData> {
    let full_w = small_w * scale;
    let full_h = small_h * scale;
    let mut full = Vec::with_capacity((full_w * full_h) as usize);

    for sy in 0..small_h {
        for dy in 0..scale {
            for sx in 0..small_w {
                let src = &small[(sy * small_w + sx) as usize];
                for _ in 0..scale {
                    full.push(src.clone());
                }
            }
        }
    }
    full
}
```

**Performance:**
- Pass 1: stretch 32K → 8.3M (256x duplication)
- Simple memory copies, cache-friendly row-major order
- CPU cost negligible vs GPU iteration compute

---

## GPU Buffer & Dispatch

GPU buffers allocated once at full canvas size. Each pass dispatches fewer workgroups.

**Buffer allocation (unchanged from Increment 1):**
```rust
// Allocated once when canvas size changes
let results_buffer = device.create_buffer(/* 8.3M * 4 bytes */);
let glitch_buffer = device.create_buffer(/* 8.3M * 4 bytes */);
```

**Per-pass dispatch:**
```rust
fn dispatch_pass(&self, pass: Pass) -> GpuRenderResult {
    let (pass_w, pass_h) = pass.dimensions(canvas_w, canvas_h);
    let pass_max_iter = pass.scale_iterations(max_iter);
    let tau_sq = if pass.is_final() { config.tau_sq } else { 0.0 };

    // Update uniforms
    uniforms.width = pass_w;
    uniforms.height = pass_h;
    uniforms.max_iterations = pass_max_iter;
    uniforms.tau_sq = tau_sq;
    uniforms.dc_step = (vp_width / pass_w, vp_height / pass_h);

    // Dispatch only needed workgroups
    encoder.dispatch_workgroups(
        (pass_w + 7) / 8,
        (pass_h + 7) / 8,
        1,
    );

    // Read back only pass_w * pass_h results
    read_buffer_range(0..pass_w * pass_h)
}
```

**Key insight:** Shader uses `uniforms.width/height` to bounds-check. Only relevant portion of buffer is read back.

---

## Reference Orbit

Computed once before all passes, reused across passes.

- Reference orbit computed at full max_iter
- Shorter passes use prefix of the orbit
- Avoids redundant BigFloat computation

```
compute_reference_orbit(full_max_iter)  // once
for pass in [1/16, 1/8, 1/4, full]:
    gpu_render(orbit, pass_resolution, pass_max_iter)
```

---

## Glitch Detection

Enabled only on final pass.

- Early passes use reduced max_iter → different glitch behavior
- Glitches are for CPU re-render, only makes sense after final pass
- Pass 1-3: `tau_sq = 0.0` (disables glitch check)
- Pass 4: `tau_sq = config.tau_sq` (normal detection)

---

## Pass Definition

```rust
#[derive(Clone, Copy)]
pub enum Pass {
    Preview16,  // 1/16 resolution, 1/16 iterations
    Preview8,   // 1/8 resolution, 1/8 iterations
    Preview4,   // 1/4 resolution, 1/4 iterations
    Full,       // Full resolution, full iterations
}

impl Pass {
    pub fn all() -> [Pass; 4] {
        [Pass::Preview16, Pass::Preview8, Pass::Preview4, Pass::Full]
    }

    pub fn scale(&self) -> u32 {
        match self {
            Pass::Preview16 => 16,
            Pass::Preview8 => 8,
            Pass::Preview4 => 4,
            Pass::Full => 1,
        }
    }

    pub fn dimensions(&self, canvas_w: u32, canvas_h: u32) -> (u32, u32) {
        let s = self.scale();
        ((canvas_w + s - 1) / s, (canvas_h + s - 1) / s)
    }

    pub fn scale_iterations(&self, max_iter: u32) -> u32 {
        (max_iter / self.scale()).max(100)  // Floor of 100 iterations
    }

    pub fn is_final(&self) -> bool {
        matches!(self, Pass::Full)
    }
}
```

---

## Integration with ParallelRenderer

**New fields:**
```rust
pub struct ParallelRenderer {
    // ... existing fields ...

    /// Current render generation (for interruption)
    render_generation: Rc<Cell<u32>>,
}
```

**Modified flow in `start_gpu_render`:**
```rust
fn start_gpu_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
    let gen = self.render_generation.get() + 1;
    self.render_generation.set(gen);

    let generation = Rc::clone(&self.render_generation);
    let gpu_renderer = Rc::clone(&self.gpu_renderer);
    let tile_results = Rc::clone(&self.tile_results);
    // ...

    self.worker_pool.borrow().set_orbit_complete_callback(move |orbit_data| {
        spawn_local(async move {
            for pass in Pass::all() {
                if generation.get() != gen { return; }

                let small_data = gpu.render_pass(&orbit_data, pass).await?;
                let full_data = stretch_compute_data(&small_data, pass);

                *tile_results.borrow_mut() = vec![TileResult { data: full_data, ... }];

                colorize_and_draw(&tile_results, &canvas_ctx);
            }
        });
    });
}
```

**Colorizer integration unchanged:** `recolorize()` works exactly as before.

---

## Acceptance Criteria

- [ ] User sees blocky preview within 50ms of render start (pass 1)
- [ ] Full render (pass 4) matches single-pass GPU output exactly
- [ ] Navigation interrupts in-progress renders without visual artifacts
- [ ] Colorizer change triggers instant recolorization (no recompute)
- [ ] Memory stable across many interrupt/restart cycles
- [ ] Performance overhead of multi-pass vs single-pass < 5%

---

## Test Strategy

| Test | Validation |
|------|------------|
| **Pass equivalence** | Pass 4 iteration counts identical to single-pass render |
| **Stretching correctness** | stretched[x,y] == small[x/scale, y/scale] for all pixels |
| **Interruption safety** | 100 rapid navigations → no memory growth, no panics |
| **Colorizer independence** | Switch colorizer mid-render → correct recolorization |
| **Timing targets** | Pass 1 < 50ms, passes 1-3 combined < 500ms at 4K |
| **Generation counter** | Stale render never draws to canvas after interruption |

---

## Summary

| Decision | Choice |
|----------|--------|
| Pass schedule | Fixed 4 passes (1/16, 1/8, 1/4, full) |
| Interruption | Generation counter (lock-free) |
| Stretching | ComputeData stretched before colorizer |
| Colorizer timing | After each pass |
| Iteration scaling | Both resolution and max_iter scale |
| Reference orbit | Compute once, reuse all passes |
| GPU buffers | Allocate max size, reuse |
| Glitch detection | Final pass only |
