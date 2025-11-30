# GPU Increment 2: Progressive Multi-Pass Rendering - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add progressive multi-pass rendering to GPU path for responsive visual feedback.

**Architecture:** GPU renders at 4 resolution levels (1/16, 1/8, 1/4, full). Each pass reads back small ComputeData buffer, stretches to full canvas size, stores, then colorizes. Generation counter enables clean interruption on navigation.

**Tech Stack:** Rust, wgpu, Leptos, WASM

**Design Document:** `docs/plans/2025-11-29-gpu-increment-2-progressive-rendering-design.md`

---

## Task 1: Add Pass Enum to GPU Crate

**Files:**
- Create: `fractalwonder-gpu/src/pass.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create pass.rs with Pass enum**

```rust
// fractalwonder-gpu/src/pass.rs

/// Defines the 4 progressive rendering passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// 1/16 resolution, 1/16 iterations
    Preview16,
    /// 1/8 resolution, 1/8 iterations
    Preview8,
    /// 1/4 resolution, 1/4 iterations
    Preview4,
    /// Full resolution, full iterations
    Full,
}

impl Pass {
    /// Returns all passes in order.
    pub fn all() -> [Pass; 4] {
        [Pass::Preview16, Pass::Preview8, Pass::Preview4, Pass::Full]
    }

    /// Returns the scale factor (16, 8, 4, or 1).
    pub fn scale(&self) -> u32 {
        match self {
            Pass::Preview16 => 16,
            Pass::Preview8 => 8,
            Pass::Preview4 => 4,
            Pass::Full => 1,
        }
    }

    /// Computes pass dimensions from canvas dimensions.
    pub fn dimensions(&self, canvas_w: u32, canvas_h: u32) -> (u32, u32) {
        let s = self.scale();
        ((canvas_w + s - 1) / s, (canvas_h + s - 1) / s)
    }

    /// Scales max iterations for this pass (floor of 100).
    pub fn scale_iterations(&self, max_iter: u32) -> u32 {
        (max_iter / self.scale()).max(100)
    }

    /// Returns true if this is the final (full resolution) pass.
    pub fn is_final(&self) -> bool {
        matches!(self, Pass::Full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions() {
        assert_eq!(Pass::Preview16.dimensions(3840, 2160), (240, 135));
        assert_eq!(Pass::Preview8.dimensions(3840, 2160), (480, 270));
        assert_eq!(Pass::Preview4.dimensions(3840, 2160), (960, 540));
        assert_eq!(Pass::Full.dimensions(3840, 2160), (3840, 2160));
    }

    #[test]
    fn test_dimensions_rounding() {
        // 1000 / 16 = 62.5, should round up to 63
        assert_eq!(Pass::Preview16.dimensions(1000, 1000), (63, 63));
    }

    #[test]
    fn test_scale_iterations() {
        assert_eq!(Pass::Preview16.scale_iterations(16000), 1000);
        assert_eq!(Pass::Preview16.scale_iterations(1600), 100);
        assert_eq!(Pass::Preview16.scale_iterations(160), 100); // Floor of 100
        assert_eq!(Pass::Full.scale_iterations(16000), 16000);
    }

    #[test]
    fn test_is_final() {
        assert!(!Pass::Preview16.is_final());
        assert!(!Pass::Preview8.is_final());
        assert!(!Pass::Preview4.is_final());
        assert!(Pass::Full.is_final());
    }
}
```

**Step 2: Run tests to verify**

Run: `cargo test -p fractalwonder-gpu pass`
Expected: All 4 tests pass

**Step 3: Add module to lib.rs**

Modify `fractalwonder-gpu/src/lib.rs`:

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pass;
mod pipeline;
mod renderer;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pass::Pass;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 4: Run full crate tests**

Run: `cargo test -p fractalwonder-gpu`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/pass.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add Pass enum for progressive rendering"
```

---

## Task 2: Add render_pass Method to GpuRenderer

**Files:**
- Modify: `fractalwonder-gpu/src/renderer.rs`

**Step 1: Add render_pass method**

Add this method to `GpuRenderer` impl block in `fractalwonder-gpu/src/renderer.rs`, after the existing `render` method:

```rust
    /// Render a single pass at reduced resolution/iterations.
    ///
    /// Returns ComputeData for `pass.dimensions()` pixels, NOT full canvas size.
    /// Caller is responsible for stretching to full size.
    #[allow(clippy::too_many_arguments)]
    pub async fn render_pass(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: (f32, f32),
        viewport_width: f32,
        viewport_height: f32,
        canvas_width: u32,
        canvas_height: u32,
        max_iterations: u32,
        tau_sq: f32,
        pass: crate::Pass,
    ) -> Result<GpuRenderResult, GpuError> {
        let (pass_w, pass_h) = pass.dimensions(canvas_width, canvas_height);
        let pass_max_iter = pass.scale_iterations(max_iterations);
        let pass_tau_sq = if pass.is_final() { tau_sq } else { 0.0 };

        // Compute dc_step for this pass resolution
        let dc_step = (
            viewport_width / pass_w as f32,
            viewport_height / pass_h as f32,
        );

        self.render(
            orbit,
            orbit_id,
            dc_origin,
            dc_step,
            pass_w,
            pass_h,
            pass_max_iter,
            pass_tau_sq,
        )
        .await
    }
```

**Step 2: Add import for Pass at top of file**

Add to imports at top of `fractalwonder-gpu/src/renderer.rs`:

```rust
use crate::Pass;
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-gpu`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/renderer.rs
git commit -m "feat(gpu): add render_pass method for progressive rendering"
```

---

## Task 3: Add stretch_compute_data Function

**Files:**
- Create: `fractalwonder-gpu/src/stretch.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create stretch.rs with function and tests**

```rust
// fractalwonder-gpu/src/stretch.rs

use fractalwonder_core::ComputeData;

/// Stretches a small ComputeData buffer to full canvas size by duplicating pixels.
///
/// Each source pixel becomes a `scale × scale` block in the output.
/// Output is in row-major order, suitable for colorization.
pub fn stretch_compute_data(
    small: &[ComputeData],
    small_w: u32,
    small_h: u32,
    scale: u32,
) -> Vec<ComputeData> {
    debug_assert_eq!(
        small.len(),
        (small_w * small_h) as usize,
        "Input size mismatch"
    );

    if scale == 1 {
        return small.to_vec();
    }

    let full_w = small_w * scale;
    let full_h = small_h * scale;
    let mut full = Vec::with_capacity((full_w * full_h) as usize);

    for sy in 0..small_h {
        // For each row in the small image, we output `scale` rows
        for _dy in 0..scale {
            for sx in 0..small_w {
                let src = &small[(sy * small_w + sx) as usize];
                // Duplicate this pixel `scale` times horizontally
                for _dx in 0..scale {
                    full.push(src.clone());
                }
            }
        }
    }

    full
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::MandelbrotData;

    fn make_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 1000,
            escaped: iterations < 1000,
            glitched: false,
        })
    }

    #[test]
    fn test_stretch_scale_1() {
        let small = vec![make_data(10), make_data(20), make_data(30), make_data(40)];
        let result = stretch_compute_data(&small, 2, 2, 1);
        assert_eq!(result.len(), 4);
        assert_eq!(result, small);
    }

    #[test]
    fn test_stretch_scale_2() {
        // 2x2 input, scale 2 -> 4x4 output
        let small = vec![make_data(1), make_data(2), make_data(3), make_data(4)];
        let result = stretch_compute_data(&small, 2, 2, 2);
        assert_eq!(result.len(), 16);

        // Expected layout:
        // 1 1 2 2
        // 1 1 2 2
        // 3 3 4 4
        // 3 3 4 4
        let expected_iters: Vec<u32> = vec![
            1, 1, 2, 2, // row 0
            1, 1, 2, 2, // row 1
            3, 3, 4, 4, // row 2
            3, 3, 4, 4, // row 3
        ];

        let actual_iters: Vec<u32> = result
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => m.iterations,
                _ => panic!("Expected Mandelbrot"),
            })
            .collect();

        assert_eq!(actual_iters, expected_iters);
    }

    #[test]
    fn test_stretch_scale_16() {
        // 1x1 input, scale 16 -> 16x16 output
        let small = vec![make_data(42)];
        let result = stretch_compute_data(&small, 1, 1, 16);
        assert_eq!(result.len(), 256);

        // All pixels should have iterations = 42
        for d in &result {
            match d {
                ComputeData::Mandelbrot(m) => assert_eq!(m.iterations, 42),
                _ => panic!("Expected Mandelbrot"),
            }
        }
    }

    #[test]
    fn test_stretch_preserves_glitch_flag() {
        let small = vec![ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 1000,
            escaped: true,
            glitched: true,
        })];
        let result = stretch_compute_data(&small, 1, 1, 4);
        assert_eq!(result.len(), 16);

        for d in &result {
            match d {
                ComputeData::Mandelbrot(m) => {
                    assert!(m.glitched);
                    assert!(m.escaped);
                }
                _ => panic!("Expected Mandelbrot"),
            }
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-gpu stretch`
Expected: All 4 tests pass

**Step 3: Add module and export to lib.rs**

Modify `fractalwonder-gpu/src/lib.rs`:

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pass;
mod pipeline;
mod renderer;
mod stretch;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pass::Pass;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
pub use stretch::stretch_compute_data;

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 4: Run full crate tests**

Run: `cargo test -p fractalwonder-gpu`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/stretch.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add stretch_compute_data for progressive rendering"
```

---

## Task 4: Add Generation Counter to ParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add render_generation field to ParallelRenderer struct**

In `fractalwonder-ui/src/rendering/parallel_renderer.rs`, add to the struct definition (around line 16-30):

```rust
pub struct ParallelRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>>,
    xray_enabled: Rc<Cell<bool>>,
    /// Stored tile results for re-colorizing without recompute
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    /// GPU renderer, lazily initialized when gpu_enabled
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,
    /// Whether GPU initialization has been attempted
    gpu_init_attempted: Rc<Cell<bool>>,
    /// Canvas dimensions for GPU rendering
    canvas_size: Rc<Cell<(u32, u32)>>,
    /// Render generation counter for interruption handling
    render_generation: Rc<Cell<u32>>,
}
```

**Step 2: Initialize render_generation in new()**

In the `new()` function, add initialization (around line 33-40):

```rust
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let xray_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let tile_results: Rc<RefCell<Vec<TileResult>>> = Rc::new(RefCell::new(Vec::new()));
        let gpu_renderer: Rc<RefCell<Option<GpuRenderer>>> = Rc::new(RefCell::new(None));
        let gpu_init_attempted: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let canvas_size: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));
        let render_generation: Rc<Cell<u32>> = Rc::new(Cell::new(0));
```

And add it to the Self return (around line 67-78):

```rust
        Ok(Self {
            config,
            worker_pool,
            progress,
            canvas_ctx,
            xray_enabled,
            tile_results,
            gpu_renderer,
            gpu_init_attempted,
            canvas_size,
            render_generation,
        })
```

**Step 3: Check compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): add render_generation counter to ParallelRenderer"
```

---

## Task 5: Update Pass Import in UI Crate

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add Pass and stretch_compute_data imports**

At the top of `fractalwonder-ui/src/rendering/parallel_renderer.rs`, update the imports:

```rust
use fractalwonder_gpu::{GpuAvailability, GpuContext, GpuRenderer, Pass, stretch_compute_data};
```

**Step 2: Check compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles (warnings about unused imports are OK for now)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): import Pass and stretch_compute_data from gpu crate"
```

---

## Task 6: Refactor start_gpu_render for Progressive Passes

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Replace start_gpu_render implementation**

Replace the entire `start_gpu_render` method (lines 168-311) with:

```rust
    /// Start GPU-accelerated perturbation render with progressive passes.
    fn start_gpu_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        // Increment generation to invalidate any in-progress renders
        let gen = self.render_generation.get() + 1;
        self.render_generation.set(gen);

        self.canvas_size.set((width, height));

        // Clone what we need for the callback
        let generation = Rc::clone(&self.render_generation);
        let gpu_renderer = Rc::clone(&self.gpu_renderer);
        let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
        let canvas_ctx = Rc::clone(&self.canvas_ctx);
        let xray_enabled = Rc::clone(&self.xray_enabled);
        let tile_results = Rc::clone(&self.tile_results);
        let worker_pool = Rc::clone(&self.worker_pool);
        let progress = self.progress;
        let config = self.config;
        let viewport_clone = viewport.clone();
        let tiles = generate_tiles(width, height, calculate_tile_size(1.0));

        // Set up callback for when orbit is ready
        self.worker_pool.borrow().set_orbit_complete_callback(
            move |orbit_data: OrbitCompleteData| {
                log::info!(
                    "Orbit ready: {} points, starting progressive GPU render",
                    orbit_data.orbit.len()
                );

                // Clone again for the async block
                let generation = Rc::clone(&generation);
                let gpu_renderer = Rc::clone(&gpu_renderer);
                let gpu_init_attempted = Rc::clone(&gpu_init_attempted);
                let canvas_ctx = Rc::clone(&canvas_ctx);
                let xray_enabled = Rc::clone(&xray_enabled);
                let tile_results = Rc::clone(&tile_results);
                let worker_pool = Rc::clone(&worker_pool);
                let viewport = viewport_clone.clone();
                let tiles = tiles.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    // Try GPU init if not attempted
                    if !gpu_init_attempted.get() {
                        gpu_init_attempted.set(true);
                        match GpuContext::try_init().await {
                            GpuAvailability::Available(ctx) => {
                                log::info!("GPU renderer initialized");
                                *gpu_renderer.borrow_mut() = Some(GpuRenderer::new(ctx));
                            }
                            GpuAvailability::Unavailable(reason) => {
                                log::warn!("GPU unavailable: {reason}");
                            }
                        }
                    }

                    // Check if we have GPU
                    let has_gpu = gpu_renderer.borrow().is_some();
                    if !has_gpu {
                        log::info!("No GPU available, using CPU");
                        worker_pool.borrow_mut().start_perturbation_render(
                            viewport,
                            (width, height),
                            tiles,
                        );
                        return;
                    }

                    let vp_width = viewport.width.to_f64() as f32;
                    let vp_height = viewport.height.to_f64() as f32;
                    let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
                    let tau_sq = config.tau_sq as f32;

                    // Progressive rendering: 4 passes
                    for pass in Pass::all() {
                        // Check generation - abort if stale
                        if generation.get() != gen {
                            log::debug!("Render interrupted at {:?}", pass);
                            return;
                        }

                        let pass_result = {
                            let mut gpu_opt = gpu_renderer.borrow_mut();
                            let gpu = gpu_opt.as_mut().unwrap();

                            gpu.render_pass(
                                &orbit_data.orbit,
                                orbit_data.orbit_id,
                                dc_origin,
                                vp_width,
                                vp_height,
                                width,
                                height,
                                orbit_data.max_iterations,
                                tau_sq,
                                pass,
                            )
                            .await
                        };

                        match pass_result {
                            Ok(result) => {
                                let (pass_w, pass_h) = pass.dimensions(width, height);
                                let scale = pass.scale();

                                log::info!(
                                    "Pass {:?}: {}x{} in {:.1}ms",
                                    pass,
                                    pass_w,
                                    pass_h,
                                    result.compute_time_ms
                                );

                                // Stretch to full canvas size
                                let full_data = stretch_compute_data(&result.data, pass_w, pass_h, scale);

                                // Store for recolorize
                                tile_results.borrow_mut().clear();
                                tile_results.borrow_mut().push(TileResult {
                                    tile: PixelRect {
                                        x: 0,
                                        y: 0,
                                        width,
                                        height,
                                    },
                                    data: full_data.clone(),
                                    compute_time_ms: result.compute_time_ms,
                                });

                                // Colorize and draw
                                let xray = xray_enabled.get();
                                let pixels: Vec<u8> =
                                    full_data.iter().flat_map(|d| colorize(d, xray)).collect();

                                if let Some(ctx) = canvas_ctx.borrow().as_ref() {
                                    let _ = draw_full_frame(ctx, &pixels, width, height);
                                }

                                // Update progress
                                if pass.is_final() {
                                    progress.update(|p| {
                                        p.completed_tiles = 1;
                                        p.is_complete = true;
                                    });
                                }
                            }
                            Err(e) => {
                                log::warn!("GPU pass {:?} failed: {e}, falling back to CPU", pass);
                                worker_pool.borrow_mut().start_perturbation_render(
                                    viewport.clone(),
                                    (width, height),
                                    tiles.clone(),
                                );
                                return;
                            }
                        }
                    }
                });
            },
        );

        // Start GPU mode render (computes orbit, triggers callback when ready)
        self.worker_pool
            .borrow_mut()
            .start_perturbation_render_gpu(viewport.clone(), (width, height));
    }
```

**Step 2: Check compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): implement progressive multi-pass GPU rendering"
```

---

## Task 7: Run Full Test Suite

**Step 1: Run cargo fmt**

Run: `cargo fmt --all`
Expected: No output (files formatted)

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings or errors

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features`
Expected: All tests pass

**Step 4: Check WASM build**

Run: `cargo check --target wasm32-unknown-unknown -p fractalwonder-ui`
Expected: Compiles without errors

**Step 5: Commit any formatting fixes**

```bash
git add -A
git commit -m "chore: format code" --allow-empty
```

---

## Task 8: Manual Browser Testing

**Prerequisites:** `trunk serve` running on port 8080

**Step 1: Open browser to http://localhost:8080**

Expected: Fractal renders

**Step 2: Verify progressive rendering**

1. Navigate to a deep zoom location (high iteration count)
2. Observe: Should see blocky preview that sharpens in 3-4 visible steps
3. Check console: Should see logs like:
   - "Pass Preview16: 240x135 in Xms"
   - "Pass Preview8: 480x270 in Xms"
   - "Pass Preview4: 960x540 in Xms"
   - "Pass Full: 3840x2160 in Xms"

**Step 3: Verify interruption**

1. Start a deep zoom render
2. Before it completes, pan or zoom
3. Observe: Previous render should abort, new render should start
4. Check console: Should see "Render interrupted at..." messages

**Step 4: Verify colorizer still works**

1. Complete a render
2. Toggle xray mode (if available) or change colorizer
3. Observe: Should recolorize instantly without recomputing

---

## Task 9: Final Commit

**Step 1: Create summary commit if any uncommitted changes**

```bash
git status
# If clean, skip to step 2
git add -A
git commit -m "chore: final cleanup for GPU Increment 2"
```

**Step 2: Verify all commits**

Run: `git log --oneline -10`

Expected commits (newest first):
- chore: final cleanup (if applicable)
- chore: format code
- feat(ui): implement progressive multi-pass GPU rendering
- feat(ui): import Pass and stretch_compute_data from gpu crate
- feat(ui): add render_generation counter to ParallelRenderer
- feat(gpu): add stretch_compute_data for progressive rendering
- feat(gpu): add render_pass method for progressive rendering
- feat(gpu): add Pass enum for progressive rendering

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add Pass enum | `fractalwonder-gpu/src/pass.rs`, `lib.rs` |
| 2 | Add render_pass method | `fractalwonder-gpu/src/renderer.rs` |
| 3 | Add stretch_compute_data | `fractalwonder-gpu/src/stretch.rs`, `lib.rs` |
| 4 | Add generation counter | `fractalwonder-ui/.../parallel_renderer.rs` |
| 5 | Update imports | `fractalwonder-ui/.../parallel_renderer.rs` |
| 6 | Implement progressive passes | `fractalwonder-ui/.../parallel_renderer.rs` |
| 7 | Run test suite | - |
| 8 | Manual browser testing | - |
| 9 | Final commit | - |

**Estimated time:** 30-45 minutes

**Key acceptance criteria:**
- [ ] Blocky preview visible within ~50ms at 4K
- [ ] Full render matches single-pass output
- [ ] Navigation interrupts cleanly
- [ ] Colorizer change works without recompute
- [ ] All tests pass
