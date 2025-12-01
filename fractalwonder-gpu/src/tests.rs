//! Tests for GPU renderer - verifies GPU output matches CPU perturbation.

use crate::pass::Adam7Pass;
use crate::{GpuAvailability, GpuContext, GpuPerturbationRenderer};
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

        let mut renderer = GpuPerturbationRenderer::new(ctx);

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
                orbit.escaped_at.is_some(),
                Adam7Pass::all_pixels(),
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
        // Note: f32 vs f64 precision differences can cause significant iteration
        // differences at boundary regions where rebase decisions diverge.
        // We allow up to 100 iterations difference for rare edge cases.
        assert!(
            max_diff <= 100,
            "Maximum iteration difference should be ≤100, got {max_diff}"
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

        let mut renderer = GpuPerturbationRenderer::new(ctx);

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
                orbit.escaped_at.is_some(),
                Adam7Pass::all_pixels(),
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

        let mut renderer = GpuPerturbationRenderer::new(ctx);

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
                orbit.escaped_at.is_some(),
                Adam7Pass::all_pixels(),
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
        assert!(!gpu_data.escaped, "Origin should not escape");
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

        let mut renderer = GpuPerturbationRenderer::new(ctx);

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
                orbit.escaped_at.is_some(),
                Adam7Pass::all_pixels(),
            )
            .await
            .expect("GPU render should succeed");

        let gpu_data = as_mandelbrot(&gpu_result.data[0]);

        println!(
            "Escape test: c=3+0i escaped at iteration {}",
            gpu_data.iterations
        );

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

// TODO: These tests use the deleted DirectFloatExp renderer - need to be updated for HDR
// /// Helper to convert FloatExp to tuple format for renderer.
// fn floatexp_to_tuple(re: FloatExp, im: FloatExp) -> (f32, i32, f32, i32) {
//     (
//         re.mantissa() as f32,
//         re.exp() as i32,
//         im.mantissa() as f32,
//         im.exp() as i32,
//     )
// }
//
// /// Test that DirectFloatExp renderer initializes without panic.
// #[test]
// fn direct_floatexp_init_does_not_panic() {
//     pollster::block_on(async {
//         let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
//             println!("Skipping test: no GPU available");
//             return;
//         };
//         let _renderer = GpuDirectFloatExpRenderer::new(ctx);
//         println!("GpuDirectFloatExpRenderer initialized successfully");
//     });
// }
//
// /// Test that DirectFloatExp produces correct results for known points.
// #[test]
// fn direct_floatexp_known_points() {
//     pollster::block_on(async {
//         let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
//             println!("Skipping test: no GPU available");
//             return;
//         };
//
//         let mut renderer = GpuDirectFloatExpRenderer::new(ctx);
//
//         let width = 3u32;
//         let height = 1u32;
//         let max_iter = 100;
//
//         // Test 3 points: origin (in set), c=3 (escapes fast), c=-2 (boundary)
//         let c_origin = floatexp_to_tuple(FloatExp::from_f64(0.0), FloatExp::from_f64(0.0));
//         let c_step = floatexp_to_tuple(
//             FloatExp::from_f64(1.5), // 0, 1.5, 3.0
//             FloatExp::from_f64(0.0),
//         );
//
//         let result = renderer
//             .render(
//                 c_origin,
//                 c_step,
//                 width,
//                 height,
//                 max_iter,
//                 Adam7Pass::all_pixels(),
//             )
//             .await
//             .expect("Render should succeed");
//
//         let iter_0 = as_mandelbrot(&result.data[0]).iterations;
//         let iter_1 = as_mandelbrot(&result.data[1]).iterations;
//         let iter_2 = as_mandelbrot(&result.data[2]).iterations;
//
//         println!("c=0: {} iterations", iter_0);
//         println!("c=1.5: {} iterations", iter_1);
//         println!("c=3: {} iterations", iter_2);
//
//         // Origin should reach max_iter (in set)
//         assert_eq!(iter_0, max_iter, "c=0 should be in set");
//
//         // c=3 should escape very quickly (1-2 iterations)
//         assert!(iter_2 < 5, "c=3 should escape within 5 iterations");
//     });
// }
//
// /// Test DirectFloatExp at moderate zoom (10^4) - the problematic range.
// #[test]
// fn direct_floatexp_moderate_zoom() {
//     pollster::block_on(async {
//         let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
//             println!("Skipping test: no GPU available");
//             return;
//         };
//
//         let mut renderer = GpuDirectFloatExpRenderer::new(ctx);
//
//         let width = 64u32;
//         let height = 64u32;
//         let max_iter = 500;
//
//         // Zoom 10^4 near the main cardioid boundary
//         let center_re = -0.1;
//         let center_im = 0.65;
//         let view_size = 5e-4;
//
//         let c_origin = floatexp_to_tuple(
//             FloatExp::from_f64(center_re - view_size / 2.0),
//             FloatExp::from_f64(center_im - view_size / 2.0),
//         );
//         let c_step = floatexp_to_tuple(
//             FloatExp::from_f64(view_size / width as f64),
//             FloatExp::from_f64(view_size / height as f64),
//         );
//
//         let result = renderer
//             .render(
//                 c_origin,
//                 c_step,
//                 width,
//                 height,
//                 max_iter,
//                 Adam7Pass::all_pixels(),
//             )
//             .await
//             .expect("Render should succeed");
//
//         // Count escaped vs in-set pixels
//         let escaped = result
//             .data
//             .iter()
//             .filter(|d| as_mandelbrot(d).escaped)
//             .count();
//         let in_set = result
//             .data
//             .iter()
//             .filter(|d| !as_mandelbrot(d).escaped)
//             .count();
//
//         println!("Moderate zoom (10^4) at ({}, {}):", center_re, center_im);
//         println!("  Escaped: {}", escaped);
//         println!("  In set: {}", in_set);
//         println!("  Compute time: {:.2}ms", result.compute_time_ms);
//
//         // Should have a mix of escaped and in-set pixels at boundary
//         assert!(escaped > 0, "Should have some escaped pixels");
//         assert!(in_set > 0, "Should have some in-set pixels");
//     });
// }
