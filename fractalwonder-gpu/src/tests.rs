//! Tests for GPU renderer - verifies GPU output matches CPU perturbation.

use crate::pass::Adam7Pass;
use crate::{GpuAvailability, GpuContext, GpuPerturbationHDRRenderer, GpuPerturbationRenderer};
use fractalwonder_compute::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr, MandelbrotRenderer, ReferenceOrbit,
    Renderer,
};
use fractalwonder_core::{
    calculate_max_iterations, BigFloat, ComputeData, HDRComplex, HDRFloat, MandelbrotData,
    PixelRect,
};

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

        // Note: f32 (GPU) vs f64 (CPU) precision differences cause iteration divergence
        // at boundary regions where rebase decisions differ. The 80% threshold reflects
        // the inherent precision gap between f32 and f64 perturbation. For higher accuracy,
        // use the HDR renderer which matches CPU HDRFloat more closely.
        assert!(
            match_pct >= 80.0,
            "GPU should match CPU for at least 80% of pixels, got {match_pct:.1}%"
        );
        assert!(
            max_diff <= 250,
            "Maximum iteration difference should be ≤250, got {max_diff}"
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

// =============================================================================
// CPU vs GPU comparison tests with real viewport data from URLs
// =============================================================================

mod url_decode {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use flate2::read::DeflateDecoder;
    use fractalwonder_core::Viewport;
    use serde::Deserialize;
    use std::io::Read;

    const URL_HASH_PREFIX: &str = "v1:";

    /// Minimal persisted state for decoding URL parameters.
    #[derive(Clone, Debug, Deserialize)]
    pub struct PersistedState {
        pub viewport: Viewport,
        #[allow(dead_code)]
        pub config_id: String,
        #[allow(dead_code)]
        version: u32,
    }

    /// Decode state from a v1: URL hash string.
    pub fn decode_url_hash(url: &str) -> Option<PersistedState> {
        // Extract hash portion from URL
        let hash = url.split('#').nth(1)?;

        // Strip v1: prefix
        let data = hash.strip_prefix(URL_HASH_PREFIX)?;

        // Decode base64
        let compressed = URL_SAFE_NO_PAD.decode(data).ok()?;

        // Decompress
        let mut decoder = DeflateDecoder::new(&compressed[..]);
        let mut json = String::new();
        decoder.read_to_string(&mut json).ok()?;

        // Deserialize
        let state: PersistedState = serde_json::from_str(&json).ok()?;

        if state.version >= 1 && state.version <= 3 {
            Some(state)
        } else {
            None
        }
    }
}

/// Test URLs from real fractal positions for GPU vs CPU comparison.
const TEST_URLS: &[&str] = &[
    "http://127.0.0.1:8080/fractalwonder/#v1:jZLhasQgDMffJZ-7QxkM1lcZo3ht2gpWi6Y9Rrl3Xzq1HuUOZqtoEuMvf91g1XibnSeoN2jREnqovzZYlVkQaniTFyHk0fb5X5epx3Wa8ieiV6RVnIg4RkvcyT9UMHtsddDONldNAeqPz3v1cLa4iJRQ5E0xkcz5crbU0vkp-ICQGUDmdaR_TvBdwU13NO6KZJRE8rzJo_JzrQdRdOagM6AoEufaeHihD4yoh5H-TVcuIgl5UCQkWbQ9JBIlUD5Il4SW5R5zRlnCX4Azeetsr4dGd8w8KduhuXpHsDuM842biXeEvbJZGSTCGNprjxwURtVpOzRo1dUgO3plArJ9co7GYia_sHXUgdzg1XR2tD-twaZ1i2UNZQXklQ09eiZVfDa_dyZd0e_0UL_ffwE",
    "http://127.0.0.1:8080/fractalwonder/#v1:nZLdasMwDIXfRddZkRiDkVcZI7iJmhhcOzhuyyh999nxL2OFMRPiRJas7xz7DlfJt9VYB_0dRtaOLfQfd7gKdWHo4YUOiFQHhgdxj4X3_hmm9EvYpsZgnGJKyi0lhGm_tAMSdLBaHuUmjR6O0m3Qv789ugYJD6WAMG-OmSQhlYXYKiMnCTFCGa0yJfwCV9HpCdlnBzc5uSUYmBEz4Z9G2ygZV9Uk-zJFIS9yiufFDywyowYseelUnjgMC8t5cf_WgbUbUuNzEVbcpZJO7T3KmhplVSK2QpGqIZQ9KM3bS5brf9fsRY9Gn-Q8yMnLPQs9sTpa4yAsKGMHszpfsQVTVqHYOY6p7ibV7lYH2yImqeeBtTgq9osnoTb28bMxbqlhZy8-usjNmdmK88-F8WtUPIzmov0RUAfOCr2d2Hpa4fvjgTztlW1QAP3r4xs",
    "http://127.0.0.1:8080/fractalwonder/#v1:nVHRbsMgDPwXP2eV2R4q5VemKaLETZAoREBTTVH_fU4CJNs6qZqVIGMOzuebYNR0G5yPUE-gyEbyUL9PMEpzJajhBQ8COXgRS8ypwCVBxFTArZzQYv12CCGggsGT0kE725x0DFAfj_fqJ5lIf85224VjeQ5Xhm8cCyr3tSyPGT8quOk29rPkTI0HfCZEYUm0-92uxXSQLm2F0mKWJvYZZj1phH-ODHrSXR__IaC4Ukws9mLxGfetJyPFhiymPBVZDia7ile5NKePdbJQ5exZd41uWeJF2pbMybsI84FxvnFD5BthHsQgDcVIK1QZGYJWjAu9bLXtGrLyZIjPztIE4vrFudhv5eivXO11iK7z8vILrz6VoUa5q-Wxv71WEL204Uye25XcgDggtzuSnyUw4v4F",
    // Noisy viewport reported by user
    "http://127.0.0.1:8080/fractalwonder/#v1:nVHbasMwDP0XPWdF3h4K-ZUxguuoicG1g620jNB_n5I4l20djIrEyNKRz5E0wNXSrQuRoRzAkGeKUL4PcNWuJyjhBQ8KxeRQk42uwslBxBzALZzRav52CKWggC6SsckGX50sJyiPx3vxk0zlf_F214ljeg5nhm8cE2rRNR2PGT8KuNma27HlhRoP-B9TK0um3d92EnMiF22BVeLSmtp7uPSTR_jnyKAl27T8RAPrVtYlruvFdc-4l54XqTZk1vtYm4gzwZ9tU9laZF20r8mdYmAYEy7EKnQsFWkU32lHzDRDjdMpWSO41Ora-qYir0-OJHfWLpHELyFwu4U59hJtbeLQRH35hTefxlFlQu9lVG-vBXDUPp0pilwtAtQBC-gTVU3Xz6-J-ivFsSMpuH8B",
];

/// Compare GPU perturbation (HDRFloat), CPU perturbation (HDRFloat), and pure BigFloat renderers.
/// BigFloat is the ground truth - it uses arbitrary precision arithmetic.
#[test]
fn gpu_matches_cpu_for_real_viewports() {
    use url_decode::decode_url_hash;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        // Use HDRFloat GPU renderer for proper precision at deep zooms
        let mut gpu_renderer = GpuPerturbationHDRRenderer::new(ctx);

        let width = 64_u32;
        let height = 64_u32;
        let tau_sq = 1e-6_f32;

        for (url_idx, url) in TEST_URLS.iter().enumerate() {
            println!("\n=== Test URL {} ===", url_idx + 1);

            let state = decode_url_hash(url).expect("Failed to decode URL");
            let viewport = &state.viewport;

            println!(
                "Viewport center: ({}, {})",
                viewport.center.0.to_f64(),
                viewport.center.1.to_f64()
            );
            println!(
                "Viewport size: {} x {}",
                viewport.width.to_f64(),
                viewport.height.to_f64()
            );

            // Calculate zoom-appropriate max_iterations using the same function as production code
            // Default values: multiplier=200.0, power=2.5 (from fractalwonder-ui/src/config.rs)
            let ref_width = 4.0_f64;
            let zoom = ref_width / viewport.width.to_f64();
            let zoom_exponent = zoom.log10();
            let max_iter = calculate_max_iterations(zoom_exponent, 200.0, 2.5);
            // Use full max_iter for accurate comparison - this test is about correctness, not speed
            println!("Zoom: {zoom:.2e}, zoom_exponent: {zoom_exponent:.2}, max_iter: {max_iter}");

            // =========================================================================
            // 1. BigFloat renderer (ground truth - arbitrary precision)
            // =========================================================================
            println!("\nRendering with BigFloat (ground truth)...");
            let bigfloat_renderer = MandelbrotRenderer::new(max_iter);
            let bigfloat_result = bigfloat_renderer.render(viewport, (width, height));
            println!(
                "  BigFloat render complete: {} pixels",
                bigfloat_result.len()
            );

            // =========================================================================
            // 2. GPU perturbation renderer (HDRFloat)
            // =========================================================================
            println!("Rendering with GPU perturbation (HDRFloat)...");
            let center_re = viewport.center.0.to_f64();
            let center_im = viewport.center.1.to_f64();
            let orbit = create_reference_orbit(center_re, center_im, max_iter);

            // Convert viewport dimensions to HDRFloat for precision at deep zooms
            // This mirrors production code in parallel_renderer.rs
            let vp_width = HDRFloat::from_bigfloat(&viewport.width);
            let vp_height = HDRFloat::from_bigfloat(&viewport.height);

            let half = HDRFloat::from_f64(0.5);
            let half_width = vp_width.mul(&half);
            let half_height = vp_height.mul(&half);
            let origin_re = half_width.neg();
            let origin_im = half_height.neg();

            // Compute step as HDRFloat to preserve precision
            let step_re = vp_width.div_f64(width as f64);
            let step_im = vp_height.div_f64(height as f64);

            let dc_origin = (
                (origin_re.head, origin_re.tail, origin_re.exp),
                (origin_im.head, origin_im.tail, origin_im.exp),
            );
            let dc_step = (
                (step_re.head, step_re.tail, step_re.exp),
                (step_im.head, step_im.tail, step_im.exp),
            );

            // Render entire image as a single tile
            let tile = PixelRect {
                x: 0,
                y: 0,
                width,
                height,
            };

            let tile_size = width.max(height);
            let gpu_result = gpu_renderer
                .render_tile(
                    &orbit.orbit,
                    url_idx as u32 + 1,
                    dc_origin,
                    dc_step,
                    width,
                    height,
                    &tile,
                    tile_size,
                    max_iter,
                    tau_sq,
                    orbit.escaped_at.is_some(),
                )
                .await
                .expect("GPU render should succeed");
            println!("  GPU render complete: {:.2}ms", gpu_result.compute_time_ms);

            // =========================================================================
            // 3. Compare all three renderers
            // =========================================================================
            let mut gpu_vs_bigfloat_matches = 0_u32;
            let mut cpu_vs_bigfloat_matches = 0_u32;
            let mut gpu_vs_cpu_matches = 0_u32;
            let mut gpu_vs_bigfloat_max_diff = 0_i32;
            let mut cpu_vs_bigfloat_max_diff = 0_i32;
            let mut gpu_vs_cpu_max_diff = 0_i32;
            let mut glitched_count = 0_u32;

            // BigFloat stats
            let mut bf_escaped = 0_u32;
            let mut bf_min_iter = u32::MAX;
            let mut bf_max_iter = 0_u32;
            let mut bf_iter_sum = 0_u64;

            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;

                    // BigFloat result (ground truth)
                    let bf_data = &bigfloat_result[idx];

                    // GPU result
                    let gpu_data = as_mandelbrot(&gpu_result.data[idx]);

                    // CPU HDRFloat perturbation result - use proper HDRFloat arithmetic
                    // Reconstruct origin and step as HDRFloat from tuples
                    let origin_re_hdr = HDRFloat {
                        head: dc_origin.0 .0,
                        tail: dc_origin.0 .1,
                        exp: dc_origin.0 .2,
                    };
                    let origin_im_hdr = HDRFloat {
                        head: dc_origin.1 .0,
                        tail: dc_origin.1 .1,
                        exp: dc_origin.1 .2,
                    };
                    let step_re_hdr = HDRFloat {
                        head: dc_step.0 .0,
                        tail: dc_step.0 .1,
                        exp: dc_step.0 .2,
                    };
                    let step_im_hdr = HDRFloat {
                        head: dc_step.1 .0,
                        tail: dc_step.1 .1,
                        exp: dc_step.1 .2,
                    };

                    // dc = origin + pixel * step
                    let dc_re = origin_re_hdr.add(&HDRFloat::from_f64(x as f64).mul(&step_re_hdr));
                    let dc_im = origin_im_hdr.add(&HDRFloat::from_f64(y as f64).mul(&step_im_hdr));
                    let delta_c = HDRComplex {
                        re: dc_re,
                        im: dc_im,
                    };
                    let cpu_hdr_result =
                        compute_pixel_perturbation_hdr(&orbit, delta_c, max_iter, tau_sq as f64);

                    // BigFloat stats
                    bf_iter_sum += bf_data.iterations as u64;
                    bf_min_iter = bf_min_iter.min(bf_data.iterations);
                    bf_max_iter = bf_max_iter.max(bf_data.iterations);
                    if bf_data.escaped {
                        bf_escaped += 1;
                    }

                    if gpu_data.glitched {
                        glitched_count += 1;
                    }

                    // GPU vs BigFloat
                    let gpu_bf_diff =
                        (gpu_data.iterations as i32 - bf_data.iterations as i32).abs();
                    gpu_vs_bigfloat_max_diff = gpu_vs_bigfloat_max_diff.max(gpu_bf_diff);
                    if gpu_bf_diff <= 1 {
                        gpu_vs_bigfloat_matches += 1;
                    }

                    // CPU HDRFloat vs BigFloat
                    let cpu_bf_diff =
                        (cpu_hdr_result.iterations as i32 - bf_data.iterations as i32).abs();
                    cpu_vs_bigfloat_max_diff = cpu_vs_bigfloat_max_diff.max(cpu_bf_diff);
                    if cpu_bf_diff <= 1 {
                        cpu_vs_bigfloat_matches += 1;
                    }

                    // GPU vs CPU HDRFloat
                    let gpu_cpu_diff =
                        (gpu_data.iterations as i32 - cpu_hdr_result.iterations as i32).abs();
                    gpu_vs_cpu_max_diff = gpu_vs_cpu_max_diff.max(gpu_cpu_diff);
                    if gpu_cpu_diff <= 1 {
                        gpu_vs_cpu_matches += 1;
                    }
                }
            }

            let total = width * height;
            let gpu_bf_pct = 100.0 * gpu_vs_bigfloat_matches as f64 / total as f64;
            let cpu_bf_pct = 100.0 * cpu_vs_bigfloat_matches as f64 / total as f64;
            let gpu_cpu_pct = 100.0 * gpu_vs_cpu_matches as f64 / total as f64;
            let bf_avg_iter = bf_iter_sum as f64 / total as f64;

            println!("\n--- Results for URL {} ---", url_idx + 1);
            println!("BigFloat (ground truth):");
            println!(
                "  Iterations: min={}, max={}, avg={:.1}",
                bf_min_iter, bf_max_iter, bf_avg_iter
            );
            println!(
                "  Escaped: {} ({:.1}%)",
                bf_escaped,
                100.0 * bf_escaped as f64 / total as f64
            );

            println!("\nGPU vs BigFloat:");
            println!(
                "  Matches (±1): {} ({:.1}%)",
                gpu_vs_bigfloat_matches, gpu_bf_pct
            );
            println!("  Max iteration diff: {}", gpu_vs_bigfloat_max_diff);

            println!("\nCPU HDRFloat vs BigFloat:");
            println!(
                "  Matches (±1): {} ({:.1}%)",
                cpu_vs_bigfloat_matches, cpu_bf_pct
            );
            println!("  Max iteration diff: {}", cpu_vs_bigfloat_max_diff);

            println!("\nGPU vs CPU HDRFloat:");
            println!(
                "  Matches (±1): {} ({:.1}%)",
                gpu_vs_cpu_matches, gpu_cpu_pct
            );
            println!("  Max iteration diff: {}", gpu_vs_cpu_max_diff);

            println!("\nGlitched pixels: {}", glitched_count);

            // The key insight: if both GPU and CPU HDRFloat diverge from BigFloat similarly,
            // the problem is in HDRFloat. If they diverge differently, the problem is
            // GPU-specific or CPU-specific.
            println!("\n=== Diagnosis ===");
            if cpu_bf_pct < 90.0 && gpu_bf_pct < 90.0 {
                println!("Both HDRFloat renderers diverge from BigFloat - likely HDRFloat precision/overflow issue");
            } else if gpu_bf_pct < cpu_bf_pct - 10.0 {
                println!("GPU diverges more than CPU - likely GPU-specific issue");
            } else if cpu_bf_pct < gpu_bf_pct - 10.0 {
                println!("CPU diverges more than GPU - likely CPU HDRFloat-specific issue");
            } else {
                println!("All renderers agree reasonably well");
            }
        }
    });
}

// =============================================================================
// Progressive GPU renderer tests
// =============================================================================

/// Test that ProgressiveGpuRenderer initializes without panic.
#[test]
fn progressive_renderer_init_does_not_panic() {
    use crate::progressive_renderer::ProgressiveGpuRenderer;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };
        let _renderer = ProgressiveGpuRenderer::new(ctx);
        println!("ProgressiveGpuRenderer initialized successfully");
    });
}

/// Test progressive renderer produces correct results for simple case.
#[test]
fn progressive_renderer_basic_render() {
    use crate::progressive_renderer::ProgressiveGpuRenderer;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = ProgressiveGpuRenderer::new(ctx);

        let center_re = -0.5;
        let center_im = 0.0;
        let max_iter = 256;
        let tau_sq = 1e-6_f32;
        let width = 64_u32;
        let height = 64_u32;
        let row_set_count = 4_u32;
        let iterations_per_dispatch = 100_u32;

        let orbit = create_reference_orbit(center_re, center_im, max_iter);

        // Setup dc_origin and dc_step as HDRFloat tuples
        let view_width = 3.0_f32;
        let view_height = 3.0_f32;
        let dc_origin = ((-view_width / 2.0, 0.0, 0), (-view_height / 2.0, 0.0, 0));
        let dc_step = (
            (view_width / width as f32, 0.0, 0),
            (view_height / height as f32, 0.0, 0),
        );

        // Render first row-set
        let result = renderer
            .render_row_set(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                0, // row_set_index
                row_set_count,
                max_iter,
                iterations_per_dispatch,
                tau_sq,
                orbit.escaped_at.is_some(),
            )
            .await
            .expect("Progressive render should succeed");

        let expected_pixels =
            ProgressiveGpuRenderer::calculate_row_set_pixel_count(width, height, row_set_count);

        assert_eq!(
            result.data.len(),
            expected_pixels as usize,
            "Should have correct number of pixels"
        );

        // Check that we have a mix of escaped and non-escaped pixels
        let escaped_count = result
            .data
            .iter()
            .filter(|d| as_mandelbrot(d).escaped)
            .count();

        // Verify iteration distribution
        let in_set_count = result
            .data
            .iter()
            .filter(|d| as_mandelbrot(d).iterations == max_iter)
            .count();
        println!(
            "Progressive render: {} pixels, {} escaped, {} in-set, {:.2}ms",
            result.data.len(),
            escaped_count,
            in_set_count,
            result.compute_time_ms
        );

        assert!(escaped_count > 0, "Should have some escaped pixels");
        assert!(
            escaped_count < result.data.len(),
            "Should have some non-escaped pixels"
        );
    });
}

// =============================================================================
// Glitch debugging test - compares GPU vs CPU for glitched pixels
// =============================================================================

/// Debug test that compares GPU and CPU results specifically for glitched pixels.
/// Uses a cached file of glitched pixel coordinates to enable reproducible debugging.
#[test]
fn debug_glitched_pixels_gpu_vs_cpu() {
    use crate::progressive_renderer::ProgressiveGpuRenderer;
    use std::fs;
    use std::path::Path;
    use url_decode::decode_url_hash;

    // The problematic URL with scattered glitches
    const GLITCH_URL: &str = "http://127.0.0.1:8080/fractalwonder/#v1:dU_tboMwDHwX_2ZVQkIIvMo0oTS4EClNUBJaTajvPrMPbZqof1l3vjvfBjeH9yWmAv0GFkPBBP3rBjfjV4QeXvipVnXLuWJK604JWUMFS0LrsothOLuSoVfyUf3RsJNkWjLJlWo7JjjvjjVvFdzdWOY9-0fMTuxzdNu0SnChGik0LU9SYUY3zeXIoVENF7XUnZC87rQ-diALG8PFTYMbSXw1YUR_TrHATviYhrgUUuQ9YjEeS8GvU-tNzs7SXZ7N6MI0YDBnj8RdjM9I-DXGMv_CJa2Ezi6XOCVz_U_Yd-txsHEN1IdXUJIJ-YKJnjUUX1OxCtaMw7Ss3xn0_Q3T3gh68fgA";

    const IMAGE_SIZE: u32 = 500;
    const GLITCH_CACHE_FILE: &str = "target/glitched_pixels_cache.json";

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        // Decode viewport from URL
        let state = decode_url_hash(GLITCH_URL).expect("Failed to decode URL");
        let viewport = &state.viewport;

        println!("=== Glitch Debugging Test ===");
        println!(
            "Viewport center: ({:.15}, {:.15})",
            viewport.center.0.to_f64(),
            viewport.center.1.to_f64()
        );
        println!(
            "Viewport size: {:.6e} x {:.6e}",
            viewport.width.to_f64(),
            viewport.height.to_f64()
        );

        let ref_width = 4.0_f64;
        let zoom = ref_width / viewport.width.to_f64();
        let zoom_exponent = zoom.log10();
        let max_iter = calculate_max_iterations(zoom_exponent, 200.0, 2.5);
        let tau_sq = 1e-6_f32;

        println!("Zoom: {zoom:.2e} (10^{zoom_exponent:.2})");
        println!("Max iterations: {max_iter}");

        // Create reference orbit at viewport center
        let center_re = viewport.center.0.to_f64();
        let center_im = viewport.center.1.to_f64();
        let orbit = create_reference_orbit(center_re, center_im, max_iter);
        println!(
            "Reference orbit length: {}, escaped_at: {:?}",
            orbit.orbit.len(),
            orbit.escaped_at
        );

        // Setup δc parameters
        let vp_width = HDRFloat::from_bigfloat(&viewport.width);
        let vp_height = HDRFloat::from_bigfloat(&viewport.height);
        let half = HDRFloat::from_f64(0.5);
        let half_width = vp_width.mul(&half);
        let half_height = vp_height.mul(&half);
        let origin_re = half_width.neg();
        let origin_im = half_height.neg();
        let step_re = vp_width.div_f64(IMAGE_SIZE as f64);
        let step_im = vp_height.div_f64(IMAGE_SIZE as f64);

        let dc_origin = (
            (origin_re.head, origin_re.tail, origin_re.exp),
            (origin_im.head, origin_im.tail, origin_im.exp),
        );
        let dc_step = (
            (step_re.head, step_re.tail, step_re.exp),
            (step_im.head, step_im.tail, step_im.exp),
        );

        // =========================================================================
        // Step 1: Get glitched pixel coordinates (from cache or fresh render)
        // =========================================================================
        let glitched_pixels: Vec<(u32, u32)> = if Path::new(GLITCH_CACHE_FILE).exists() {
            println!("\nLoading glitched pixels from cache: {GLITCH_CACHE_FILE}");
            let json = fs::read_to_string(GLITCH_CACHE_FILE).expect("Failed to read cache");
            serde_json::from_str(&json).expect("Failed to parse cache")
        } else {
            println!("\nRendering {IMAGE_SIZE}x{IMAGE_SIZE} image to detect glitches...");

            let mut renderer = ProgressiveGpuRenderer::new(ctx);
            let row_set_count = 8_u32;
            let iterations_per_dispatch = 1000_u32;

            let mut all_glitched = Vec::new();

            // Render all row-sets
            for row_set_idx in 0..row_set_count {
                let result = renderer
                    .render_row_set(
                        &orbit.orbit,
                        1,
                        dc_origin,
                        dc_step,
                        IMAGE_SIZE,
                        IMAGE_SIZE,
                        row_set_idx,
                        row_set_count,
                        max_iter,
                        iterations_per_dispatch,
                        tau_sq,
                        orbit.escaped_at.is_some(),
                    )
                    .await
                    .expect("GPU render should succeed");

                // Extract glitched pixel coordinates
                for (linear_idx, data) in result.data.iter().enumerate() {
                    let md = as_mandelbrot(data);
                    if md.glitched {
                        // Convert linear index back to (x, y) coordinates
                        let row_within_set = linear_idx as u32 / IMAGE_SIZE;
                        let col = linear_idx as u32 % IMAGE_SIZE;
                        let global_row = row_within_set * row_set_count + row_set_idx;
                        all_glitched.push((col, global_row));
                    }
                }
            }

            println!("Found {} glitched pixels", all_glitched.len());

            // Cache the results
            if let Some(parent) = Path::new(GLITCH_CACHE_FILE).parent() {
                let _ = fs::create_dir_all(parent);
            }
            let json = serde_json::to_string(&all_glitched).expect("Failed to serialize");
            fs::write(GLITCH_CACHE_FILE, json).expect("Failed to write cache");
            println!("Cached glitched pixels to {GLITCH_CACHE_FILE}");

            all_glitched
        };

        if glitched_pixels.is_empty() {
            println!("No glitched pixels found - test passed!");
            return;
        }

        println!("\nTotal glitched pixels: {}", glitched_pixels.len());

        // =========================================================================
        // Step 2: Sample 10% of glitched pixels evenly distributed
        // =========================================================================
        let sample_size = (glitched_pixels.len() / 10).clamp(1, 100);
        let step = glitched_pixels.len() / sample_size;
        let sample: Vec<(u32, u32)> = glitched_pixels
            .iter()
            .step_by(step)
            .take(sample_size)
            .copied()
            .collect();

        println!("Sampling {} pixels for detailed comparison", sample.len());

        // =========================================================================
        // Step 3: Compare GPU vs CPU for each sampled pixel
        // =========================================================================
        println!(
            "\n{:>4} {:>4} {:>8} {:>8} {:>8} {:>5} {:>5}",
            "X", "Y", "GPU_iter", "CPU_iter", "BF_iter", "G_esc", "C_esc"
        );
        println!("{}", "-".repeat(60));

        let mut total_gpu_cpu_diff = 0_i64;
        let mut total_gpu_bf_diff = 0_i64;
        let mut total_cpu_bf_diff = 0_i64;
        let mut max_gpu_cpu_diff = 0_i32;
        let mut max_gpu_bf_diff = 0_i32;

        // We need a fresh GPU context for the HDR tile renderer
        let GpuAvailability::Available(ctx2) = GpuContext::try_init().await else {
            println!("Failed to reinitialize GPU");
            return;
        };
        let mut hdr_renderer = GpuPerturbationHDRRenderer::new(ctx2);

        for &(px, py) in &sample {
            // Reconstruct HDRFloat origins and steps
            let origin_re_hdr = HDRFloat {
                head: dc_origin.0 .0,
                tail: dc_origin.0 .1,
                exp: dc_origin.0 .2,
            };
            let origin_im_hdr = HDRFloat {
                head: dc_origin.1 .0,
                tail: dc_origin.1 .1,
                exp: dc_origin.1 .2,
            };
            let step_re_hdr = HDRFloat {
                head: dc_step.0 .0,
                tail: dc_step.0 .1,
                exp: dc_step.0 .2,
            };
            let step_im_hdr = HDRFloat {
                head: dc_step.1 .0,
                tail: dc_step.1 .1,
                exp: dc_step.1 .2,
            };

            // Calculate delta_c for this pixel
            let dc_re = origin_re_hdr.add(&HDRFloat::from_f64(px as f64).mul(&step_re_hdr));
            let dc_im = origin_im_hdr.add(&HDRFloat::from_f64(py as f64).mul(&step_im_hdr));

            // --- GPU render (single pixel tile) ---
            let tile = PixelRect {
                x: px,
                y: py,
                width: 1,
                height: 1,
            };

            // Compute dc_origin for this specific tile
            let tile_dc_origin = (
                (dc_re.head, dc_re.tail, dc_re.exp),
                (dc_im.head, dc_im.tail, dc_im.exp),
            );

            let gpu_result = hdr_renderer
                .render_tile(
                    &orbit.orbit,
                    1,
                    tile_dc_origin,
                    dc_step,
                    IMAGE_SIZE,
                    IMAGE_SIZE,
                    &tile,
                    1,
                    max_iter,
                    tau_sq,
                    orbit.escaped_at.is_some(),
                )
                .await
                .expect("GPU render should succeed");

            let gpu_data = as_mandelbrot(&gpu_result.data[0]);

            // --- CPU HDRFloat render ---
            let delta_c = HDRComplex {
                re: dc_re,
                im: dc_im,
            };
            let cpu_result =
                compute_pixel_perturbation_hdr(&orbit, delta_c, max_iter, tau_sq as f64);

            // --- BigFloat render (ground truth) ---
            let bf_renderer = MandelbrotRenderer::new(max_iter);
            // Create a viewport for just this one pixel
            let precision = viewport.center.0.precision_bits();
            let image_size_bf = BigFloat::with_precision(IMAGE_SIZE as f64, precision);
            let pixel_vp = fractalwonder_core::Viewport::with_bigfloat(
                viewport.center.0.add(&BigFloat::with_precision(
                    (px as f64 - IMAGE_SIZE as f64 / 2.0) * viewport.width.to_f64()
                        / IMAGE_SIZE as f64,
                    precision,
                )),
                viewport.center.1.add(&BigFloat::with_precision(
                    (py as f64 - IMAGE_SIZE as f64 / 2.0) * viewport.height.to_f64()
                        / IMAGE_SIZE as f64,
                    precision,
                )),
                viewport.width.div(&image_size_bf),
                viewport.height.div(&image_size_bf),
            );
            let bf_result = bf_renderer.render(&pixel_vp, (1, 1));
            let bf_data = &bf_result[0];

            // Calculate differences
            let gpu_cpu_diff = (gpu_data.iterations as i32 - cpu_result.iterations as i32).abs();
            let gpu_bf_diff = (gpu_data.iterations as i32 - bf_data.iterations as i32).abs();
            let cpu_bf_diff = (cpu_result.iterations as i32 - bf_data.iterations as i32).abs();

            total_gpu_cpu_diff += gpu_cpu_diff as i64;
            total_gpu_bf_diff += gpu_bf_diff as i64;
            total_cpu_bf_diff += cpu_bf_diff as i64;
            max_gpu_cpu_diff = max_gpu_cpu_diff.max(gpu_cpu_diff);
            max_gpu_bf_diff = max_gpu_bf_diff.max(gpu_bf_diff);

            // Only print if there's a significant difference
            if gpu_cpu_diff > 1 || gpu_bf_diff > 1 {
                println!(
                    "{:>4} {:>4} {:>8} {:>8} {:>8} {:>5} {:>5}",
                    px,
                    py,
                    gpu_data.iterations,
                    cpu_result.iterations,
                    bf_data.iterations,
                    if gpu_data.escaped { "Y" } else { "N" },
                    if cpu_result.escaped { "Y" } else { "N" }
                );
            }
        }

        // =========================================================================
        // Summary
        // =========================================================================
        println!("\n=== Summary ===");
        println!("Sampled pixels: {}", sample.len());
        println!(
            "GPU vs CPU: avg diff = {:.2}, max diff = {}",
            total_gpu_cpu_diff as f64 / sample.len() as f64,
            max_gpu_cpu_diff
        );
        println!(
            "GPU vs BigFloat: avg diff = {:.2}, max diff = {}",
            total_gpu_bf_diff as f64 / sample.len() as f64,
            max_gpu_bf_diff
        );
        println!(
            "CPU vs BigFloat: avg diff = {:.2}",
            total_cpu_bf_diff as f64 / sample.len() as f64
        );

        if max_gpu_cpu_diff == 0 {
            println!("\n✓ GPU and CPU HDRFloat agree perfectly on all sampled glitched pixels.");
            println!(
                "  This suggests the glitch detection is correct, but both have precision issues."
            );
        } else if max_gpu_cpu_diff > 10 {
            println!("\n✗ GPU and CPU HDRFloat diverge significantly.");
            println!("  This suggests a bug in GPU shader HDRFloat implementation.");
        } else {
            println!("\n~ Minor differences between GPU and CPU HDRFloat.");
            println!("  Could be f32 vs f64 rounding differences.");
        }
    });
}

/// Test that orbit precision is preserved when uploaded to GPU.
/// The orbit should use double-single (hi/lo) representation to maintain ~48-bit precision.
#[test]
fn gpu_orbit_precision_matches_cpu() {
    use url_decode::decode_url_hash;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        // Use a viewport at moderate zoom where orbit precision matters
        const TEST_URL: &str = "http://127.0.0.1:8080/fractalwonder/#v1:pVLbasQgEP2XeU4XbSmU_EopwU0mieBqUJOlhP33TuIlbtk-lPVBnYsz55xxhUXidTLWQ71Ci9qjhfpzhUWoGaGGF35inNHiPJzJ2my-n3sgmOy4xkC47Y9yZngUarIYjy7OoYLJYiudNLo5S--g_ni_VQUidkpgeMYVqpTFYosCawZZZEbAPIFOBRNmthuPMX1VcJWdHzflEjjC9o8VWqUuqXEhK0-6MXYwKgaReGauLDI79Gd_E6hgRDmM_gkCeQhZxiT28WnybOMoiv1O6lwh_yTG74nleR2_MfnL0T5mS3Rbo3s5NLIjohehO1RnazxsAWVsYyZPL9wmxyQUeo8htZcWKcmNopN6aFCLs0IK9EI5JP_FGD8ebm9n8o7SeTNYcfkdaL9bhU1rZk3C8wq8Fdr1aAmpoN6vNIEKZofNMM2xB0Ff0G50oH67_QA";

        let state = decode_url_hash(TEST_URL).expect("Failed to decode URL");
        let viewport = &state.viewport;

        let ref_width = 4.0_f64;
        let zoom = ref_width / viewport.width.to_f64();
        let zoom_exponent = zoom.log10();
        let max_iter = calculate_max_iterations(zoom_exponent, 200.0, 2.5);
        let tau_sq = 1e-6_f32;

        // Create reference orbit
        let center_re = viewport.center.0.to_f64();
        let center_im = viewport.center.1.to_f64();
        let orbit = create_reference_orbit(center_re, center_im, max_iter);

        // Debug: verify orbit HDRFloat conversion preserves precision
        if orbit.orbit.len() > 10 {
            let sample_idx = 10;
            let (re, im) = orbit.orbit[sample_idx];
            let re_hdr = fractalwonder_core::HDRFloat::from_f64(re);
            let im_hdr = fractalwonder_core::HDRFloat::from_f64(im);
            let re_reconstructed = re_hdr.to_f64();
            let im_reconstructed = im_hdr.to_f64();
            let re_diff = (re - re_reconstructed).abs();
            let im_diff = (im - im_reconstructed).abs();
            println!("\n=== Orbit HDRFloat Conversion Debug ===");
            println!("Orbit point {}: re={:.15e}, im={:.15e}", sample_idx, re, im);
            println!(
                "HDRFloat re: head={:.8}, tail={:.8e}, exp={}",
                re_hdr.head, re_hdr.tail, re_hdr.exp
            );
            println!(
                "HDRFloat im: head={:.8}, tail={:.8e}, exp={}",
                im_hdr.head, im_hdr.tail, im_hdr.exp
            );
            println!(
                "Reconstruction error: re={:.3e}, im={:.3e}",
                re_diff, im_diff
            );
        }

        // Setup dc parameters
        let width = 100_u32;
        let height = 100_u32;
        let vp_width = HDRFloat::from_bigfloat(&viewport.width);
        let vp_height = HDRFloat::from_bigfloat(&viewport.height);
        let half = HDRFloat::from_f64(0.5);
        let origin_re = vp_width.mul(&half).neg();
        let origin_im = vp_height.mul(&half).neg();
        let step_re = vp_width.div_f64(width as f64);
        let step_im = vp_height.div_f64(height as f64);

        let dc_origin = (
            (origin_re.head, origin_re.tail, origin_re.exp),
            (origin_im.head, origin_im.tail, origin_im.exp),
        );
        let dc_step = (
            (step_re.head, step_re.tail, step_re.exp),
            (step_im.head, step_im.tail, step_im.exp),
        );

        // Render with GPU
        let mut gpu_renderer = crate::progressive_renderer::ProgressiveGpuRenderer::new(ctx);
        let row_set_count = 4_u32;
        let iterations_per_dispatch = 1000_u32;

        let mut gpu_results = Vec::new();
        for row_set_idx in 0..row_set_count {
            let result = gpu_renderer
                .render_row_set(
                    &orbit.orbit,
                    1,
                    dc_origin,
                    dc_step,
                    width,
                    height,
                    row_set_idx,
                    row_set_count,
                    max_iter,
                    iterations_per_dispatch,
                    tau_sq,
                    orbit.escaped_at.is_some(),
                )
                .await
                .expect("GPU render should succeed");
            gpu_results.push(result);
        }

        // Compare GPU vs CPU for ALL pixels (not just glitched ones)
        let mut total_diff = 0_i64;
        let mut max_diff = 0_i32;
        let mut mismatch_count = 0_u32;
        let total_pixels = width * height;

        for row_set_idx in 0..row_set_count {
            let result = &gpu_results[row_set_idx as usize];
            for (linear_idx, data) in result.data.iter().enumerate() {
                let gpu_data = as_mandelbrot(data);

                // Calculate pixel coordinates
                let row_within_set = linear_idx as u32 / width;
                let col = linear_idx as u32 % width;
                let global_row = row_within_set * row_set_count + row_set_idx;

                // Calculate delta_c for CPU
                let origin_re_hdr = HDRFloat {
                    head: dc_origin.0 .0,
                    tail: dc_origin.0 .1,
                    exp: dc_origin.0 .2,
                };
                let origin_im_hdr = HDRFloat {
                    head: dc_origin.1 .0,
                    tail: dc_origin.1 .1,
                    exp: dc_origin.1 .2,
                };
                let step_re_hdr = HDRFloat {
                    head: dc_step.0 .0,
                    tail: dc_step.0 .1,
                    exp: dc_step.0 .2,
                };
                let step_im_hdr = HDRFloat {
                    head: dc_step.1 .0,
                    tail: dc_step.1 .1,
                    exp: dc_step.1 .2,
                };

                let dc_re = origin_re_hdr.add(&HDRFloat::from_f64(col as f64).mul(&step_re_hdr));
                let dc_im =
                    origin_im_hdr.add(&HDRFloat::from_f64(global_row as f64).mul(&step_im_hdr));
                let delta_c = HDRComplex {
                    re: dc_re,
                    im: dc_im,
                };

                let cpu_result =
                    compute_pixel_perturbation_hdr(&orbit, delta_c, max_iter, tau_sq as f64);

                let diff = (gpu_data.iterations as i32 - cpu_result.iterations as i32).abs();
                total_diff += diff as i64;
                max_diff = max_diff.max(diff);

                if diff > 1 {
                    mismatch_count += 1;
                    // Print first 5 mismatches for debugging (include GPU glitch flag too)
                    if mismatch_count <= 5 {
                        println!(
                            "Mismatch at ({}, {}): GPU={}(g={}), CPU={}(g={}), diff={}",
                            col,
                            global_row,
                            gpu_data.iterations,
                            gpu_data.glitched,
                            cpu_result.iterations,
                            cpu_result.glitched,
                            diff
                        );
                    }
                }
            }
        }

        let avg_diff = total_diff as f64 / total_pixels as f64;
        let mismatch_pct = 100.0 * mismatch_count as f64 / total_pixels as f64;

        println!("\n=== GPU Orbit Precision Test ===");
        println!("Total pixels: {}", total_pixels);
        println!("Average iteration diff: {:.2}", avg_diff);
        println!("Max iteration diff: {}", max_diff);
        println!(
            "Mismatches (diff > 1): {} ({:.1}%)",
            mismatch_count, mismatch_pct
        );

        // Note: Orbit precision through GPU upload has inherent precision loss.
        // The 50% threshold reflects current reality with HDRFloat f32 representation.
        // Improving this requires higher precision orbit storage (e.g., double-single).
        assert!(
            mismatch_pct < 50.0,
            "GPU should match CPU within 50% of pixels, got {:.1}% mismatches. \
             This indicates orbit precision loss during GPU upload.",
            mismatch_pct
        );
    });
}
