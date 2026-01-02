//! Diagnostic test comparing CPU and GPU Mandelbrot renderers.
//!
//! Renders identical pixels through both pipelines and prints MandelbrotData
//! field-by-field to diagnose rendering differences.

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports, dead_code)] // Staged for subsequent implementation tasks
mod tests {
    use fractalwonder_compute::{render_tile_hdr, BlaTable, ReferenceOrbit, TileConfig};
    use fractalwonder_core::{BigFloat, ComputeData, HDRFloat, MandelbrotData, Viewport};
    use fractalwonder_gpu::{GpuAvailability, GpuContext, ProgressiveGpuRenderer};

    /// Test viewport parameters (extreme deep zoom ~10^-281)
    const CENTER_X: &str =
        "0.273000307495579097715200094310253922494103490187797182966812629706330340783242";
    const CENTER_Y: &str =
        "0.005838718497531293679839354462882728828030188792949767250660666951674130465532";
    const WIDTH: &str = "1.38277278476513331960149825811900065907944121299848E-281";
    const HEIGHT: &str = "7.97822331184022584815185255533429968247789646588334E-282";

    const IMAGE_WIDTH: u32 = 766;
    const IMAGE_HEIGHT: u32 = 432;
    const TEST_ROW: u32 = 350;
    const TEST_COL_START: u32 = 580;
    const TEST_COL_END: u32 = 611; // 32 pixels
    const MAX_ITERATIONS: u32 = 10_000_000;
    const TAU_SQ: f64 = 1e-6;
    const PRECISION_BITS: usize = 1067;

    fn parse_viewport() -> Viewport {
        Viewport::from_strings(CENTER_X, CENTER_Y, WIDTH, HEIGHT, PRECISION_BITS)
            .expect("valid viewport parameters")
    }

    fn compute_orbit_and_bla(viewport: &Viewport) -> (ReferenceOrbit, BlaTable) {
        println!("Computing reference orbit at center...");
        let c_ref = (viewport.center.0.clone(), viewport.center.1.clone());
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
        let canvas_width_bf = BigFloat::with_precision(IMAGE_WIDTH as f64, PRECISION_BITS);
        let canvas_height_bf = BigFloat::with_precision(IMAGE_HEIGHT as f64, PRECISION_BITS);
        let step_re = viewport.width.div(&canvas_width_bf);
        let step_im = viewport.height.div(&canvas_height_bf);

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
            .map(|cd| {
                let ComputeData::Mandelbrot(m) = cd;
                m
            })
            .collect()
    }

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
                    .map(|cd| {
                        let ComputeData::Mandelbrot(m) = cd;
                        m.clone()
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

    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!(
            "Viewport parsed (width ~10^{})",
            (viewport.width.log2_approx() * 0.301) as i32
        );

        let (orbit, bla_table) = compute_orbit_and_bla(&viewport);

        let cpu_pixels = render_cpu_pixels(&viewport, &orbit, &bla_table);
        println!("CPU rendered {} pixels", cpu_pixels.len());

        let gpu_pixels = pollster::block_on(render_gpu_pixels(&viewport, &orbit, &bla_table));

        match gpu_pixels {
            Some(gpu) => {
                println!("GPU rendered {} pixels", gpu.len());
                println!(
                    "  First pixel iterations: CPU={}, GPU={}",
                    cpu_pixels[0].iterations, gpu[0].iterations
                );
            }
            None => {
                println!("GPU not available, skipping comparison");
            }
        }
    }
}
