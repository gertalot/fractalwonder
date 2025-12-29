//! Reproduction test for the deep zoom center tile bug.
//!
//! At ~10^270 zoom with 10M iterations, center tiles fail to render with CPU while GPU works.
//! This test reproduces the EXACT conditions using EXACT production code paths and parameters.
//!
//! URL decoded from: http://127.0.0.1:8080/fractalwonder/#v1:...
//! Canvas size: 773x446
//! Tile size at this zoom: 32x32 (DEEP_ZOOM_TILE_SIZE)

use super::helpers::TEST_TAU_SQ;
use crate::bla::BlaTable;
use crate::perturbation::{compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla};
use crate::ReferenceOrbit;
use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat, Viewport};

// =============================================================================
// EXACT VALUES FROM DECODED URL (Canvas: 773x446, Tile size: 32)
// =============================================================================

/// Canvas dimensions from the actual failing render
const CANVAS_WIDTH: u32 = 773;
const CANVAS_HEIGHT: u32 = 446;

/// Tile size at deep zoom (from tiles.rs: DEEP_ZOOM_TILE_SIZE = 32)
const TILE_SIZE: u32 = 32;

/// The EXACT c_ref JSON from the decoded URL.
/// Center: (0.2730003..., 0.0058387...)
/// This is in BigFloat's JSON serialization format (binary representation).
const C_REF_JSON: &str = r#"[{"value":"0.0100010111100011010110010010000001111101110100000000110110011011110101010010000101011011011111011101110011000101111110011100011000011101001101100011011010111000101000101111100010100001010000011110100000000010100111011111010110011101001100100111010110100001000001000111100010101010001001110110001110111101111110000011101111011100011001011010110001010001101011010111100001101101101000010001100010011100010100010101000011100111011001011011100001110110110100110000000001101001100101011001110100011111111010001011101101110010010111000111010010001010110001110111001100010010011010100101110011011100011100010111011011001000000000100111110000000110110010000110100100000000110011000111011100000011101110011100100001000110110000010110110011101101010000000100111000110100011011110110001100111111100110001100001010110100110110000010110000110010100111011101110100010000100010000001110010000011100111001111110101101101101100101011111101110000011010001001010001011011011011110101101000001100001100100000111000000101011110101111111010011010101101001000000000101011101100100111101110110000010101111111111111111001001011000111111101000001000111001111010000000101110000111010100010001011100111001010010000010011110100110111011011010111000111000001001011110001010010100100011100001010011101110000010110110101111110110110011011011010100010111011100010110100001001100110101101111010100001111000100001111110111000001011100101110100110011111011110101000011011000001010110101110101111111111010001111111101111110010010000101001000101111010101001000110010101001001001000110111001100101101000010010010111010111111100011000101000000011001100010111011110111111111111101100001101010101101111110110111001011111000011010000100011100111000000001110010011111010101111101101011011100010111111010101101011111100001111011101111100111010000100100000101111101000100110001101000000100000010101011011101100001011011100101110000000011110110100101110110011010001100110101010101000010001110000010110101001001111101111110100001011001010111001000110001010010001001000010010111000101000101101100111110011101001000100000001010110110111010101000100000101100100101011100001111010101100101101111111101111110100111111100000011111001101001100000101011001011000010000000001111110001001000110110001011100000101001011000000010110100101010111000100110111100011100000010111101010001000111000110111010110011100000101110001100010100001","precision_bits":1026},{"value":"0.000000010111111010100101011100001111111101011000101100101000000011110111010001001010100010001000010011001101011111111000001001100001000110111000110000110111101111011001111101110110001110011001110011000110110110000111101110000111110001111010011100111001101010011011011110111110101110101111000010110001011011011000100001001001010010000010111101000101000101010001010100110111101001100000101101111100011100100010100000001100001110100011000101101111010111000000000010100001110110101011111010000001110010011010111110111111100111000101010111010111011101110010110011011011000000100110001111110111101011111101111010011000111000110001011010011001010000110111011010101011100001100110000111010010011001010110101010000000110010001000010011111100101010010000100100100111111011001001000001101000110010110001001010111011000011100101001110001111000110100101111001101101011101101010011010001111111111000000111100011010101100010001110011101010110000001100100100111000010010100001001110011110100001011010010110101110100010011111010011100101100011100101","precision_bits":1026}]"#;

/// The EXACT viewport width from the decoded URL.
/// Width: 3.68629585526668733757870313779318701180348758566795E-270
const VIEWPORT_WIDTH_STR: &str = "3.68629585526668733757870313779318701180348758566795E-270";

/// The EXACT viewport height from the decoded URL.
/// Height: 2.12689256332334093913116602106093685402570700118706E-270
const VIEWPORT_HEIGHT_STR: &str = "2.12689256332334093913116602106093685402570700118706E-270";

/// Production max iterations from the failing render
const MAX_ITERATIONS: u32 = 10_000_000;

/// Production precision bits
const PRECISION_BITS: usize = 1026;

// =============================================================================
// Helper Functions (EXACT same code as production)
// =============================================================================

/// Parse BigFloat tuple from JSON (EXACT same code as worker.rs)
fn parse_bigfloat_tuple(json: &str) -> (BigFloat, BigFloat) {
    serde_json::from_str(json).expect("Valid BigFloat tuple JSON")
}

/// Create viewport from production values.
/// This is the EXACT viewport that causes the bug.
fn create_production_viewport() -> Viewport {
    let c_ref = parse_bigfloat_tuple(C_REF_JSON);
    let width =
        BigFloat::from_string(VIEWPORT_WIDTH_STR, PRECISION_BITS).expect("Valid viewport width");
    let height =
        BigFloat::from_string(VIEWPORT_HEIGHT_STR, PRECISION_BITS).expect("Valid viewport height");

    Viewport {
        center: c_ref,
        width,
        height,
    }
}

/// Calculate dc_max EXACTLY as production does (same as helpers.rs:calculate_dc_max).
/// Uses HDRFloat to avoid underflow when squaring very small viewport dimensions.
fn calculate_dc_max(viewport: &Viewport) -> HDRFloat {
    let half_width = HDRFloat::from_bigfloat(&viewport.width).div_f64(2.0);
    let half_height = HDRFloat::from_bigfloat(&viewport.height).div_f64(2.0);
    half_width.square().add(&half_height.square()).sqrt()
}

/// Calculate delta_c_origin for a tile EXACTLY as coordinator.rs does (lines 269-278).
fn calculate_delta_c_origin(
    tile_x: u32,
    tile_y: u32,
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> (BigFloat, BigFloat) {
    let precision = viewport.width.precision_bits();

    // Exactly as coordinator.rs lines 270-271
    let norm_x = tile_x as f64 / canvas_size.0 as f64 - 0.5;
    let norm_y = tile_y as f64 / canvas_size.1 as f64 - 0.5;

    // Exactly as coordinator.rs lines 273-278
    let norm_x_bf = BigFloat::with_precision(norm_x, precision);
    let norm_y_bf = BigFloat::with_precision(norm_y, precision);
    (
        norm_x_bf.mul(&viewport.width),
        norm_y_bf.mul(&viewport.height),
    )
}

/// Calculate delta_c_step EXACTLY as coordinator.rs does (lines 181-187).
fn calculate_delta_c_step(viewport: &Viewport, canvas_size: (u32, u32)) -> (BigFloat, BigFloat) {
    let precision = viewport.width.precision_bits();
    let canvas_width_bf = BigFloat::with_precision(canvas_size.0 as f64, precision);
    let canvas_height_bf = BigFloat::with_precision(canvas_size.1 as f64, precision);
    (
        viewport.width.div(&canvas_width_bf),
        viewport.height.div(&canvas_height_bf),
    )
}

/// Create reference orbit using EXACT same method as production.
fn create_production_reference_orbit() -> ReferenceOrbit {
    let c_ref = parse_bigfloat_tuple(C_REF_JSON);
    ReferenceOrbit::compute(&c_ref, MAX_ITERATIONS)
}

/// Convert BigFloat tuple to HDRComplex (EXACT same code as worker.rs lines 426-433)
fn bigfloat_to_hdr_complex(bf_tuple: &(BigFloat, BigFloat)) -> HDRComplex {
    HDRComplex {
        re: HDRFloat::from_bigfloat(&bf_tuple.0),
        im: HDRFloat::from_bigfloat(&bf_tuple.1),
    }
}

// =============================================================================
// Production-Matching Tests
// =============================================================================

/// This test reproduces the EXACT failing condition from production.
/// It computes ALL 1024 pixels (32x32) of the center tile, exactly as production does.
///
/// Run with: cargo test deep_zoom_full_tile -- --ignored --nocapture
#[test]
#[ignore]
fn deep_zoom_full_tile_with_bla() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);

    // The first center tile (closest to canvas center): PixelRect(384, 192, 32, 32)
    let tile_x = 384u32;
    let tile_y = 192u32;

    println!("=== DEEP ZOOM FULL TILE TEST ===");
    println!("Canvas: {}x{}", CANVAS_WIDTH, CANVAS_HEIGHT);
    println!("Tile: ({}, {}, {}, {})", tile_x, tile_y, TILE_SIZE, TILE_SIZE);
    println!("Viewport width: {}", VIEWPORT_WIDTH_STR);
    println!("Viewport height: {}", VIEWPORT_HEIGHT_STR);
    println!();

    // Calculate dc_max EXACTLY as production does
    let dc_max = calculate_dc_max(&viewport);
    println!("dc_max = {:e} (log2 = {:.1})", dc_max.to_f64(), dc_max.log2());

    // Simulate JSON round-trip for dc_max (like production)
    let dc_max_json = serde_json::to_string(&dc_max).expect("serialize dc_max");
    let dc_max: HDRFloat = serde_json::from_str(&dc_max_json).expect("deserialize dc_max");
    println!("dc_max after JSON: {:e} (log2 = {:.1})", dc_max.to_f64(), dc_max.log2());
    println!();

    // Compute reference orbit
    println!("Computing reference orbit ({} iterations)...", MAX_ITERATIONS);
    let orbit = create_production_reference_orbit();
    println!(
        "Orbit: {} points, escaped_at={:?}",
        orbit.orbit.len(),
        orbit.escaped_at
    );

    // Simulate JSON round-trip for orbit (like production)
    // In production: ReferenceOrbitComplete -> main thread -> StoreReferenceOrbit -> worker
    let orbit_json = serde_json::to_string(&orbit.orbit).expect("serialize orbit");
    let derivative_json = serde_json::to_string(&orbit.derivative).expect("serialize derivative");
    let orbit_data: Vec<(f64, f64)> = serde_json::from_str(&orbit_json).expect("deserialize orbit");
    let derivative_data: Vec<(f64, f64)> =
        serde_json::from_str(&derivative_json).expect("deserialize derivative");
    let orbit = ReferenceOrbit {
        c_ref: orbit.c_ref,
        orbit: orbit_data,
        derivative: derivative_data,
        escaped_at: orbit.escaped_at,
    };
    println!("Orbit after JSON round-trip: {} points", orbit.orbit.len());
    println!();

    // Build BLA table
    let bla_table = BlaTable::compute(&orbit, dc_max);
    println!(
        "BLA table: {} entries, {} levels",
        bla_table.entries.len(),
        bla_table.num_levels
    );
    println!();

    // Calculate delta values EXACTLY as coordinator.rs does
    let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
    let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);

    // Simulate JSON round-trip for deltas (like production)
    let origin_json = serde_json::to_string(&delta_c_origin).expect("serialize origin");
    let step_json = serde_json::to_string(&delta_c_step).expect("serialize step");
    let delta_c_origin: (BigFloat, BigFloat) =
        serde_json::from_str(&origin_json).expect("deserialize origin");
    let delta_c_step: (BigFloat, BigFloat) =
        serde_json::from_str(&step_json).expect("deserialize step");

    println!(
        "delta_c_origin log2: re={:.1}, im={:.1}",
        delta_c_origin.0.log2_approx(),
        delta_c_origin.1.log2_approx()
    );
    println!(
        "delta_c_step log2: re={:.1}, im={:.1}",
        delta_c_step.0.log2_approx(),
        delta_c_step.1.log2_approx()
    );
    println!();

    // Convert to HDRComplex (EXACTLY as worker.rs does)
    let delta_origin = bigfloat_to_hdr_complex(&delta_c_origin);
    let delta_step = bigfloat_to_hdr_complex(&delta_c_step);

    // Compute ALL pixels in the tile (EXACTLY as worker.rs lines 436-464)
    println!(
        "Computing {} pixels ({} x {})...",
        TILE_SIZE * TILE_SIZE,
        TILE_SIZE,
        TILE_SIZE
    );

    let mut results = Vec::with_capacity((TILE_SIZE * TILE_SIZE) as usize);
    let mut delta_c_row = delta_origin;

    for py in 0..TILE_SIZE {
        let mut delta_c = delta_c_row;

        for _px in 0..TILE_SIZE {
            let result = compute_pixel_perturbation_hdr_bla(
                &orbit,
                &bla_table,
                delta_c,
                MAX_ITERATIONS,
                TEST_TAU_SQ,
            );
            results.push(result);

            delta_c.re = delta_c.re.add(&delta_step.re);
        }

        delta_c_row.im = delta_c_row.im.add(&delta_step.im);

        // Progress indicator
        if (py + 1) % 8 == 0 {
            println!("  Row {}/{} complete", py + 1, TILE_SIZE);
        }
    }

    // Analyze results
    let escaped_count = results.iter().filter(|r| r.escaped).count();
    let glitched_count = results.iter().filter(|r| r.glitched).count();
    let in_set_count = results.iter().filter(|r| !r.escaped).count();

    println!();
    println!("=== RESULTS ===");
    println!("Total pixels: {}", results.len());
    println!("Escaped: {}", escaped_count);
    println!("In set (not escaped): {}", in_set_count);
    println!("Glitched: {}", glitched_count);

    // ALL black = bug! (center at this location should have colors)
    if in_set_count == results.len() {
        println!();
        println!("*** BUG REPRODUCED: ALL pixels marked as 'in set' (all black) ***");
    }
}

/// Same test WITHOUT BLA to compare
#[test]
#[ignore]
fn deep_zoom_full_tile_without_bla() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);
    let tile_x = 384u32;
    let tile_y = 192u32;

    println!("=== DEEP ZOOM FULL TILE (NO BLA) ===");

    let orbit = create_production_reference_orbit();
    println!(
        "Orbit: {} points, escaped_at={:?}",
        orbit.orbit.len(),
        orbit.escaped_at
    );

    let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
    let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);

    // JSON round-trip
    let origin_json = serde_json::to_string(&delta_c_origin).expect("serialize origin");
    let step_json = serde_json::to_string(&delta_c_step).expect("serialize step");
    let delta_c_origin: (BigFloat, BigFloat) =
        serde_json::from_str(&origin_json).expect("deserialize origin");
    let delta_c_step: (BigFloat, BigFloat) =
        serde_json::from_str(&step_json).expect("deserialize step");

    let delta_origin = bigfloat_to_hdr_complex(&delta_c_origin);
    let delta_step = bigfloat_to_hdr_complex(&delta_c_step);

    println!("Computing {} pixels...", TILE_SIZE * TILE_SIZE);

    let mut results = Vec::with_capacity((TILE_SIZE * TILE_SIZE) as usize);
    let mut delta_c_row = delta_origin;

    for py in 0..TILE_SIZE {
        let mut delta_c = delta_c_row;

        for _px in 0..TILE_SIZE {
            let result =
                compute_pixel_perturbation(&orbit, delta_c, MAX_ITERATIONS, TEST_TAU_SQ);
            results.push(result);

            delta_c.re = delta_c.re.add(&delta_step.re);
        }

        delta_c_row.im = delta_c_row.im.add(&delta_step.im);

        if (py + 1) % 8 == 0 {
            println!("  Row {}/{}", py + 1, TILE_SIZE);
        }
    }

    let escaped_count = results.iter().filter(|r| r.escaped).count();
    let in_set_count = results.iter().filter(|r| !r.escaped).count();
    let glitched_count = results.iter().filter(|r| r.glitched).count();

    println!();
    println!("Total: {}, Escaped: {}, In set: {}, Glitched: {}",
        results.len(), escaped_count, in_set_count, glitched_count);

    if in_set_count == results.len() {
        println!("*** BUG: ALL pixels in set ***");
    }
}

/// Quick sanity test with small iteration count to verify test setup works
#[test]
fn deep_zoom_sanity_check() {
    let c_ref = parse_bigfloat_tuple(C_REF_JSON);
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let viewport = create_production_viewport();
    let delta_c_origin = calculate_delta_c_origin(384, 192, &viewport, (CANVAS_WIDTH, CANVAS_HEIGHT));
    let delta_origin = bigfloat_to_hdr_complex(&delta_c_origin);

    let result = compute_pixel_perturbation(&orbit, delta_origin, 1000, TEST_TAU_SQ);
    println!(
        "Sanity check (1000 iter): escaped={}, iterations={}, glitched={}",
        result.escaped, result.iterations, result.glitched
    );
}

/// Verify HDRFloat dc_max does NOT underflow at extreme zoom.
#[test]
fn dc_max_at_extreme_zoom() {
    let viewport = create_production_viewport();
    let dc_max = calculate_dc_max(&viewport);

    println!("=== DC_MAX AT EXTREME ZOOM (10^270) ===");
    println!("Viewport width: {}", VIEWPORT_WIDTH_STR);
    println!("Viewport height: {}", VIEWPORT_HEIGHT_STR);
    println!("dc_max = {:e} (log2 = {:.1})", dc_max.to_f64(), dc_max.log2());
    println!("dc_max.is_zero() = {}", dc_max.is_zero());

    assert!(
        !dc_max.is_zero(),
        "HDRFloat dc_max should NOT underflow at 10^270 zoom"
    );
    assert!(
        dc_max.log2() < -100.0,
        "dc_max log2 should be very negative, got {}",
        dc_max.log2()
    );
    println!("\n*** SUCCESS: HDRFloat dc_max does NOT underflow ***");
}

/// Test all 4 center tiles to identify which one(s) fail.
#[test]
#[ignore]
fn deep_zoom_all_center_tiles() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);

    // The 4 center tiles for 773x446 canvas (sorted by distance from center):
    // 1. (384, 192) - closest
    // 2. (384, 224)
    // 3. (352, 192)
    // 4. (352, 224)
    let center_tiles = [
        (384, 192),
        (384, 224),
        (352, 192),
        (352, 224),
    ];

    let dc_max = calculate_dc_max(&viewport);
    let orbit = create_production_reference_orbit();
    let bla_table = BlaTable::compute(&orbit, dc_max);

    println!("=== ALL 4 CENTER TILES ===");
    println!("Orbit: {} points", orbit.orbit.len());
    println!("BLA: {} entries, {} levels", bla_table.entries.len(), bla_table.num_levels);
    println!();

    for (tile_x, tile_y) in center_tiles {
        let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
        let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);

        let delta_origin = bigfloat_to_hdr_complex(&delta_c_origin);
        let _delta_step = bigfloat_to_hdr_complex(&delta_c_step);

        // Just compute first pixel for quick check
        let result = compute_pixel_perturbation_hdr_bla(
            &orbit,
            &bla_table,
            delta_origin,
            MAX_ITERATIONS,
            TEST_TAU_SQ,
        );

        println!(
            "Tile ({}, {}): escaped={}, iter={}, glitched={}",
            tile_x, tile_y, result.escaped, result.iterations, result.glitched
        );
    }
}
