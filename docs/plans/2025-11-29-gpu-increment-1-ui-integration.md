# GPU Increment 1: UI Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate GPU renderer into UI so GPU-accelerated rendering is visible in the browser.

**Architecture:** `GpuRenderer` lives inside `ParallelRenderer`, lazily initialized. GPU hooks into `ReferenceOrbitComplete` handler, renders all pixels at once, returns `Vec<ComputeData>` matching CPU output. Silent fallback to CPU on failure.

**Tech Stack:** wgpu, Leptos, wasm-bindgen-futures

**Design Document:** `docs/plans/2025-11-29-gpu-increment-1-ui-integration-design.md`

---

## Task 1: Add fractalwonder-core Dependency to GPU Crate

**Files:**
- Modify: `fractalwonder-gpu/Cargo.toml`

**Step 1: Move fractalwonder-core from dev-dependencies to dependencies**

Edit `fractalwonder-gpu/Cargo.toml`:

```toml
[dependencies]
wgpu = "23.0"
bytemuck = { version = "1.14", features = ["derive"] }
futures-channel = "0.3"
log = "0.4"
thiserror = "1.0"
fractalwonder-core = { path = "../fractalwonder-core" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Window", "Performance"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
pollster = "0.4"

[dev-dependencies]
fractalwonder-compute = { path = "../fractalwonder-compute" }
```

**Step 2: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/Cargo.toml
git commit -m "build(gpu): add fractalwonder-core dependency"
```

---

## Task 2: Update GpuRenderResult to Return Vec<ComputeData>

**Files:**
- Modify: `fractalwonder-gpu/src/renderer.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Update GpuRenderResult struct**

In `fractalwonder-gpu/src/renderer.rs`, replace the struct and update imports:

```rust
//! High-level GPU renderer API.

use crate::buffers::{GpuBuffers, Uniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::pipeline::GpuPipeline;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a GPU render operation.
pub struct GpuRenderResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

impl GpuRenderResult {
    pub fn has_glitches(&self) -> bool {
        self.data.iter().any(|d| match d {
            ComputeData::Mandelbrot(m) => m.glitched,
            _ => false,
        })
    }

    pub fn glitched_pixel_count(&self) -> usize {
        self.data
            .iter()
            .filter(|d| match d {
                ComputeData::Mandelbrot(m) => m.glitched,
                _ => false,
            })
            .count()
    }
}
```

**Step 2: Update render() return conversion**

In `fractalwonder-gpu/src/renderer.rs`, update the end of the `render()` method (after reading back buffers):

```rust
        // Read back results
        let iterations = self
            .read_buffer(&buffers.staging_results, pixel_count)
            .await?;
        let glitch_data = self
            .read_buffer(&buffers.staging_glitches, pixel_count)
            .await?;

        // Convert to ComputeData (same format as CPU)
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(glitch_data.iter())
            .map(|(&iter, &glitch_flag)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations,
                    glitched: glitch_flag != 0,
                })
            })
            .collect();

        let end = Self::now();

        Ok(GpuRenderResult {
            data,
            compute_time_ms: end - start,
        })
```

**Step 3: Update lib.rs exports**

In `fractalwonder-gpu/src/lib.rs`:

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pipeline;
mod renderer;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src
git commit -m "feat(gpu): return Vec<ComputeData> from GpuRenderResult"
```

---

## Task 3: Update GPU Tests for New Return Type

**Files:**
- Modify: `fractalwonder-gpu/src/tests.rs`

**Step 1: Update test assertions**

Replace assertions that use old `.iterations` and `.glitch_flags` with new `.data` field:

```rust
//! Tests for GPU renderer - verifies GPU output matches CPU perturbation.

use crate::{GpuAvailability, GpuContext, GpuRenderer};
use fractalwonder_compute::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{BigFloat, ComputeData, MandelbrotData};

/// Helper to create a reference orbit at a given center point.
fn create_reference_orbit(center_re: f64, center_im: f64, max_iter: u32) -> ReferenceOrbit {
    let precision = 128;
    let c_ref = (
        BigFloat::with_precision(center_re, precision),
        BigFloat::with_precision(center_im, precision),
    );
    ReferenceOrbit::compute(&c_ref, max_iter)
}

/// Extract MandelbrotData from ComputeData, panics if wrong variant.
fn as_mandelbrot(data: &ComputeData) -> &MandelbrotData {
    match data {
        ComputeData::Mandelbrot(m) => m,
        _ => panic!("Expected Mandelbrot data"),
    }
}

/// Test that GPU initialization doesn't panic.
#[test]
fn gpu_init_does_not_panic() {
    pollster::block_on(async {
        let result = GpuContext::try_init().await;
        match result {
            GpuAvailability::Available(_) => {
                println!("GPU available");
            }
            GpuAvailability::Unavailable(reason) => {
                println!("GPU unavailable: {reason}");
            }
        }
    });
}

/// Verify GPU iteration counts match CPU for a grid of test points.
#[test]
fn gpu_matches_cpu_iteration_counts() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        let center_re = -0.5;
        let center_im = 0.0;
        let max_iter = 256;
        let tau_sq = 1e-6_f32;
        let width = 64_u32;
        let height = 64_u32;

        let orbit = create_reference_orbit(center_re, center_im, max_iter);

        let view_width = 3.0_f32;
        let view_height = 3.0_f32;
        let dc_origin = (-view_width / 2.0, -view_height / 2.0);
        let dc_step = (view_width / width as f32, view_height / height as f32);

        let gpu_result = renderer
            .render(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                max_iter,
                tau_sq,
            )
            .await
            .expect("GPU render should succeed");

        let mut matches = 0;
        let mut mismatches = 0;
        let mut max_diff = 0_i32;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                let delta_c = (
                    dc_origin.0 as f64 + x as f64 * dc_step.0 as f64,
                    dc_origin.1 as f64 + y as f64 * dc_step.1 as f64,
                );

                let cpu_result =
                    compute_pixel_perturbation(&orbit, delta_c, max_iter, tau_sq as f64);

                let gpu_data = as_mandelbrot(&gpu_result.data[idx]);
                let gpu_iter = gpu_data.iterations;
                let cpu_iter = cpu_result.iterations;

                let diff = (gpu_iter as i32 - cpu_iter as i32).abs();
                max_diff = max_diff.max(diff);

                if diff <= 1 {
                    matches += 1;
                } else {
                    mismatches += 1;
                    if mismatches <= 5 {
                        println!(
                            "Mismatch at ({x}, {y}): GPU={gpu_iter}, CPU={cpu_iter}, diff={diff}"
                        );
                    }
                }
            }
        }

        let total = width * height;
        let match_pct = 100.0 * matches as f64 / total as f64;

        println!("GPU vs CPU comparison:");
        println!("  Total pixels: {total}");
        println!("  Matches (±1): {matches} ({match_pct:.1}%)");
        println!("  Mismatches: {mismatches}");
        println!("  Max iteration difference: {max_diff}");

        assert!(
            match_pct >= 99.0,
            "GPU should match CPU for at least 99% of pixels, got {match_pct:.1}%"
        );
        assert!(
            max_diff <= 5,
            "Maximum iteration difference should be ≤5, got {max_diff}"
        );
    });
}

/// Verify glitch detection flags are set correctly.
#[test]
fn gpu_glitch_detection_works() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        let max_iter = 500;
        let orbit = create_reference_orbit(-0.5, 0.0, max_iter);

        let width = 100;
        let height = 100;

        let dc_origin = (-2.0_f32, -1.5_f32);
        let dc_step = (3.0 / width as f32, 3.0 / height as f32);

        let gpu_result = renderer
            .render(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                max_iter,
                1e-6,
            )
            .await
            .expect("GPU render should succeed");

        let glitch_count = gpu_result.glitched_pixel_count();
        let total = (width * height) as usize;

        println!("Glitch detection test:");
        println!("  Total pixels: {total}");
        println!("  Glitched pixels: {glitch_count}");
        println!(
            "  Glitch rate: {:.1}%",
            100.0 * glitch_count as f64 / total as f64
        );

        assert!(glitch_count < total, "Not all pixels should be glitched");
    });
}

/// Test that known in-set points reach max iterations.
#[test]
fn gpu_in_set_points_reach_max_iter() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        let max_iter = 100;
        let orbit = create_reference_orbit(0.0, 0.0, max_iter);

        let width = 1;
        let height = 1;
        let dc_origin = (0.0_f32, 0.0_f32);
        let dc_step = (0.0, 0.0);

        let gpu_result = renderer
            .render(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                max_iter,
                1e-6,
            )
            .await
            .expect("GPU render should succeed");

        let gpu_data = as_mandelbrot(&gpu_result.data[0]);

        println!(
            "In-set test: origin reached {} iterations (max={max_iter})",
            gpu_data.iterations
        );

        assert_eq!(
            gpu_data.iterations, max_iter,
            "Origin should reach max_iter={max_iter}, got {}",
            gpu_data.iterations
        );
        assert!(
            !gpu_data.escaped,
            "Origin should not escape"
        );
    });
}

/// Test that known escaping points escape quickly.
#[test]
fn gpu_escaping_points_escape_quickly() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        let max_iter = 100;
        let orbit = create_reference_orbit(0.0, 0.0, max_iter);

        let width = 1;
        let height = 1;
        let dc_origin = (3.0_f32, 0.0_f32);
        let dc_step = (0.0, 0.0);

        let gpu_result = renderer
            .render(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                max_iter,
                1e-6,
            )
            .await
            .expect("GPU render should succeed");

        let gpu_data = as_mandelbrot(&gpu_result.data[0]);

        println!("Escape test: c=3+0i escaped at iteration {}", gpu_data.iterations);

        assert!(
            gpu_data.iterations < 5,
            "Point at c=3+0i should escape within 5 iterations, got {}",
            gpu_data.iterations
        );
        assert!(
            gpu_data.escaped,
            "Point at c=3+0i should be marked as escaped"
        );
    });
}
```

**Step 2: Run tests**

```bash
cargo test -p fractalwonder-gpu -- --nocapture
```

Expected: All tests pass (or skip gracefully if no GPU).

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/tests.rs
git commit -m "test(gpu): update tests for Vec<ComputeData> return type"
```

---

## Task 4: Add fractalwonder-gpu to Workspace and UI

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `fractalwonder-ui/Cargo.toml`

**Step 1: Add fractalwonder-gpu to workspace dependencies**

In root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
[workspace.dependencies]
# Core crates
fractalwonder-core = { path = "./fractalwonder-core" }
fractalwonder-compute = { path = "./fractalwonder-compute" }
fractalwonder-gpu = { path = "./fractalwonder-gpu" }
```

**Step 2: Add fractalwonder-gpu dependency to UI**

In `fractalwonder-ui/Cargo.toml`, add to `[dependencies]`:

```toml
[dependencies]
fractalwonder-core = { workspace = true }
fractalwonder-compute = { workspace = true }
fractalwonder-gpu = { workspace = true }
```

Also add `wasm-bindgen-futures` for async GPU operations:

```toml
wasm-bindgen-futures = "0.4"
```

**Step 3: Verify compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add Cargo.toml fractalwonder-ui/Cargo.toml
git commit -m "build(ui): add fractalwonder-gpu dependency"
```

---

## Task 5: Add draw_full_frame to canvas_utils

**Files:**
- Modify: `fractalwonder-ui/src/rendering/canvas_utils.rs`

**Step 1: Add draw_full_frame function**

Add after `draw_pixels_to_canvas`:

```rust
/// Draw an entire frame to canvas (for GPU results).
///
/// Unlike draw_pixels_to_canvas which draws at an offset for tiles,
/// this draws the full image starting at (0, 0).
pub fn draw_full_frame(
    ctx: &CanvasRenderingContext2d,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<(), JsValue> {
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(pixels),
        width,
        height,
    )?;
    ctx.put_image_data(&image_data, 0.0, 0.0)
}
```

**Step 2: Verify compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/canvas_utils.rs
git commit -m "feat(ui): add draw_full_frame for GPU results"
```

---

## Task 6: Add GPU Fields to ParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add imports and new fields**

At the top of the file, add imports:

```rust
use fractalwonder_gpu::{GpuAvailability, GpuContext, GpuRenderer, GpuRenderResult};
use std::cell::Cell;
```

Add new fields to `ParallelRenderer` struct:

```rust
pub struct ParallelRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>>,
    xray_enabled: Rc<Cell<bool>>,
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    /// GPU renderer, lazily initialized when gpu_enabled
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,
    /// Whether GPU initialization has been attempted
    gpu_init_attempted: Rc<Cell<bool>>,
    /// Canvas dimensions for GPU rendering
    canvas_size: Rc<Cell<(u32, u32)>>,
}
```

**Step 2: Initialize new fields in constructor**

In `ParallelRenderer::new()`, add initialization:

```rust
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let xray_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let tile_results: Rc<RefCell<Vec<TileResult>>> = Rc::new(RefCell::new(Vec::new()));
        let gpu_renderer: Rc<RefCell<Option<GpuRenderer>>> = Rc::new(RefCell::new(None));
        let gpu_init_attempted: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let canvas_size: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));

        // ... existing tile callback code ...

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
        })
    }
```

**Step 3: Add GPU initialization helper**

Add method to `impl ParallelRenderer`:

```rust
    /// Initialize GPU renderer if not already attempted.
    /// Returns true if GPU is available after this call.
    async fn try_init_gpu(&self) -> bool {
        if self.gpu_init_attempted.get() {
            return self.gpu_renderer.borrow().is_some();
        }

        self.gpu_init_attempted.set(true);

        match GpuContext::try_init().await {
            GpuAvailability::Available(ctx) => {
                log::info!("GPU renderer initialized successfully");
                *self.gpu_renderer.borrow_mut() = Some(GpuRenderer::new(ctx));
                true
            }
            GpuAvailability::Unavailable(reason) => {
                log::warn!("GPU unavailable: {reason}, using CPU fallback");
                false
            }
        }
    }
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles (may have warnings about unused fields, that's fine for now).

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): add GPU fields to ParallelRenderer"
```

---

## Task 7: Implement GPU Render Path

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add GPU render method**

Add new method to `impl ParallelRenderer`:

```rust
    /// Attempt GPU render for the entire frame.
    /// Returns Ok(()) if GPU succeeded, Err(reason) if should fall back to CPU.
    async fn try_gpu_render(
        &self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        viewport: &Viewport,
        width: u32,
        height: u32,
        max_iterations: u32,
    ) -> Result<(), String> {
        let mut gpu_ref = self.gpu_renderer.borrow_mut();
        let gpu = gpu_ref.as_mut().ok_or("GPU not initialized")?;

        // Calculate delta-c origin and step
        let vp_width = viewport.width.to_f64() as f32;
        let vp_height = viewport.height.to_f64() as f32;

        let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
        let dc_step = (vp_width / width as f32, vp_height / height as f32);

        let tau_sq = self.config.tau_sq as f32;

        // GPU render
        let result = gpu
            .render(
                orbit,
                orbit_id,
                dc_origin,
                dc_step,
                width,
                height,
                max_iterations,
                tau_sq,
            )
            .await
            .map_err(|e| format!("GPU render failed: {e}"))?;

        log::info!(
            "GPU render complete: {}x{} pixels in {:.1}ms, {} glitched",
            width,
            height,
            result.compute_time_ms,
            result.glitched_pixel_count()
        );

        // Colorize and draw
        let xray = self.xray_enabled.get();
        let pixels: Vec<u8> = result
            .data
            .iter()
            .flat_map(|d| colorize(d, xray))
            .collect();

        if let Some(ctx) = self.canvas_ctx.borrow().as_ref() {
            crate::rendering::canvas_utils::draw_full_frame(ctx, &pixels, width, height)
                .map_err(|e| format!("Draw failed: {e:?}"))?;
        }

        // Store results for re-colorizing
        // Convert to TileResult covering entire canvas for consistency
        let tile = PixelRect {
            x: 0,
            y: 0,
            width,
            height,
        };
        self.tile_results.borrow_mut().clear();
        self.tile_results.borrow_mut().push(TileResult {
            tile,
            data: result.data,
            compute_time_ms: result.compute_time_ms,
        });

        // Mark progress complete
        self.progress.update(|p| {
            p.completed_tiles = p.total_tiles;
            p.is_complete = true;
        });

        Ok(())
    }
```

**Step 2: Add render_with_gpu method**

Add method that spawns async GPU render:

```rust
    /// Start GPU-accelerated render if available.
    /// Falls back to CPU tile rendering if GPU fails.
    pub fn render_with_gpu(
        &self,
        viewport: &Viewport,
        canvas: &HtmlCanvasElement,
        orbit: Vec<(f64, f64)>,
        orbit_id: u32,
        max_iterations: u32,
    ) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Store canvas context
        if let Ok(ctx) = get_2d_context(canvas) {
            *self.canvas_ctx.borrow_mut() = Some(ctx);
        }

        self.canvas_size.set((width, height));

        // Set up progress for GPU (1 "tile" = whole frame)
        self.progress.set(RenderProgress::new(1));

        // Clone what we need for the async block
        let gpu_renderer = Rc::clone(&self.gpu_renderer);
        let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
        let canvas_ctx = Rc::clone(&self.canvas_ctx);
        let xray_enabled = Rc::clone(&self.xray_enabled);
        let tile_results = Rc::clone(&self.tile_results);
        let progress = self.progress;
        let config = self.config;
        let worker_pool = Rc::clone(&self.worker_pool);
        let viewport_clone = viewport.clone();
        let tiles = generate_tiles(width, height, calculate_tile_size(1.0));

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

            // Try GPU render
            let gpu_result = {
                let mut gpu_ref = gpu_renderer.borrow_mut();
                if let Some(gpu) = gpu_ref.as_mut() {
                    let vp_width = viewport_clone.width.to_f64() as f32;
                    let vp_height = viewport_clone.height.to_f64() as f32;
                    let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
                    let dc_step = (vp_width / width as f32, vp_height / height as f32);
                    let tau_sq = config.tau_sq as f32;

                    Some(
                        gpu.render(
                            &orbit,
                            orbit_id,
                            dc_origin,
                            dc_step,
                            width,
                            height,
                            max_iterations,
                            tau_sq,
                        )
                        .await,
                    )
                } else {
                    None
                }
            };

            match gpu_result {
                Some(Ok(result)) => {
                    log::info!(
                        "GPU render: {}x{} in {:.1}ms",
                        width,
                        height,
                        result.compute_time_ms
                    );

                    let xray = xray_enabled.get();
                    let pixels: Vec<u8> = result
                        .data
                        .iter()
                        .flat_map(|d| colorize(d, xray))
                        .collect();

                    if let Some(ctx) = canvas_ctx.borrow().as_ref() {
                        let _ = crate::rendering::canvas_utils::draw_full_frame(
                            ctx, &pixels, width, height,
                        );
                    }

                    // Store for recolorize
                    tile_results.borrow_mut().clear();
                    tile_results.borrow_mut().push(TileResult {
                        tile: PixelRect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        data: result.data,
                        compute_time_ms: result.compute_time_ms,
                    });

                    progress.update(|p| {
                        p.completed_tiles = 1;
                        p.is_complete = true;
                    });
                }
                Some(Err(e)) => {
                    log::warn!("GPU render failed: {e}, falling back to CPU");
                    // Fall back to CPU
                    worker_pool
                        .borrow_mut()
                        .start_perturbation_render(viewport_clone, (width, height), tiles);
                }
                None => {
                    log::info!("No GPU available, using CPU");
                    worker_pool
                        .borrow_mut()
                        .start_perturbation_render(viewport_clone, (width, height), tiles);
                }
            }
        });
    }
```

**Step 3: Add necessary imports at top of file**

Make sure these imports are present:

```rust
use crate::rendering::canvas_utils::{draw_full_frame, get_2d_context};
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use fractalwonder_core::PixelRect;
use fractalwonder_gpu::{GpuAvailability, GpuContext, GpuRenderer};
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): implement GPU render path with CPU fallback"
```

---

## Task 8: Hook GPU into WorkerPool Orbit Complete

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add method to expose orbit for GPU**

Add a method that returns orbit data when complete:

```rust
    /// Get the current reference orbit if available.
    pub fn get_orbit(&self) -> Option<(Vec<(f64, f64)>, u32)> {
        self.perturbation.pending_orbit.as_ref().map(|o| {
            (o.orbit.clone(), self.perturbation.orbit_id)
        })
    }

    /// Get max iterations for current render.
    pub fn get_max_iterations(&self) -> u32 {
        self.perturbation.max_iterations
    }
```

**Step 2: Verify compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles with no errors.

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat(ui): expose orbit data from WorkerPool for GPU"
```

---

## Task 9: Verify Full Build and Run Tests

**Files:** None (verification only)

**Step 1: Format code**

```bash
cargo fmt --all
```

**Step 2: Run Clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: No warnings.

**Step 3: Run all tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All tests pass.

**Step 4: Build for WASM**

```bash
trunk build
```

Expected: Build succeeds.

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Task 10: Manual Browser Test

**Files:** None (manual verification)

**Step 1: Start dev server**

```bash
trunk serve
```

**Step 2: Open browser**

Navigate to `http://localhost:8080`

**Step 3: Verify GPU rendering**

1. Open browser console (F12)
2. Navigate to Mandelbrot view
3. Look for console log: "GPU renderer initialized"
4. Look for console log: "GPU render: WxH in X.Xms"
5. Verify fractal renders correctly

**Step 4: Verify fallback**

1. In `fractalwonder-ui/src/config.rs`, set `gpu_enabled: false` for mandelbrot
2. Rebuild and verify CPU tile rendering still works
3. Restore `gpu_enabled: true`

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(gpu): complete GPU Increment 1 UI integration"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Add core dependency to GPU crate | `fractalwonder-gpu/Cargo.toml` |
| 2 | Return Vec<ComputeData> from GPU | `renderer.rs` |
| 3 | Update GPU tests | `tests.rs` |
| 4 | Add GPU to workspace and UI | `Cargo.toml` files |
| 5 | Add draw_full_frame | `canvas_utils.rs` |
| 6 | Add GPU fields to ParallelRenderer | `parallel_renderer.rs` |
| 7 | Implement GPU render path | `parallel_renderer.rs` |
| 8 | Expose orbit from WorkerPool | `worker_pool.rs` |
| 9 | Full build verification | - |
| 10 | Manual browser test | - |

**After completion:** GPU renders visible in browser. Mandelbrot renders via GPU when available, falls back to CPU if not.
