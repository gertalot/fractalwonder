//! Diagnostic test comparing CPU and GPU Mandelbrot renderers.
//!
//! Renders identical pixels through both pipelines and prints MandelbrotData
//! field-by-field to diagnose rendering differences.

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use fractalwonder_compute::{render_tile_hdr, BlaTable, ReferenceOrbit, TileConfig};
    use fractalwonder_core::{BigFloat, ComputeData, HDRFloat, MandelbrotData, Viewport};
    use fractalwonder_gpu::{GpuAvailability, GpuContext, ProgressiveGpuRenderer};

    /// Test viewport parameters (extreme deep zoom ~10^-301)
    const CENTER_X: &str = "0.2730003074955790977152000943102539224941034901877971829668126297063303407832423613869955086599456315688065774822084767969195008271260025052238452032391007263549994293766362921954037893505083246289924017524293579630007968065405107950517380261032289242338419178553209271869818436155991127814312651370767039529790390371669202702173115030152440733903704744891978701000886412551973402111747412797857562890678439266725717111306584186451326456823944631946842223793250196344441718118616611401798736302156711562899499186682755344237831667472378161701650189215869661245649090085723648929497017803093570326388018031614422864596391699687669402025984504194187361856596306389038337007658182380034042958910838285175922191326544301967582883277101985182849925838785480664935840850519795743754970843257750371804341910672629844310833648878540083867067043508254608465780851461280479281295322114922286840675529678822383095381787353285116296030807330820596880639823206658671223760724173129025359067138182337171720749800569";
    const CENTER_Y: &str = "0.005838718497531293679839354462882728828030188792949767250660666951674130465532588091396071885418790682911941182466374117896236132252584247402520010866544094350570137907725338151684505273501026943769605906645454851269816759514725340456638922976512726483271644272744008492930125282597595902682072300413706251167948205508278816766531246881090368207825659538929519971798157560790147006473552430833630039884532920761031884789517289671816155304035266250863755423721764653016583392886117574382942448562303298550801320226101222506312293295494413926654014742720571573642434093735724674792020205169969930225827819813160130800252073572525841270036373827679763141080348247834453184608392940600601324493040055811134675921524066069185692051227223857184990089862739945119771885138220215168430246298458776311304431218420943676051927134299650314210673350332280119864537824887838799141455684401072852310865411143709690861261408593780956964263045314581718744417705960109543941488382510390531275524147521938803586050469584";
    const WIDTH: &str = "1.567986963032091957511281424805116538272768892701202673614342266835786917712324039272158637058068343435873961531906768065059361422357389087253764156964276998744107441489466253063061493446648220908611150065008350334154024679881994516667079141201802718989407489906152769121710130351337244511829885510034912122428610599660020231730166775811381349123319593591721347675170998159202835661172858831966295909447244131441175784210879403880487553151624197206718695522416894705011946848362198686898061055808378466837446709112162796376007366348103549908549808410374984710482906821010861557450858642961681159934837548418520051557388178637724062209133553951062425965992470608705268777721374709620602999510883256706271639464006107556602354343441734459384745539778022712343614452553504102695377339581347644990287838782966381534056677429047233709417303264286837858901505394021215044427007903746747968057106405785873411674050497539310889491902715463477128207414744064264487008966207432912410279686978913240873537109969E-301";
    const HEIGHT: &str = "9.046858803522807376943115796714275461612707987334177667429547757668768011988907628063891622708356117967685045555700046420113754356418532342042939285148851379515969873489172226483383005389166730119625777557815864567264204609475885756455423207430661470858862452759424284453730843615825261193267015715221890877205583702933994735818885192198445874519112274657185887165170166784992675077231393669246542683795749560404761354682126638279730173616667503912062721635629816670137493990472026487378448059842133422410867919999399675806422748764817411645109964338292900360686664813238194262088222586782139356864305718913281819939546152958124072122700102599775924374943467675904234668399452222337545591778932056651862525777323494065202648679375115524674041058548833398687965834199402073042111712382706932023832617779318156077494262582725422536545113574712429588326705466203085504111911105643621072223159132228932036998937400817160437960572201377049777956516458241313303327125426958027408129822638276073307715462583E-302";

    const IMAGE_WIDTH: u32 = 773;
    const IMAGE_HEIGHT: u32 = 446;
    const TEST_ROW: u32 = 35;
    const TEST_COL_START: u32 = 0;
    const TEST_COL_END: u32 = 772; // Full width: 773 pixels
    const MAX_ITERATIONS: u32 = 10_000_000;
    const TAU_SQ: f64 = 1e-6;
    const PRECISION_BITS: usize = 1139;

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
        println!(
            "Rendering {} CPU pixels...",
            TEST_COL_END - TEST_COL_START + 1
        );

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
        let mut iteration_diffs = 0;
        let mut escaped_diffs = 0;
        let mut glitched_diffs = 0;
        let mut z_norm_diffs = 0;
        let mut surface_normal_diffs = 0;

        let mut max_iteration_diff: i64 = 0;
        let mut max_z_norm_diff: f32 = 0.0;
        let mut max_normal_re_diff: f32 = 0.0;
        let mut max_normal_im_diff: f32 = 0.0;

        let mut first_diffs: Vec<(u32, String)> = Vec::new();

        for (i, (cpu, gpu)) in cpu_pixels.iter().zip(gpu_pixels.iter()).enumerate() {
            let col = TEST_COL_START + i as u32;
            let mut has_diff = false;
            let mut diff_desc = Vec::new();

            if cpu.iterations != gpu.iterations {
                iteration_diffs += 1;
                has_diff = true;
                let d = (cpu.iterations as i64 - gpu.iterations as i64).abs();
                max_iteration_diff = max_iteration_diff.max(d);
                diff_desc.push(format!("iter diff={}", d));
            }
            if cpu.escaped != gpu.escaped {
                escaped_diffs += 1;
                has_diff = true;
                diff_desc.push("escaped mismatch".to_string());
            }
            if cpu.glitched != gpu.glitched {
                glitched_diffs += 1;
                has_diff = true;
                diff_desc.push("glitched mismatch".to_string());
            }
            if cpu.final_z_norm_sq != gpu.final_z_norm_sq {
                z_norm_diffs += 1;
                has_diff = true;
                let d = (cpu.final_z_norm_sq - gpu.final_z_norm_sq).abs();
                max_z_norm_diff = max_z_norm_diff.max(d);
                diff_desc.push(format!("z_norm diff={:.2e}", d));
            }
            if cpu.surface_normal_re != gpu.surface_normal_re
                || cpu.surface_normal_im != gpu.surface_normal_im
            {
                surface_normal_diffs += 1;
                has_diff = true;
                let d_re = (cpu.surface_normal_re - gpu.surface_normal_re).abs();
                let d_im = (cpu.surface_normal_im - gpu.surface_normal_im).abs();
                max_normal_re_diff = max_normal_re_diff.max(d_re);
                max_normal_im_diff = max_normal_im_diff.max(d_im);
                diff_desc.push(format!(
                    "normal CPU=({:.4},{:.4}) GPU=({:.4},{:.4})",
                    cpu.surface_normal_re,
                    cpu.surface_normal_im,
                    gpu.surface_normal_re,
                    gpu.surface_normal_im
                ));
            }

            if has_diff {
                diff_count += 1;
                if first_diffs.len() < 10 {
                    first_diffs.push((col, diff_desc.join(", ")));
                }
            }
        }

        // Print first 10 differences in detail
        if !first_diffs.is_empty() {
            println!("First {} pixels with differences:", first_diffs.len());
            for (col, desc) in &first_diffs {
                println!("  col {}: {}", col, desc);
            }
            println!();
        }

        // Print sample of actual values for analysis
        println!("Sample pixel values (actual, not diffs):");
        for i in [0, 100, 200, 300, 400, 500, 600, 700, 772].iter() {
            if *i < cpu_pixels.len() {
                let cpu = &cpu_pixels[*i];
                let gpu = &gpu_pixels[*i];
                println!(
                    "  col {}: CPU iter={} z_norm={:.2e} escaped={} | GPU iter={} z_norm={:.2e} escaped={}",
                    i, cpu.iterations, cpu.final_z_norm_sq, cpu.escaped,
                    gpu.iterations, gpu.final_z_norm_sq, gpu.escaped
                );
            }
        }
        println!();

        println!("========================================");
        println!(
            "SUMMARY: {} of {} pixels have differences",
            diff_count,
            cpu_pixels.len()
        );
        println!(
            "  - Iteration diffs:      {} (max diff: {})",
            iteration_diffs, max_iteration_diff
        );
        println!("  - Escaped diffs:        {}", escaped_diffs);
        println!("  - Glitched diffs:       {}", glitched_diffs);
        println!(
            "  - Z norm diffs:         {} (max: {:.2e})",
            z_norm_diffs, max_z_norm_diff
        );
        println!(
            "  - Surface normal diffs: {} (max re: {:.2e}, im: {:.2e})",
            surface_normal_diffs, max_normal_re_diff, max_normal_im_diff
        );
        println!("========================================\n");
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
        println!(
            "Rendering GPU pixels (full image, extracting row {})...",
            TEST_ROW
        );

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
                0, // row_set_index
                1, // row_set_count (all rows in one set)
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

                println!(
                    "GPU extracted {} pixels from row {}",
                    pixels.len(),
                    TEST_ROW
                );
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
        println!("\n========== CPU/GPU DIAGNOSTIC TEST ==========\n");

        let viewport = parse_viewport();
        println!(
            "Viewport: width ~10^{}, precision {} bits",
            (viewport.width.log2_approx() * 0.301) as i32,
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
}
