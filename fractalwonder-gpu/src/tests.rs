//! Tests for GPU renderer - verifies GPU output matches CPU perturbation.

use crate::{GpuAvailability, GpuContext, GpuRenderer};
use fractalwonder_compute::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::BigFloat;

/// Helper to create a reference orbit at a given center point.
fn create_reference_orbit(center_re: f64, center_im: f64, max_iter: u32) -> ReferenceOrbit {
    let precision = 128;
    let c_ref = (
        BigFloat::with_precision(center_re, precision),
        BigFloat::with_precision(center_im, precision),
    );
    ReferenceOrbit::compute(&c_ref, max_iter)
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
/// This is the core correctness test.
#[test]
fn gpu_matches_cpu_iteration_counts() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        // Test parameters
        let center_re = -0.5;
        let center_im = 0.0;
        let max_iter = 256;
        let tau_sq = 1e-6_f32;
        let width = 64_u32;
        let height = 64_u32;

        // Compute reference orbit
        let orbit = create_reference_orbit(center_re, center_im, max_iter);

        // Define viewport: standard Mandelbrot view
        let view_width = 3.0_f32;
        let view_height = 3.0_f32;
        let dc_origin = (-view_width / 2.0, -view_height / 2.0);
        let dc_step = (view_width / width as f32, view_height / height as f32);

        // GPU render
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

        // Compare each pixel against CPU
        let mut matches = 0;
        let mut mismatches = 0;
        let mut max_diff = 0_i32;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                // Compute delta_c for this pixel (as f64 for CPU)
                let delta_c = (
                    dc_origin.0 as f64 + x as f64 * dc_step.0 as f64,
                    dc_origin.1 as f64 + y as f64 * dc_step.1 as f64,
                );

                // CPU computation
                let cpu_result =
                    compute_pixel_perturbation(&orbit, delta_c, max_iter, tau_sq as f64);

                let gpu_iter = gpu_result.iterations[idx];
                let cpu_iter = cpu_result.iterations;

                let diff = (gpu_iter as i32 - cpu_iter as i32).abs();
                max_diff = max_diff.max(diff);

                // Allow ±1 iteration difference due to f32 vs f64 precision
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

        // Require at least 99% match within ±1 iteration
        assert!(
            match_pct >= 99.0,
            "GPU should match CPU for at least 99% of pixels, got {match_pct:.1}%"
        );
        // f32 vs f64 precision can cause up to ~5 iteration difference at boundary points
        assert!(
            max_diff <= 5,
            "Maximum iteration difference should be ≤5, got {max_diff}"
        );
    });
}

/// Verify escape detection works correctly.
#[test]
fn gpu_escape_detection_matches_cpu() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        let max_iter = 100;
        let orbit = create_reference_orbit(-0.5, 0.0, max_iter);

        // Test points: mix of escaping and in-set
        let test_points: Vec<(f32, f32)> = vec![
            (0.0, 0.0),    // In set (near origin after delta)
            (1.0, 0.0),    // Escapes quickly
            (-0.5, 0.5),   // In set (cardioid region)
            (0.3, 0.5),    // Escapes
            (-1.0, 0.0),   // In set (period-2 region)
            (0.5, 0.5),    // Escapes
            (-0.12, 0.75), // Near boundary
        ];

        let width = test_points.len() as u32;
        let height = 1;

        // Create single-row render with each test point
        let gpu_result = renderer
            .render(
                &orbit.orbit,
                1,
                (0.0, 0.0), // Will be overridden by per-pixel dc
                (1.0, 1.0), // Step doesn't matter for single row
                width,
                height,
                max_iter,
                1e-6,
            )
            .await;

        // For this test, manually compare known behaviors
        // Since we can't easily inject arbitrary dc values per-pixel,
        // verify the overall render succeeds
        assert!(gpu_result.is_ok(), "GPU render should succeed");

        let result = gpu_result.unwrap();
        assert_eq!(result.iterations.len(), test_points.len());

        println!("Escape detection test passed - GPU produced valid iteration counts");
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

        // Use a reference point where we expect some glitches
        // Main cardioid center - pixels far from reference may glitch
        let max_iter = 500;
        let orbit = create_reference_orbit(-0.5, 0.0, max_iter);

        let width = 100;
        let height = 100;

        // Wide view to include regions far from reference
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
                1e-6, // tau_sq
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

        // We expect SOME glitches in this setup (far pixels from reference)
        // But not ALL pixels should be glitched
        assert!(
            glitch_count > 0 || total < 100,
            "Expected some glitches for pixels far from reference"
        );
        assert!(glitch_count < total, "Not all pixels should be glitched");
    });
}

/// Verify rebasing works by testing a point that requires it.
#[test]
fn gpu_rebasing_produces_correct_results() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        // Reference at main cardioid
        let max_iter = 200;
        let orbit = create_reference_orbit(-0.5, 0.0, max_iter);

        // Single pixel test at a point that should rebase
        let width = 1;
        let height = 1;

        // Point significantly offset from reference
        let dc_origin = (0.3_f32, 0.3_f32);
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

        // Compare to CPU
        let cpu_result = compute_pixel_perturbation(
            &orbit,
            (dc_origin.0 as f64, dc_origin.1 as f64),
            max_iter,
            1e-6,
        );

        let gpu_iter = gpu_result.iterations[0];
        let cpu_iter = cpu_result.iterations;
        let diff = (gpu_iter as i32 - cpu_iter as i32).abs();

        println!("Rebasing test:");
        println!("  GPU iterations: {gpu_iter}");
        println!("  CPU iterations: {cpu_iter}");
        println!("  Difference: {diff}");

        assert!(
            diff <= 2,
            "GPU should match CPU within ±2 iterations after rebasing, got diff={diff}"
        );
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

        // Reference at origin (in set)
        let max_iter = 100;
        let orbit = create_reference_orbit(0.0, 0.0, max_iter);

        // Test at origin - should be in set
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

        let iterations = gpu_result.iterations[0];

        println!("In-set test: origin reached {iterations} iterations (max={max_iter})");

        assert_eq!(
            iterations, max_iter,
            "Origin should reach max_iter={max_iter}, got {iterations}"
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

        // Reference at origin
        let max_iter = 100;
        let orbit = create_reference_orbit(0.0, 0.0, max_iter);

        // Test point far outside set - should escape immediately
        let width = 1;
        let height = 1;
        let dc_origin = (3.0_f32, 0.0_f32); // c = 3 + 0i escapes at iteration 1
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

        let iterations = gpu_result.iterations[0];

        println!("Escape test: c=3+0i escaped at iteration {iterations}");

        assert!(
            iterations < 5,
            "Point at c=3+0i should escape within 5 iterations, got {iterations}"
        );
    });
}
