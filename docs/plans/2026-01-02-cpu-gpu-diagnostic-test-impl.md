# CPU/GPU Diagnostic Test Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create integration test that compares CPU and GPU Mandelbrot renderers on identical pixels.

**Architecture:** Single test file using production `render_tile_hdr` (CPU) and `render_row_set` (GPU), printing all `MandelbrotData` fields for 32 pixels.

**Tech Stack:** Rust, pollster (async blocking), fractalwonder-compute, fractalwonder-gpu

---

## Task 1: Create Test File Skeleton

**Files:**
- Create: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Create minimal test file**

```rust
//! Diagnostic test comparing CPU and GPU Mandelbrot renderers.
//!
//! Renders identical pixels through both pipelines and prints MandelbrotData
//! field-by-field to diagnose rendering differences.

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use fractalwonder_compute::perturbation::tile::{render_tile_hdr, TileConfig};
    use fractalwonder_compute::{BlaTable, ReferenceOrbit};
    use fractalwonder_core::{BigFloat, ComputeData, HDRFloat, MandelbrotData};
    use fractalwonder_gpu::{GpuAvailability, GpuContext, ProgressiveGpuRenderer};

    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        println!("CPU/GPU comparison test starting...");
        // TODO: Implement
    }
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS with "CPU/GPU comparison test starting..."

**Step 3: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add cpu/gpu comparison test skeleton"
```

---

## Task 2: Add Viewport and Test Parameters

**Files:**
- Modify: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Add constants and viewport setup**

Add inside `mod tests`:

```rust
    /// Test viewport parameters (extreme deep zoom ~10^-281)
    const CENTER_X: &str = "0.273000307495579097715200094310253922494103490187797182966812629706330340783242";
    const CENTER_Y: &str = "0.005838718497531293679839354462882728828030188792949767250660666951674130465532";
    const WIDTH: &str = "1.38277278476513331960149825811900065907944121299848E-281";
    const HEIGHT: &str = "7.97822331184022584815185255533429968247789646588334E-282";

    const IMAGE_WIDTH: u32 = 766;
    const IMAGE_HEIGHT: u32 = 432;
    const TEST_ROW: u32 = 350;
    const TEST_COL_START: u32 = 580;
    const TEST_COL_END: u32 = 611; // 32 pixels
    const MAX_ITERATIONS: u32 = 10_000_000;
    const TAU_SQ: f64 = 1e-6;
    const PRECISION_BITS: u32 = 1067;

    struct Viewport {
        center_re: BigFloat,
        center_im: BigFloat,
        width: BigFloat,
        height: BigFloat,
    }

    fn parse_viewport() -> Viewport {
        Viewport {
            center_re: BigFloat::parse(CENTER_X, PRECISION_BITS),
            center_im: BigFloat::parse(CENTER_Y, PRECISION_BITS),
            width: BigFloat::parse(WIDTH, PRECISION_BITS),
            height: BigFloat::parse(HEIGHT, PRECISION_BITS),
        }
    }
```

**Step 2: Update test to parse viewport**

Replace test body:

```rust
    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!("Viewport parsed:");
        println!("  width exponent: ~10^{}", viewport.width.log2_approx() as i32 * 301 / 1000);
        println!("  precision: {} bits", PRECISION_BITS);
    }
```

**Step 3: Run test to verify viewport parsing**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS showing "width exponent: ~10^-281"

**Step 4: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add viewport parameters for cpu/gpu comparison"
```

---

## Task 3: Add Reference Orbit and BLA Table Computation

**Files:**
- Modify: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Add orbit computation function**

Add inside `mod tests`:

```rust
    fn compute_orbit_and_bla(viewport: &Viewport) -> (ReferenceOrbit, BlaTable) {
        println!("Computing reference orbit at center...");
        let c_ref = (viewport.center_re.clone(), viewport.center_im.clone());
        let orbit = ReferenceOrbit::compute(&c_ref, MAX_ITERATIONS);
        println!("  Orbit length: {}", orbit.orbit.len());
        println!("  Escaped at: {:?}", orbit.escaped_at);

        // Compute dc_max as half the viewport diagonal (conservative)
        let half_w = HDRFloat::from_bigfloat(&viewport.width).mul(&HDRFloat::from_f64(0.5));
        let half_h = HDRFloat::from_bigfloat(&viewport.height).mul(&HDRFloat::from_f64(0.5));
        let dc_max = half_w.add(&half_h);

        println!("Computing BLA table...");
        let bla_table = BlaTable::compute(&orbit, &dc_max);
        println!("  BLA entries: {}", bla_table.entries.len());
        println!("  BLA levels: {}", bla_table.num_levels);

        (orbit, bla_table)
    }
```

**Step 2: Update test to compute orbit**

Update test body:

```rust
    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!("Viewport parsed (width ~10^{})", viewport.width.log2_approx() as i32 * 301 / 1000);

        let (orbit, bla_table) = compute_orbit_and_bla(&viewport);
        println!("Orbit and BLA ready.");
    }
```

**Step 3: Run test**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS showing orbit length and BLA entries

**Step 4: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add reference orbit and BLA computation"
```

---

## Task 4: Add CPU Rendering

**Files:**
- Modify: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Add CPU rendering function**

Add inside `mod tests`:

```rust
    fn render_cpu_pixels(
        viewport: &Viewport,
        orbit: &ReferenceOrbit,
        bla_table: &BlaTable,
    ) -> Vec<MandelbrotData> {
        println!("Rendering {} CPU pixels...", TEST_COL_END - TEST_COL_START + 1);

        // Compute delta_origin for tile at (TEST_COL_START, TEST_ROW)
        // Matches coordinator.rs:253-262
        let norm_x = TEST_COL_START as f64 / IMAGE_WIDTH as f64 - 0.5;
        let norm_y = TEST_ROW as f64 / IMAGE_HEIGHT as f64 - 0.5;

        let norm_x_bf = BigFloat::with_precision(norm_x, PRECISION_BITS);
        let norm_y_bf = BigFloat::with_precision(norm_y, PRECISION_BITS);

        let delta_origin_re = norm_x_bf.mul(&viewport.width);
        let delta_origin_im = norm_y_bf.mul(&viewport.height);

        let delta_origin = (
            HDRFloat::from_bigfloat(&delta_origin_re),
            HDRFloat::from_bigfloat(&delta_origin_im),
        );

        // Compute delta_step
        // Matches coordinator.rs:185-188
        let step_re = viewport.width.div_f64(IMAGE_WIDTH as f64);
        let step_im = viewport.height.div_f64(IMAGE_HEIGHT as f64);

        let delta_step = (
            HDRFloat::from_bigfloat(&step_re),
            HDRFloat::from_bigfloat(&step_im),
        );

        let config = TileConfig {
            size: (TEST_COL_END - TEST_COL_START + 1, 1), // 32x1 tile
            max_iterations: MAX_ITERATIONS,
            tau_sq: TAU_SQ,
            bla_enabled: true,
        };

        let result = render_tile_hdr(orbit, Some(bla_table), delta_origin, delta_step, &config);

        result
            .data
            .into_iter()
            .filter_map(|cd| match cd {
                ComputeData::Mandelbrot(m) => Some(m),
                _ => None,
            })
            .collect()
    }
```

**Step 2: Update test to render CPU**

Update test body:

```rust
    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!("Viewport parsed (width ~10^{})", viewport.width.log2_approx() as i32 * 301 / 1000);

        let (orbit, bla_table) = compute_orbit_and_bla(&viewport);

        let cpu_pixels = render_cpu_pixels(&viewport, &orbit, &bla_table);
        println!("CPU rendered {} pixels", cpu_pixels.len());
        println!("  First pixel iterations: {}", cpu_pixels[0].iterations);
    }
```

**Step 3: Run test**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS showing "CPU rendered 32 pixels" and iteration count

**Step 4: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add CPU pixel rendering"
```

---

## Task 5: Add GPU Rendering

**Files:**
- Modify: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Add GPU rendering function**

Add inside `mod tests`:

```rust
    async fn render_gpu_pixels(
        viewport: &Viewport,
        orbit: &ReferenceOrbit,
        bla_table: &BlaTable,
    ) -> Option<Vec<MandelbrotData>> {
        println!("Initializing GPU...");
        let ctx = match GpuContext::try_init().await {
            GpuAvailability::Available(ctx) => ctx,
            GpuAvailability::Unavailable(reason) => {
                println!("GPU unavailable: {}", reason);
                return None;
            }
        };

        let mut renderer = ProgressiveGpuRenderer::new(ctx);
        println!("Rendering GPU pixels (full image, extracting row {})...", TEST_ROW);

        // Compute dc_origin and dc_step
        // Matches parallel_renderer.rs:411-431
        let vp_width = HDRFloat::from_bigfloat(&viewport.width);
        let vp_height = HDRFloat::from_bigfloat(&viewport.height);
        let half = HDRFloat::from_f64(0.5);
        let origin_re = vp_width.mul(&half).neg();
        let origin_im = vp_height.mul(&half).neg();
        let step_re = vp_width.div_f64(IMAGE_WIDTH as f64);
        let step_im = vp_height.div_f64(IMAGE_HEIGHT as f64);

        let dc_origin = (
            (origin_re.head, origin_re.tail, origin_re.exp),
            (origin_im.head, origin_im.tail, origin_im.exp),
        );
        let dc_step = (
            (step_re.head, step_re.tail, step_re.exp),
            (step_im.head, step_im.tail, step_im.exp),
        );

        let reference_escaped = orbit.escaped_at.is_some();

        // Render entire image in one row-set to simplify extraction
        let result = renderer
            .render_row_set(
                &orbit.orbit,
                &orbit.derivative,
                1, // orbit_id
                dc_origin,
                dc_step,
                IMAGE_WIDTH,
                IMAGE_HEIGHT,
                0,  // row_set_index
                1,  // row_set_count (all rows in one set)
                MAX_ITERATIONS,
                10000, // iterations_per_dispatch
                TAU_SQ as f32,
                reference_escaped,
                Some(bla_table),
            )
            .await;

        match result {
            Ok(result) => {
                // Extract row TEST_ROW, columns TEST_COL_START..=TEST_COL_END
                let start_idx = (TEST_ROW * IMAGE_WIDTH + TEST_COL_START) as usize;
                let end_idx = (TEST_ROW * IMAGE_WIDTH + TEST_COL_END + 1) as usize;

                let pixels: Vec<MandelbrotData> = result.data[start_idx..end_idx]
                    .iter()
                    .filter_map(|cd| match cd {
                        ComputeData::Mandelbrot(m) => Some(m.clone()),
                        _ => None,
                    })
                    .collect();

                println!("GPU extracted {} pixels from row {}", pixels.len(), TEST_ROW);
                Some(pixels)
            }
            Err(e) => {
                println!("GPU render failed: {:?}", e);
                None
            }
        }
    }
```

**Step 2: Update test to render GPU**

Update test body:

```rust
    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!("Viewport parsed (width ~10^{})", viewport.width.log2_approx() as i32 * 301 / 1000);

        let (orbit, bla_table) = compute_orbit_and_bla(&viewport);

        let cpu_pixels = render_cpu_pixels(&viewport, &orbit, &bla_table);
        println!("CPU rendered {} pixels", cpu_pixels.len());

        let gpu_pixels = pollster::block_on(render_gpu_pixels(&viewport, &orbit, &bla_table));

        match gpu_pixels {
            Some(gpu) => {
                println!("GPU rendered {} pixels", gpu.len());
                println!("  First pixel iterations: CPU={}, GPU={}",
                         cpu_pixels[0].iterations, gpu[0].iterations);
            }
            None => {
                println!("GPU not available, skipping comparison");
            }
        }
    }
```

**Step 3: Run test**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS showing both CPU and GPU pixel counts

**Step 4: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add GPU pixel rendering"
```

---

## Task 6: Add Comparison and Output

**Files:**
- Modify: `fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

**Step 1: Add comparison function**

Add inside `mod tests`:

```rust
    fn print_comparison(col: u32, cpu: &MandelbrotData, gpu: &MandelbrotData) {
        println!("Pixel {}:", col);
        println!(
            "  CPU: iterations={}, max_iterations={}, escaped={}, glitched={},",
            cpu.iterations, cpu.max_iterations, cpu.escaped, cpu.glitched
        );
        println!(
            "       final_z_norm_sq={}, surface_normal_re={}, surface_normal_im={}",
            cpu.final_z_norm_sq, cpu.surface_normal_re, cpu.surface_normal_im
        );
        println!(
            "  GPU: iterations={}, max_iterations={}, escaped={}, glitched={},",
            gpu.iterations, gpu.max_iterations, gpu.escaped, gpu.glitched
        );
        println!(
            "       final_z_norm_sq={}, surface_normal_re={}, surface_normal_im={}",
            gpu.final_z_norm_sq, gpu.surface_normal_re, gpu.surface_normal_im
        );

        let mut diffs = Vec::new();
        if cpu.iterations != gpu.iterations {
            diffs.push(format!(
                "iterations={}",
                (cpu.iterations as i64 - gpu.iterations as i64).abs()
            ));
        }
        if cpu.max_iterations != gpu.max_iterations {
            diffs.push("max_iterations".to_string());
        }
        if cpu.escaped != gpu.escaped {
            diffs.push("escaped".to_string());
        }
        if cpu.glitched != gpu.glitched {
            diffs.push("glitched".to_string());
        }
        if cpu.final_z_norm_sq != gpu.final_z_norm_sq {
            diffs.push(format!(
                "final_z_norm_sq={:.7}",
                (cpu.final_z_norm_sq - gpu.final_z_norm_sq).abs()
            ));
        }
        if cpu.surface_normal_re != gpu.surface_normal_re {
            diffs.push(format!(
                "surface_normal_re={:.7}",
                (cpu.surface_normal_re - gpu.surface_normal_re).abs()
            ));
        }
        if cpu.surface_normal_im != gpu.surface_normal_im {
            diffs.push(format!(
                "surface_normal_im={:.7}",
                (cpu.surface_normal_im - gpu.surface_normal_im).abs()
            ));
        }

        if diffs.is_empty() {
            println!("  (identical)");
        } else {
            println!("  DIFF: {}", diffs.join(", "));
        }
        println!();
    }

    fn compare_all_pixels(cpu_pixels: &[MandelbrotData], gpu_pixels: &[MandelbrotData]) {
        println!("\n========== CPU/GPU COMPARISON ==========\n");

        let mut diff_count = 0;
        for (i, (cpu, gpu)) in cpu_pixels.iter().zip(gpu_pixels.iter()).enumerate() {
            let col = TEST_COL_START + i as u32;
            print_comparison(col, cpu, gpu);

            // Count pixels with any difference
            if cpu.iterations != gpu.iterations
                || cpu.escaped != gpu.escaped
                || cpu.glitched != gpu.glitched
                || cpu.final_z_norm_sq != gpu.final_z_norm_sq
                || cpu.surface_normal_re != gpu.surface_normal_re
                || cpu.surface_normal_im != gpu.surface_normal_im
            {
                diff_count += 1;
            }
        }

        println!("========================================");
        println!(
            "SUMMARY: {} of {} pixels have differences",
            diff_count,
            cpu_pixels.len()
        );
        println!("========================================\n");
    }
```

**Step 2: Update test to run comparison**

Replace test body:

```rust
    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        println!("\n========== CPU/GPU DIAGNOSTIC TEST ==========\n");

        let viewport = parse_viewport();
        println!(
            "Viewport: width ~10^{}, precision {} bits",
            viewport.width.log2_approx() as i32 * 301 / 1000,
            PRECISION_BITS
        );
        println!(
            "Image: {}x{}, testing row {}, cols {}..{}",
            IMAGE_WIDTH, IMAGE_HEIGHT, TEST_ROW, TEST_COL_START, TEST_COL_END
        );
        println!("Max iterations: {}, tau_sq: {}\n", MAX_ITERATIONS, TAU_SQ);

        let (orbit, bla_table) = compute_orbit_and_bla(&viewport);

        let cpu_pixels = render_cpu_pixels(&viewport, &orbit, &bla_table);

        let gpu_pixels = pollster::block_on(render_gpu_pixels(&viewport, &orbit, &bla_table));

        match gpu_pixels {
            Some(gpu) => {
                assert_eq!(
                    cpu_pixels.len(),
                    gpu.len(),
                    "CPU and GPU should produce same number of pixels"
                );
                compare_all_pixels(&cpu_pixels, &gpu);
            }
            None => {
                println!("\nGPU not available - cannot compare. Printing CPU results only:\n");
                for (i, cpu) in cpu_pixels.iter().enumerate() {
                    let col = TEST_COL_START + i as u32;
                    println!(
                        "Pixel {}: iterations={}, escaped={}, final_z_norm_sq={}",
                        col, cpu.iterations, cpu.escaped, cpu.final_z_norm_sq
                    );
                }
            }
        }
    }
```

**Step 3: Run test**

Run: `cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture`
Expected: PASS with full comparison output showing all 32 pixels

**Step 4: Commit**

```bash
git add fractalwonder-gpu/tests/cpu_gpu_comparison.rs
git commit -m "test: add CPU/GPU comparison output"
```

---

## Task 7: Final Verification

**Step 1: Run clippy**

Run: `cargo clippy -p fractalwonder-gpu --all-targets -- -D warnings`
Expected: No warnings

**Step 2: Run all GPU tests**

Run: `cargo test -p fractalwonder-gpu -- --nocapture`
Expected: All tests pass

**Step 3: Final commit if any fixes needed**

```bash
git add -A
git commit -m "test: fix any clippy warnings in cpu/gpu comparison"
```

---

## Notes

- Test gracefully skips if GPU unavailable (prints CPU-only results)
- Uses `pollster::block_on` for async (matches existing GPU test patterns)
- Renders entire image with `row_set_count=1` to simplify pixel extraction
- All production code paths used (no replicated logic)
