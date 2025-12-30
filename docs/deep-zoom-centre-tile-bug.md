# Deep Zoom Center Tile Bug

At ~10^270 zoom, center tiles hang during CPU rendering while GPU works fine.

test cases:

```bash
# Verifies HDRFloat dc_max does NOT underflow - PASSES
cargo test --package fractalwonder-compute dc_max_at_extreme_zoom -- --nocapture

# This test still HANGS (additional issue beyond dc_max underflow)
cargo test --package fractalwonder-compute deep_zoom_full_tile_with_bla -- --ignored --nocapture

# This test is SLOW but progresses (no BLA)
cargo test --package fractalwonder-compute deep_zoom_full_tile_without_bla -- --ignored --nocapture
```

## Test Case Details

**Location:** `fractalwonder-compute/src/perturbation/tests/deep_zoom_center_tile.rs`

**Failing URL (decoded):**
```
http://127.0.0.1:8080/fractalwonder/#v1:7ZvLbhvZFUX_hWMh2CcIMtA8XxEEhCyVbQJqyaBodxpG_3soiXXvWqfoadADlV9iFe_rPPbZZxf8c_fjsPz-7fl42t3-3N0vT6fluLv998_dj7vH78vudpe_pZLznzpfrz_U28-vd94evN16ffT2J1lvvX_nMuz91_vjy_jLo8u4y5fXdd4_XpZ7XyE1Zl2Hr1_BBufn9Z91A2Nnucxd686K84-nqTHP-jfmX9e8zJQ55dh_u5d1-rHoOknNm-tpxwnHynM1HDjDOGMfq2Hn8plng3PGbnj-y94LFh3TVabBa3hznqV5cXUXxhXHa_rIP5kGjJ7PZddHbe14IzPiZugMC03zjBkZdGMMYm41T8LQDCzMJabD56amaddtzU3PcJ1_zUiEpUaIzTAb0TgchxRDLGtfY7V5jJqzFzzYDL9-GfkyP4dbnNcIoBllzqyWqA7iYSv7tpCd3DLinhE3g3hk0Qzk6U3Fk2ZiwOIBgnmaf2wLGT7PMjeRbNw5jodErdpMYHOP78PmTmn6ITz6BqNHnGX-DhPcR0xfeyJWbXK1A-Xc3ghlmEsAVATIHjoTFQogMjZFJ7DMCHxrumZWiFnYEG5EyRAzkNM4x8QKOXdueeZYIcDC-AHgK6ILnqKj54ojX2PPVBgNV-opQs-gn2wqOYsiFp5rJzQ4Xaw9F-rnXBsVTBNEsDQZgDBfqTUHIlUrpBMjV42PDYkKSTELCrFLbGR3s_t2XO4PL4fnp_2nw-lld1v5-z__vBHV4mIon7QgaAvcq0hSPSiWGiMSqUpgTjEW1kGFhKhPULZIYbip6VCgNYahipYZmtgRjj49TChymmZLmbhUj63iHpMtjJEtcYhzOmRnBqKOU4AoU9_o8GA2rkDyX7lS6FNKgRsyF5WVIg6X8Bs4AAJZHeoVb1UuKKgqAH9wkogtC0YKlJeEDRi8hgopAKMLeUScwbQl4FGsgLyonJDCMUGYzjPVel2N-HxE3kX6AkwAJQr9BNwk7ovksxg5dJTHwEv0LM5E94XVaOpMFECrylJMEqdrczVzOiQxMgTFpKfshXhNEKpNGaZh0TJiUPUzmRQnG_RSp1YoySTkrHNB98GZCf-iOxFoIfVJ1Vjy5lkUKHFtLxMkNZTEHe0xNiAo7YhCdBvVGzxXMedfymmu2GYXnTK2kCu7FTNnUx8pdl9CrojzzA5CVmhhi0ZVbSqZDoG1talpZy3VSeF9V1HIsLsYgBwNGXd3Qal9VJiwjSpx0wYrxYZAaIZS4KawYRPpaIkLjTP_inH952b3--Hh9PVV8NpQr4_r4_q4_v8XGIs6vXQFZKseEgZIW6800q0LALdIldlw2N-ijKtBv1Y2B1xFPVs2uJVOwPvKaF2qrhaqhEpZrJ-JpVQjC9I4pwswxlperOym0e2gVFgJLPbNG9gm9S92GShQwU1OFStntCyKREDR5UCSRRBBFbVIyBbHpohY1jtIlCRNwktdvi3Lq-j91emoUbna-gQ6hNUrTLqNXuxA6gd4uZhrQnoq4YH8Xl1Qy2OfTjKajc9zMrApC0jCqw1XpqrgiaF-paIWbXZ2lCyakTlN0NFNohuQslBMc0vfM13sEtmA6JwxDzeWE3VzrKjvc1sV5q5VjtnoILOcDtQUShhFS0XCztWs64Zs2EAFLJPyFXdZxjdzZXZAaXFUkJnZIkXNmDK35UGlyWqMcYeDwFAqF-CIrpc-wyYaIIGOOhRtFWzlJi1ECgJIkcIjIbbZAJSVEa-ISY5oMHwVJOBOSsipwC8Wy6j57hU1KqnxB9exksxScbcHZsEE9JE7SAIei5lqKSxNuACDoGTgfvr1h18KzLuvy-HL19NHu_NxfVx_kUuYn2oMIaK5FTVFgggJdARxKo2XcUTVSAcPkVZvDSj5BFzfGnlYYsJqLV1q221RV-sSJu4SWi0C85VGJGNRB2RfwF7Q7WU0EujcpMlqnIwUuR2EdkgrZ2pHzXCTbV1DgAxDmJPPromVnlSx0QG_gVEZr67WSonU2whKqBZ3m3rvoK5qrZLf-og9t6alWtyKkldJmFYPfvW9EPworXuchyJCrFKSd7DnSSk61HEzuRFom5AwM-U6DvAyK6YgjQOYKzElZDG38RZcW2wiSa2O00GR4ovgKGVc48-hkBJBTMcmm0LItFV1TGjlTuVDGlIWFJ2uuqShoN7OpK3JA1uC32x3240n8r2lHrc3yk1q-UhFYZEiABnt2hJigdNRin2qty96_eHXE8p6n4lpGb16sZazef0h7bA2yWqSHUuJZOm9PPbKY9VlU9qdl0Ar1Ev6pqkjrRJtuusCkqlMh9XI4iUQFXpDkzdnHQ-xMDJ2aDCIA5JM1TG2rs0mN360BJvgJcAoYbwKOwXDKkorktuk_mz-tXaJEgoxMSVJRnW0FUHKANBSVDEtRZVObVSIjCLJq4oiXMd2eMIEIQS4WYXCPj8sBFS91OFH3LJpJmK9aYWqGIyVxlHAbBquM3iVNFaqoHu5eMkn80w8SKQVXNHgcoVFm7wgl8XQwCqRdwz2WDto-FZQZUgByOU2-caA3zIC2Rngvm7oV3LDWW-4f376fPiyPzyclYbf7p4elsdPx-fT64C7x-V0WvZPd7-9qhD_elzuT8fD_fnJcTl_7bh_OT8-PH15efu_CH_cPy77--fvT2fpom5231-W_Zdv33e3n-8eX5ab3X-Pd3_sl6e7T4_Lw7j5-fl4v-y_Phz3nx-f706X--c9_ViOrzvd3f7jz_8B```

Decode script: scripts/decode_url.py

VIEWPORT:
  Center X:  0.273000307495579097715200094310253922494103490187797182966812629706330340783242
  Center Y:  0.005838718497531293679839354462882728828030188792949767250660666951674130465532
  Width:     3.68629585526668733757870313779318701180348758566795E-270
  Height:    2.12689256332334093913116602106093685402570700118706E-270
  Zoom Depth: ~10^269 (2^895)
  Precision:  1026 bits

**Canvas:** 773x446 pixels

**Tile size:** 32x32 (DEEP_ZOOM_TILE_SIZE at this zoom)

**Center tiles (sorted by distance from canvas center):**
1. (384, 192) - 20.18px from center
2. (384, 224) - 21.71px from center
3. (352, 192) - 23.82px from center
4. (352, 224) - 25.12px from center

---

## Code path

 Complete Code Path Analysis: Center Tile Rendering

  Step 1: URL Parsing (persistence.rs:310-331)

  The URL hash v1:7ZvLbh... is decoded via:
  1. load_from_url_hash() extracts the hash fragment
  2. decode_state() strips v1:, base64-decodes, deflate-decompresses, JSON-deserializes
  3. Returns PersistedState containing:

  viewport: {
    center: (0.27300030749557909..., 0.00583871849753129...)
    width:  3.686e-270
    height: 2.127e-270
    precision: 1026 bits
  }
  use_gpu: false
  force_hdr_float: false

  Step 2: Render Initiated (parallel_renderer.rs:186-240)

  ParallelRenderer::render() is called with this viewport and a 773x446 canvas.

  // parallel_renderer.rs:208-214
  let reference_width = self.config.default_viewport(viewport.precision_bits()).width; // 4.0
  let zoom = reference_width.to_f64() / viewport.width.to_f64(); // 4.0 / 3.686e-270 = 1.08e270
  let tile_size = calculate_tile_size(zoom); // tiles.rs:48-57

  Since zoom >= 1e7, tile_size = 32 pixels.

  Step 3: Tile Generation (tiles.rs:64-95)

  let tiles = generate_tiles(773, 446, 32);

  This creates a grid of 32x32 tiles (25×14 = 350 tiles total, some edge tiles smaller).

  For a 773×446 canvas with 32px tiles:
  - Columns: ceil(773/32) = 25 tiles (0-767 full, 768-772 partial)
  - Rows: ceil(446/32) = 14 tiles (0-447 full, but 446 means row 13 is partial)

  Center tiles are sorted first (by distance from canvas center 386.5, 223):
  - Canvas center: (386.5, 223.0)
  - Center tiles are around pixel (384, 192) and (384, 224), (352, 192), etc.

  The 4 center tiles would be approximately:
  1. Tile at (384, 192), size 32×32 → center at (400, 208), dist from (386.5,223) = ~19.5
  2. Tile at (352, 192), size 32×32 → center at (368, 208), dist ≈ 22.8
  3. Tile at (384, 224), size 32×32 → center at (400, 240), dist ≈ 21.6
  4. Tile at (352, 224), size 32×32 → center at (368, 240), dist ≈ 24.0

  Step 4: Perturbation Render Start (worker_pool.rs:490-548)

  Since use_gpu: false, the code goes to:
  // parallel_renderer.rs:231-238
  self.worker_pool.borrow_mut().start_perturbation_render(
      viewport.clone(),
      (773, 446),
      tiles,
      force_hdr_float,  // false
  );

  Inside start_perturbation_render:

  // worker_pool.rs:503-514
  self.perturbation.set_force_hdr_float(force_hdr_float); // false
  let orbit_request = self.perturbation.start_render(
      self.current_render_id, &viewport, (773, 446)
  )?;

  Step 5: PerturbationCoordinator.start_render (coordinator.rs:153-215)

  Critical calculations happen here:

  // coordinator.rs:174-187
  self.state.max_iterations = calculate_render_max_iterations(viewport, config);
  // helpers.rs:26-41: zoom_exp = log10(4.0 / 3.686e-270) = 270.03
  // max_iter = 200 * 270.03^2.8 = 200 * 1,582,138 ≈ 316,427,600
  // BUT capped at reasonable value by calculate_max_iterations()

  self.state.tau_sq = 1e-6;  // from config
  self.state.dc_max = calculate_dc_max(viewport);  // sqrt((1.843e-270)² + (1.063e-270)²) ≈ 2.1e-270
  self.state.bla_enabled = true;

  // Delta step per pixel
  let precision = 1026;
  let canvas_width_bf = BigFloat::with_precision(773.0, 1026);
  let canvas_height_bf = BigFloat::with_precision(446.0, 1026);
  self.state.delta_step = (
      viewport.width.div(&canvas_width_bf),   // 3.686e-270 / 773 = 4.77e-273 per pixel X
      viewport.height.div(&canvas_height_bf), // 2.127e-270 / 446 = 4.77e-273 per pixel Y
  );

  c_ref is the viewport center (0.273..., 0.00583...) serialized to JSON.

  Step 6: Reference Orbit Computation (worker.rs:220-262)

  A worker receives MainToWorker::ComputeReferenceOrbit:
  // worker.rs:240
  let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

  In perturbation.rs:50-103:
  // Iterates z = z² + c at 1026-bit BigFloat precision
  // Stores orbit as Vec<(f64, f64)> - f64 is safe because |z| < 256
  // Also computes derivative Der_n = 2·Z·Der + 1

  The orbit is sent back and distributed to all workers via StoreReferenceOrbit.

  Step 7: Build Tile Message (coordinator.rs:265-319)

  For each tile, build_tile_message() computes delta_c_origin:

  // coordinator.rs:269-278
  // For tile at (384, 192):
  let norm_x = 384.0 / 773.0 - 0.5;  // = -0.003106... (close to center)
  let norm_y = 192.0 / 446.0 - 0.5;  // = -0.0695...

  let norm_x_bf = BigFloat::with_precision(norm_x, 1026);
  let norm_y_bf = BigFloat::with_precision(norm_y, 1026);

  // delta_c_origin = offset from viewport center in fractal space
  let delta_c_origin = (
      norm_x_bf.mul(&viewport.width),   // -0.003106 * 3.686e-270 = -1.15e-273
      norm_y_bf.mul(&viewport.height),  // -0.0695 * 2.127e-270 = -1.48e-272
  );

  For the center-most tile, delta_c_origin values are VERY SMALL (~10^-272 to 10^-273).

  The message sent to workers:
  MainToWorker::RenderTilePerturbation {
      render_id,
      tile: PixelRect { x: 384, y: 192, width: 32, height: 32 },
      orbit_id,
      delta_c_origin_json: "[[BigFloat ~-1.15e-273], [BigFloat ~-1.48e-272]]",
      delta_c_step_json: "[[BigFloat ~4.77e-273], [BigFloat ~4.77e-273]]",
      max_iterations,
      tau_sq: 1e-6,
      bla_enabled: true,
      force_hdr_float: false,  // ← KEY PARAMETER
  }

  Step 8: Worker Receives Tile (worker.rs:324-398)

  The worker parses the message and calls render_tile():

  // worker.rs:373-384
  let input = TileRenderInput {
      delta_c_origin,       // (BigFloat ~-1.15e-273, BigFloat ~-1.48e-272)
      delta_c_step,         // (BigFloat ~4.77e-273, BigFloat ~4.77e-273)
      tile_width: 32,
      tile_height: 32,
      max_iterations,
      tau_sq: 1e-6,
      bla_enabled: true,
      force_hdr_float: false,  // ← IMPORTANT
  };

  let data = render_tile(&orbit, cached.bla_table.as_ref(), &input);

  Step 9: Render Tile Dispatch (tile_render.rs:28-127)

  THIS IS THE CRITICAL DECISION POINT:

  // tile_render.rs:36-41
  let delta_log2 = input.delta_c_origin.0.log2_approx()
      .max(input.delta_c_origin.1.log2_approx());
  // delta_log2 ≈ log2(1.48e-272) ≈ -903

  let deltas_fit_f64 = !input.force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;
  // deltas_fit_f64 = !false && (-903 > -900) = false && true = FALSE

  Because delta_log2 ≈ -903 < -900, the code takes the HDRFloat path:

  // tile_render.rs:83-123 (HDRFloat path)
  let delta_origin = HDRComplex {
      re: HDRFloat::from_bigfloat(&input.delta_c_origin.0),  // head≈-0.59, exp≈-903
      im: HDRFloat::from_bigfloat(&input.delta_c_origin.1),  // head≈-0.76, exp≈-900
  };
  let delta_step = HDRComplex {
      re: HDRFloat::from_bigfloat(&input.delta_c_step.0),   // head≈0.61, exp≈-905
      im: HDRFloat::from_bigfloat(&input.delta_c_step.1),   // head≈0.61, exp≈-905
  };

  let mut delta_c_row = delta_origin;

  for _py in 0..32 {
      let mut delta_c = delta_c_row;
      for _px in 0..32 {
          let result = if input.bla_enabled {
              compute_pixel_perturbation_hdr_bla(
                  orbit, bla_table, delta_c, max_iterations, tau_sq
              )
          } else {
              compute_pixel_perturbation(orbit, delta_c, max_iterations, tau_sq)
          };
          data.push(ComputeData::Mandelbrot(result));
          delta_c.re = delta_c.re.add(&delta_step.re);  // HDRFloat add
      }
      delta_c_row.im = delta_c_row.im.add(&delta_step.im);  // HDRFloat add
  }

  Step 10: Pixel Computation (perturbation.rs:107-310 or 314-437)

  With BLA enabled, each pixel calls compute_pixel_perturbation_hdr_bla():

  // perturbation.rs:107-310
  fn compute_pixel_perturbation_hdr_bla(
      orbit: &ReferenceOrbit,
      bla_table: &BlaTable,
      delta_c: HDRComplex,  // e.g., (head=-0.59, exp=-903), (head=-0.76, exp=-900)
      max_iterations: u32,
      tau_sq: f64,          // 1e-6
  ) -> MandelbrotData {
      let mut dz = HDRComplex::ZERO;
      let mut drho = HDRComplex::ZERO;
      let mut m: usize = 0;  // orbit index
      let mut n: u32 = 0;    // iteration count
      let mut glitched = false;

      let mut loop_count: u32 = 0;
      let max_loops = max_iterations.saturating_mul(4);  // safety limit

      while n < max_iterations {
          loop_count += 1;
          if loop_count > max_loops {
              glitched = true;
              break;
          }

          let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];
          let z_re = HDRFloat::from_f64(z_m_re).add(&dz.re);
          let z_im = HDRFloat::from_f64(z_m_im).add(&dz.im);

          let z_mag_sq = z_re.square().add(&z_im.square()).to_f64();
          let dz_mag_sq = dz.norm_sq();

          // Escape check
          if z_mag_sq > 65536.0 { return escaped... }

          // Glitch detection
          if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
              glitched = true;
          }

          // Rebase check - CRITICAL FOR CENTER TILES
          if z_mag_sq < dz_mag_sq {
              dz = HDRComplex { re: z_re, im: z_im };
              drho = ...;
              m = 0;
              continue;  // n does NOT increment!
          }

          // Try BLA acceleration
          if let Some(bla) = bla_table.find_valid(m, dz_mag_sq) {
              // Skip iterations using BLA
              dz = apply_bla(dz, delta_c, bla);
              m += bla.l;
              n += bla.l;
          } else {
              // Standard delta iteration
              dz = 2·Z_m·dz + dz² + delta_c
              drho = 2·Z_m·drho + 2·dz·Der_m + 2·dz·drho
              m += 1;
              n += 1;
          }
      }

      // Returns interior point (not escaped)
      MandelbrotData { iterations: max_iterations, escaped: false, glitched, ... }
  }

  ---
  Summary: The Center Tile Bug's Root Cause Location

  For center tiles at extreme zoom (10^270):

  1. delta_c_origin ≈ 10^-272 to 10^-273 (very small offset from viewport center)
  2. delta_c_step ≈ 10^-273 per pixel
  3. Path taken: HDRFloat (because log2(delta) ≈ -903 < -900)

  The computation uses compute_pixel_perturbation_hdr_bla() at perturbation.rs:107-310.

  The bug manifests when:
  - δz grows large enough that z_mag_sq < dz_mag_sq triggers rebasing
  - Rebasing resets m = 0 but does NOT increment n
  - If this happens repeatedly, the loop_count safety limit is hit
  - Result: glitched = true, pixel marked as interior (black)

  Exact files and line numbers for the computation:
  - fractalwonder-compute/src/tile_render.rs:28-127 - dispatch logic
  - fractalwonder-compute/src/perturbation.rs:107-310 - HDR+BLA computation
  - fractalwonder-compute/src/perturbation.rs:194-204 - rebase logic (the likely culprit)