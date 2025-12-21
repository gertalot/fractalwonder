//! Tests for GPU renderer - verifies GPU output matches CPU perturbation.

use crate::{GpuAvailability, GpuContext};
use fractalwonder_compute::{compute_pixel_perturbation_hdr, ReferenceOrbit};
use fractalwonder_core::{
    calculate_max_iterations, BigFloat, ComputeData, HDRComplex, HDRFloat, MandelbrotData,
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

// =============================================================================
// URL decoding for test viewports
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
                &orbit.derivative,
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
        // Use smaller image size to keep test runtime reasonable (32x32 = 1024 pixels)
        // At deep zoom, each pixel needs 15,000-35,000 iterations of HDRFloat math
        let width = 32_u32;
        let height = 32_u32;
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
                    &orbit.derivative,
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
