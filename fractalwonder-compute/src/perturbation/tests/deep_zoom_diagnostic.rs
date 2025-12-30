//! Diagnostic test for the deep zoom center tile bug.
//!
//! This test calls the EXACT production code path via render_tile().

use crate::bla::BlaTable;
use crate::tile_render::{render_tile, TileRenderInput};
use crate::ReferenceOrbit;
use fractalwonder_core::{BigFloat, ComputeData, HDRFloat, Viewport};
use std::time::Instant;

// =============================================================================
// EXACT VALUES FROM DECODED URL
// =============================================================================

const C_REF_JSON: &str = r#"[{"value":"0.0100010111100011010110010010000001111101110100000000110110011011110101010010000101011011011111011101110011000101111110011100011000011101001101100011011010111000101000101111100010100001010000011110100000000010100111011111010110011101001100100111010110100001000001000111100010101010001001110110001110111101111110000011101111011100011001011010110001010001101011010111100001101101101000010001100010011100010100010101000011100111011001011011100001110110110100110000000001101001100101011001110100011111111010001011101101110010010111000111010010001010110001110111001100010010011010100101110011011100011100010111011011001000000000100111110000000110110010000110100100000000110011000111011100000011101110011100100001000110110000010110110011101101010000000100111000110100011011110110001100111111100110001100001010110100110110000010110000110010100111011101110100010000100010000001110010000011100111001111110101101101101100101011111101110000011010001001010001011011011011110101101000001100001100100000111000000101011110101111111010011010101101001000000000101011101100100111101110110000010101111111111111111001001011000111111101000001000111001111010000000101110000111010100010001011100111001010010000010011110100110111011011010111000111000001001011110001010010100100011100001010011101110000010110110101111110110110011011011010100010111011100010110100001001100110101101111010100001111000100001111110111000001011100101110100110011111011110101000011011000001010110101110101111111111010001111111101111110010010000101001000101111010101001000110010101001001001000110111001100101101000010010010111010111111100011000101000000011001100010111011110111111111111101100001101010101101111110110111001011111000011010000100011100111000000001110010011111010101111101101011011100010111111010101101011111100001111011101111100111010000100100000101111101000100110001101000000100000010101011011101100001011011100101110000000011110110100101110110011010001100110101010101000010001110000010110101001001111101111110100001011001010111001000110001010010001001000010010111000101000101101100111110011101001000100000001010110110111010101000100000101100100101011100001111010101100101101111111101111110100111111100000011111001101001100000101011001011000010000000001111110001001000110110001011100000101001011000000010110100101010111000100110111100011100000010111101010001000111000110111010110011100000101110001100010100001","precision_bits":1026},{"value":"0.000000010111111010100101011100001111111101011000101100101000000011110111010001001010100010001000010011001101011111111000001001100001000110111000110000110111101111011001111101110110001110011001110011000110110110000111101110000111110001111010011100111001101010011011011110111110101110101111000010110001011011011000100001001001010010000010111101000101000101010001010100110111101001100000101101111100011100100010100000001100001110100011000101101111010111000000000010100001110110101011111010000001110010011010111110111111100111000101010111010111011101110010110011011011000000100110001111110111101011111101111010011000111000110001011010011001010000110111011010101011100001100110000111010010011001010110101010000000110010001000010011111100101010010000100100100111111011001001000001101000110010110001001010111011000011100101001110001111000110100101111001101101011101101010011010001111111111000000111100011010101100010001110011101010110000001100100100111000010010100001001110011110100001011010010110101110100010011111010011100101100011100101","precision_bits":1026}]"#;
const VIEWPORT_WIDTH_STR: &str = "3.68629585526668733757870313779318701180348758566795E-270";
const VIEWPORT_HEIGHT_STR: &str = "2.12689256332334093913116602106093685402570700118706E-270";
const CANVAS_WIDTH: u32 = 773;
const CANVAS_HEIGHT: u32 = 446;
const TILE_SIZE: u32 = 32;
const PRECISION_BITS: usize = 1026;

/// PRODUCTION max iterations
const MAX_ITERATIONS: u32 = 10_000_000;

/// PRODUCTION tau_sq default
const TAU_SQ: f64 = 1e-6;

/// PRODUCTION bla_enabled default
const BLA_ENABLED: bool = true;

/// PRODUCTION force_hdr_float default
const FORCE_HDR_FLOAT: bool = false;

// =============================================================================
// Helper functions - EXACT same as coordinator.rs
// =============================================================================

fn parse_bigfloat_tuple(json: &str) -> (BigFloat, BigFloat) {
    serde_json::from_str(json).expect("Valid BigFloat tuple JSON")
}

fn create_viewport() -> Viewport {
    let c_ref = parse_bigfloat_tuple(C_REF_JSON);
    let width = BigFloat::from_string(VIEWPORT_WIDTH_STR, PRECISION_BITS).unwrap();
    let height = BigFloat::from_string(VIEWPORT_HEIGHT_STR, PRECISION_BITS).unwrap();
    Viewport { center: c_ref, width, height }
}

fn calculate_dc_max(viewport: &Viewport) -> HDRFloat {
    let half_width = HDRFloat::from_bigfloat(&viewport.width).div_f64(2.0);
    let half_height = HDRFloat::from_bigfloat(&viewport.height).div_f64(2.0);
    half_width.square().add(&half_height.square()).sqrt()
}

/// EXACTLY as coordinator.rs lines 269-278
fn calculate_delta_c_origin(tile_x: u32, tile_y: u32, viewport: &Viewport) -> (BigFloat, BigFloat) {
    let precision = viewport.width.precision_bits();
    let norm_x = tile_x as f64 / CANVAS_WIDTH as f64 - 0.5;
    let norm_y = tile_y as f64 / CANVAS_HEIGHT as f64 - 0.5;
    let norm_x_bf = BigFloat::with_precision(norm_x, precision);
    let norm_y_bf = BigFloat::with_precision(norm_y, precision);
    (norm_x_bf.mul(&viewport.width), norm_y_bf.mul(&viewport.height))
}

/// EXACTLY as coordinator.rs lines 181-187
fn calculate_delta_c_step(viewport: &Viewport) -> (BigFloat, BigFloat) {
    let precision = viewport.width.precision_bits();
    let canvas_width_bf = BigFloat::with_precision(CANVAS_WIDTH as f64, precision);
    let canvas_height_bf = BigFloat::with_precision(CANVAS_HEIGHT as f64, precision);
    (viewport.width.div(&canvas_width_bf), viewport.height.div(&canvas_height_bf))
}

// =============================================================================
// DIAGNOSTIC TEST - Calls render_tile (exact production code)
// =============================================================================

#[test]
fn diagnose_deep_zoom_hang() {
    println!("=== DEEP ZOOM DIAGNOSTIC (via render_tile) ===");
    println!("MAX_ITERATIONS: {}", MAX_ITERATIONS);
    println!("TAU_SQ: {}", TAU_SQ);
    println!("BLA_ENABLED: {}", BLA_ENABLED);
    println!("FORCE_HDR_FLOAT: {}", FORCE_HDR_FLOAT);
    println!();

    let test_start = Instant::now();

    // Setup viewport
    let viewport = create_viewport();
    let c_ref = parse_bigfloat_tuple(C_REF_JSON);

    // Compute reference orbit
    println!("Computing reference orbit...");
    let orbit_start = Instant::now();
    let orbit = ReferenceOrbit::compute(&c_ref, MAX_ITERATIONS);
    println!("Orbit: {} points, escaped_at={:?} (took {:?})",
        orbit.orbit.len(), orbit.escaped_at, orbit_start.elapsed());

    // JSON round-trip for orbit - EXACTLY as production
    let orbit_json = serde_json::to_string(&orbit.orbit).expect("serialize orbit");
    let derivative_json = serde_json::to_string(&orbit.derivative).expect("serialize derivative");
    let orbit_data: Vec<(f64, f64)> = serde_json::from_str(&orbit_json).expect("deserialize orbit");
    let derivative_data: Vec<(f64, f64)> = serde_json::from_str(&derivative_json).expect("deserialize derivative");
    let orbit = ReferenceOrbit {
        c_ref: orbit.c_ref,
        orbit: orbit_data,
        derivative: derivative_data,
        escaped_at: orbit.escaped_at,
    };

    // Calculate dc_max with JSON round-trip
    let dc_max = calculate_dc_max(&viewport);
    let dc_max_json = serde_json::to_string(&dc_max).expect("serialize dc_max");
    let dc_max: HDRFloat = serde_json::from_str(&dc_max_json).expect("deserialize dc_max");
    println!("dc_max: {:e} (log2 = {:.1})", dc_max.to_f64(), dc_max.log2());

    // Build BLA table - EXACTLY as production
    println!("\nBuilding BLA table...");
    let bla_start = Instant::now();
    let bla_table = BlaTable::compute(&orbit, dc_max);
    println!("BLA table: {} entries, {} levels (took {:?})",
        bla_table.entries.len(), bla_table.num_levels, bla_start.elapsed());

    // Calculate delta values with JSON round-trip
    let tile_x = 384u32;
    let tile_y = 192u32;

    let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport);
    let delta_c_step = calculate_delta_c_step(&viewport);

    let origin_json = serde_json::to_string(&delta_c_origin).expect("serialize origin");
    let step_json = serde_json::to_string(&delta_c_step).expect("serialize step");
    let delta_c_origin: (BigFloat, BigFloat) = serde_json::from_str(&origin_json).expect("deserialize origin");
    let delta_c_step: (BigFloat, BigFloat) = serde_json::from_str(&step_json).expect("deserialize step");

    // Log dispatch info
    let delta_log2 = delta_c_origin.0.log2_approx().max(delta_c_origin.1.log2_approx());
    println!("\ndelta_log2 = {:.1} (threshold: -900.0)", delta_log2);
    if !FORCE_HDR_FLOAT && delta_log2 > -900.0 && delta_log2 < 900.0 {
        println!("Path: F64 (BLA not used in F64 path)");
    } else {
        println!("Path: HDRFloat (BLA_ENABLED={})", BLA_ENABLED);
    }

    // Build input - EXACTLY as worker.rs does
    let input = TileRenderInput {
        delta_c_origin,
        delta_c_step,
        tile_width: TILE_SIZE,
        tile_height: TILE_SIZE,
        max_iterations: MAX_ITERATIONS,
        tau_sq: TAU_SQ,
        bla_enabled: BLA_ENABLED,
        force_hdr_float: FORCE_HDR_FLOAT,
    };

    // Call render_tile - THE EXACT PRODUCTION CODE
    println!("\n=== COMPUTING TILE ({}, {}) - {} pixels ===", tile_x, tile_y, TILE_SIZE * TILE_SIZE);
    let compute_start = Instant::now();
    let data = render_tile(&orbit, Some(&bla_table), &input);
    let compute_elapsed = compute_start.elapsed();

    // Analyze results
    let mut escaped_count = 0;
    let mut glitched_count = 0;
    let mut in_set_count = 0;
    let mut total_iterations: u64 = 0;

    for item in &data {
        if let ComputeData::Mandelbrot(result) = item {
            if result.escaped {
                escaped_count += 1;
            } else {
                in_set_count += 1;
            }
            if result.glitched {
                glitched_count += 1;
            }
            total_iterations += result.iterations as u64;
        }
    }

    let avg_iterations = total_iterations as f64 / data.len() as f64;

    println!();
    println!("=== RESULTS ===");
    println!("Total pixels: {}", data.len());
    println!("Compute time: {:?}", compute_elapsed);
    println!("Time per pixel: {:.1}ms", compute_elapsed.as_millis() as f64 / data.len() as f64);
    println!();
    println!("Escaped: {}", escaped_count);
    println!("In set: {}", in_set_count);
    println!("Glitched: {}", glitched_count);
    println!("Avg iterations: {:.0}", avg_iterations);
    println!();
    println!("Total test time: {:?}", test_start.elapsed());
}
