//! Reproduction test for the deep zoom center tile bug.
//!
//! At ~10^270 zoom with 10M iterations, center tiles fail to render with CPU while GPU works.
//! This test reproduces the EXACT conditions using EXACT production code paths and parameters.
//!
//! CRITICAL: This test uses the ACTUAL production functions from fractalwonder_core,
//! not copied/duplicated logic. Any changes to production code will automatically
//! be reflected in this test.
//!
//! URL decoded from: http://127.0.0.1:8080/fractalwonder/#v1:...
//! Canvas size: 773x446
//! Tile size at this zoom: 32x32 (DEEP_ZOOM_TILE_SIZE)

use crate::bla::BlaTable;
use crate::tile_render::{render_tile, TileRenderInput};
use crate::ReferenceOrbit;
use fractalwonder_core::{
    calculate_dc_max, calculate_render_max_iterations, is_bla_useful, BigFloat, ComputeData,
    Viewport, MANDELBROT_CONFIG,
};

// =============================================================================
// EXACT VALUES FROM DECODED URL (Canvas: 773x446, Tile size: 32)
// =============================================================================

/// Canvas dimensions from the actual failing render
const CANVAS_WIDTH: u32 = 773;
const CANVAS_HEIGHT: u32 = 446;

/// Tile size at deep zoom (from tiles.rs: DEEP_ZOOM_TILE_SIZE = 32)
const TILE_SIZE: u32 = 32;

/// Precision bits from the decoded URL
const PRECISION_BITS: usize = 1026;

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

// =============================================================================
// Helper Functions - Use ACTUAL production code paths
// =============================================================================

/// Create viewport from production values.
/// This is the EXACT viewport that causes the bug.
fn create_production_viewport() -> Viewport {
    let center: (BigFloat, BigFloat) =
        serde_json::from_str(C_REF_JSON).expect("Valid c_ref JSON");
    let width =
        BigFloat::from_string(VIEWPORT_WIDTH_STR, PRECISION_BITS).expect("Valid viewport width");
    let height =
        BigFloat::from_string(VIEWPORT_HEIGHT_STR, PRECISION_BITS).expect("Valid viewport height");

    Viewport {
        center,
        width,
        height,
    }
}

/// Calculate delta_c_origin for a tile EXACTLY as coordinator.rs does (lines 269-278).
/// This is the only function we replicate because it's internal to coordinator.
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

/// Simulate the EXACT JSON round-trip that happens in production for messages.
/// In production, messages go through: serialize -> send -> deserialize
/// This simulates that for any serializable type.
fn json_round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

/// Simulate the DOUBLE JSON encoding that happens for delta values in production.
/// Production flow:
/// 1. coordinator: delta_c_origin_json = serde_json::to_string(&delta_c_origin)
/// 2. MainToWorker message created with this JSON string
/// 3. Entire message serialized to JSON for worker messaging
/// 4. Worker deserializes message, gets back the JSON string
/// 5. Worker parses: serde_json::from_str(&delta_c_origin_json)
fn double_json_round_trip_bigfloat_tuple(value: &(BigFloat, BigFloat)) -> (BigFloat, BigFloat) {
    // Step 1: coordinator serializes delta to JSON string
    let inner_json = serde_json::to_string(value).expect("serialize delta");

    // Steps 2-4: The JSON string goes inside MainToWorker message, which is itself serialized
    // We simulate this by serializing the string as JSON (which escapes it), then deserializing
    let outer_json = serde_json::to_string(&inner_json).expect("serialize message");
    let recovered_inner_json: String =
        serde_json::from_str(&outer_json).expect("deserialize message");

    // Step 5: Worker parses the recovered JSON string
    serde_json::from_str(&recovered_inner_json).expect("deserialize delta")
}

// =============================================================================
// Production-Matching Tests
// =============================================================================

/// This test reproduces the EXACT failing condition from production.
/// It uses render_tile() with TileRenderInput - the EXACT production code path.
/// ALL parameters use ACTUAL production functions, not hardcoded values.
///
/// Run with: cargo test deep_zoom_full_tile_with_bla -- --ignored --nocapture
#[test]
#[ignore]
fn deep_zoom_full_tile_with_bla() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);

    // The first center tile (closest to canvas center): PixelRect(384, 192, 32, 32)
    let tile_x = 384u32;
    let tile_y = 192u32;

    println!("=== DEEP ZOOM FULL TILE TEST (PRODUCTION CODE PATH) ===");
    println!("Canvas: {}x{}", CANVAS_WIDTH, CANVAS_HEIGHT);
    println!("Tile: ({}, {}, {}, {})", tile_x, tile_y, TILE_SIZE, TILE_SIZE);
    println!("Viewport width: {}", VIEWPORT_WIDTH_STR);
    println!("Viewport height: {}", VIEWPORT_HEIGHT_STR);
    println!();

    // =========================================================================
    // Use ACTUAL production functions - no hardcoded values
    // =========================================================================

    // Calculate max_iterations using ACTUAL production function and config
    let max_iterations = calculate_render_max_iterations(&viewport, &MANDELBROT_CONFIG);
    println!(
        "max_iterations = {} (from MANDELBROT_CONFIG: multiplier={}, power={})",
        max_iterations, MANDELBROT_CONFIG.iteration_multiplier, MANDELBROT_CONFIG.iteration_power
    );

    // Get tau_sq from ACTUAL production config
    let tau_sq = MANDELBROT_CONFIG.tau_sq;
    println!("tau_sq = {:e} (from MANDELBROT_CONFIG)", tau_sq);

    // Get bla_enabled from ACTUAL production config
    let bla_enabled = MANDELBROT_CONFIG.bla_enabled;
    println!("bla_enabled = {} (from MANDELBROT_CONFIG)", bla_enabled);
    println!();

    // Calculate dc_max using ACTUAL production function
    let dc_max = calculate_dc_max(&viewport);
    println!(
        "dc_max = {:e} (log2 = {:.1}) [from calculate_dc_max]",
        dc_max.to_f64(),
        dc_max.log2()
    );

    // Simulate JSON round-trip for dc_max (production: coordinator -> worker message)
    let dc_max = json_round_trip(&dc_max);
    println!(
        "dc_max after JSON round-trip: {:e} (log2 = {:.1})",
        dc_max.to_f64(),
        dc_max.log2()
    );

    // Check bla_useful using ACTUAL production function
    let bla_useful = is_bla_useful(&dc_max);
    println!(
        "is_bla_useful = {} [from is_bla_useful(), threshold log2 < -80]",
        bla_useful
    );
    println!();

    // =========================================================================
    // Compute reference orbit (same as worker.rs:240)
    // =========================================================================

    println!("Computing reference orbit ({} iterations)...", max_iterations);
    let c_ref: (BigFloat, BigFloat) =
        serde_json::from_str(C_REF_JSON).expect("Valid c_ref JSON");
    let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);
    println!(
        "Orbit: {} points, escaped_at={:?}",
        orbit.orbit.len(),
        orbit.escaped_at
    );

    // =========================================================================
    // Simulate FULL JSON round-trip for orbit (ReferenceOrbitComplete -> StoreReferenceOrbit)
    // Production sends orbit TWICE through JSON: worker->main, main->worker
    // =========================================================================

    // First round-trip: ReferenceOrbitComplete message
    let c_ref_rt1 = json_round_trip(&orbit.c_ref);
    let orbit_data_rt1 = json_round_trip(&orbit.orbit);
    let derivative_rt1 = json_round_trip(&orbit.derivative);
    let escaped_at_rt1 = json_round_trip(&orbit.escaped_at);

    // Second round-trip: StoreReferenceOrbit message
    let c_ref_rt2 = json_round_trip(&c_ref_rt1);
    let orbit_data_rt2 = json_round_trip(&orbit_data_rt1);
    let derivative_rt2 = json_round_trip(&derivative_rt1);
    let escaped_at_rt2 = json_round_trip(&escaped_at_rt1);

    let orbit = ReferenceOrbit {
        c_ref: c_ref_rt2,
        orbit: orbit_data_rt2,
        derivative: derivative_rt2,
        escaped_at: escaped_at_rt2,
    };
    println!(
        "Orbit after 2x JSON round-trip: {} points, escaped_at={:?}",
        orbit.orbit.len(),
        orbit.escaped_at
    );
    println!();

    // =========================================================================
    // Build BLA table (worker.rs:279-309) - only if bla_enabled AND bla_useful
    // =========================================================================

    let bla_table = if bla_enabled && bla_useful {
        let table = BlaTable::compute(&orbit, dc_max);
        println!(
            "BLA table: {} entries, {} levels",
            table.entries.len(),
            table.num_levels
        );
        Some(table)
    } else {
        println!(
            "BLA table SKIPPED: bla_enabled={}, bla_useful={}",
            bla_enabled, bla_useful
        );
        None
    };
    println!();

    // =========================================================================
    // Calculate delta values with DOUBLE JSON encoding (production behavior)
    // =========================================================================

    let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
    let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);

    // Simulate DOUBLE JSON round-trip for deltas (nested in MainToWorker message)
    let delta_c_origin = double_json_round_trip_bigfloat_tuple(&delta_c_origin);
    let delta_c_step = double_json_round_trip_bigfloat_tuple(&delta_c_step);

    println!(
        "delta_c_origin log2: re={:.1}, im={:.1} (after double JSON encoding)",
        delta_c_origin.0.log2_approx(),
        delta_c_origin.1.log2_approx()
    );
    println!(
        "delta_c_step log2: re={:.1}, im={:.1} (after double JSON encoding)",
        delta_c_step.0.log2_approx(),
        delta_c_step.1.log2_approx()
    );
    println!();

    // =========================================================================
    // Create TileRenderInput EXACTLY as worker.rs:373-382 does
    // =========================================================================

    let input = TileRenderInput {
        delta_c_origin,
        delta_c_step,
        tile_width: TILE_SIZE,
        tile_height: TILE_SIZE,
        max_iterations,
        tau_sq,
        bla_enabled, // Production passes config value; render_tile handles bla_table=None
        force_hdr_float: false,
    };

    println!(
        "Calling render_tile() with {} x {} pixels, bla_enabled={}...",
        TILE_SIZE,
        TILE_SIZE,
        input.bla_enabled
    );

    // =========================================================================
    // Call render_tile() - the EXACT production code path (worker.rs:384)
    // =========================================================================

    let results = render_tile(&orbit, bla_table.as_ref(), &input);

    // Analyze results
    let escaped_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if m.escaped))
        .count();
    let glitched_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if m.glitched))
        .count();
    let in_set_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if !m.escaped))
        .count();

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

/// Same test WITHOUT BLA - uses render_tile() with bla_enabled=false
#[test]
#[ignore]
fn deep_zoom_full_tile_without_bla() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);
    let tile_x = 384u32;
    let tile_y = 192u32;

    println!("=== DEEP ZOOM FULL TILE (NO BLA, PRODUCTION CODE PATH) ===");

    // Use ACTUAL production functions
    let max_iterations = calculate_render_max_iterations(&viewport, &MANDELBROT_CONFIG);
    let tau_sq = MANDELBROT_CONFIG.tau_sq;

    println!("max_iterations = {} (from MANDELBROT_CONFIG)", max_iterations);
    println!("tau_sq = {:e} (from MANDELBROT_CONFIG)", tau_sq);

    let c_ref: (BigFloat, BigFloat) =
        serde_json::from_str(C_REF_JSON).expect("Valid c_ref JSON");
    let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);
    println!(
        "Orbit: {} points, escaped_at={:?}",
        orbit.orbit.len(),
        orbit.escaped_at
    );

    // Full JSON round-trip for orbit (2x as in production)
    let c_ref_rt = json_round_trip(&json_round_trip(&orbit.c_ref));
    let orbit_data_rt = json_round_trip(&json_round_trip(&orbit.orbit));
    let derivative_rt = json_round_trip(&json_round_trip(&orbit.derivative));
    let escaped_at_rt = json_round_trip(&json_round_trip(&orbit.escaped_at));

    let orbit = ReferenceOrbit {
        c_ref: c_ref_rt,
        orbit: orbit_data_rt,
        derivative: derivative_rt,
        escaped_at: escaped_at_rt,
    };

    let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
    let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);

    // Double JSON round-trip for deltas
    let delta_c_origin = double_json_round_trip_bigfloat_tuple(&delta_c_origin);
    let delta_c_step = double_json_round_trip_bigfloat_tuple(&delta_c_step);

    let input = TileRenderInput {
        delta_c_origin,
        delta_c_step,
        tile_width: TILE_SIZE,
        tile_height: TILE_SIZE,
        max_iterations,
        tau_sq,
        bla_enabled: false, // Key difference: no BLA
        force_hdr_float: false,
    };

    println!(
        "Calling render_tile() with {} x {} pixels, bla_enabled=false...",
        TILE_SIZE, TILE_SIZE
    );

    let results = render_tile(&orbit, None, &input);

    let escaped_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if m.escaped))
        .count();
    let in_set_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if !m.escaped))
        .count();
    let glitched_count = results
        .iter()
        .filter(|r| matches!(r, ComputeData::Mandelbrot(m) if m.glitched))
        .count();

    println!();
    println!(
        "Total: {}, Escaped: {}, In set: {}, Glitched: {}",
        results.len(),
        escaped_count,
        in_set_count,
        glitched_count
    );

    if in_set_count == results.len() {
        println!("*** BUG: ALL pixels in set ***");
    }
}

/// Quick sanity test with small iteration count to verify test setup works
#[test]
fn deep_zoom_sanity_check() {
    let c_ref: (BigFloat, BigFloat) =
        serde_json::from_str(C_REF_JSON).expect("Valid c_ref JSON");
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let viewport = create_production_viewport();
    let delta_c_origin = calculate_delta_c_origin(384, 192, &viewport, (CANVAS_WIDTH, CANVAS_HEIGHT));
    let delta_c_step = calculate_delta_c_step(&viewport, (CANVAS_WIDTH, CANVAS_HEIGHT));

    // Use render_tile with a 1x1 tile for single pixel test
    let input = TileRenderInput {
        delta_c_origin,
        delta_c_step,
        tile_width: 1,
        tile_height: 1,
        max_iterations: 1000,
        tau_sq: MANDELBROT_CONFIG.tau_sq,
        bla_enabled: false,
        force_hdr_float: false,
    };

    let results = render_tile(&orbit, None, &input);
    if let Some(ComputeData::Mandelbrot(result)) = results.first() {
        println!(
            "Sanity check (1000 iter): escaped={}, iterations={}, glitched={}",
            result.escaped, result.iterations, result.glitched
        );
    }
}

/// Verify HDRFloat dc_max does NOT underflow at extreme zoom.
/// Uses ACTUAL production function.
#[test]
fn dc_max_at_extreme_zoom() {
    let viewport = create_production_viewport();

    // Use ACTUAL production function
    let dc_max = calculate_dc_max(&viewport);

    println!("=== DC_MAX AT EXTREME ZOOM (10^270) ===");
    println!("Viewport width: {}", VIEWPORT_WIDTH_STR);
    println!("Viewport height: {}", VIEWPORT_HEIGHT_STR);
    println!(
        "dc_max = {:e} (log2 = {:.1}) [from calculate_dc_max]",
        dc_max.to_f64(),
        dc_max.log2()
    );
    println!("dc_max.is_zero() = {}", dc_max.is_zero());
    println!(
        "is_bla_useful = {} [from is_bla_useful]",
        is_bla_useful(&dc_max)
    );

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

/// Verify max_iterations calculation matches expected value.
#[test]
fn max_iterations_at_extreme_zoom() {
    let viewport = create_production_viewport();

    // Use ACTUAL production function
    let max_iterations = calculate_render_max_iterations(&viewport, &MANDELBROT_CONFIG);

    println!("=== MAX_ITERATIONS AT EXTREME ZOOM ===");
    println!(
        "MANDELBROT_CONFIG: multiplier={}, power={}",
        MANDELBROT_CONFIG.iteration_multiplier, MANDELBROT_CONFIG.iteration_power
    );
    println!("max_iterations = {}", max_iterations);

    // At 10^270 zoom, the formula gives a huge value that gets clamped to 10M
    assert_eq!(
        max_iterations, 10_000_000,
        "max_iterations should be capped at 10M"
    );
}

/// Test all 4 center tiles to identify which one(s) fail.
/// Uses render_tile() with 1x1 tiles for quick first-pixel check.
#[test]
#[ignore]
fn deep_zoom_all_center_tiles() {
    let viewport = create_production_viewport();
    let canvas_size = (CANVAS_WIDTH, CANVAS_HEIGHT);

    // Use ACTUAL production functions
    let max_iterations = calculate_render_max_iterations(&viewport, &MANDELBROT_CONFIG);
    let tau_sq = MANDELBROT_CONFIG.tau_sq;
    let bla_enabled = MANDELBROT_CONFIG.bla_enabled;

    let dc_max = calculate_dc_max(&viewport);
    let dc_max = json_round_trip(&dc_max);
    let bla_useful = is_bla_useful(&dc_max);

    // The 4 center tiles for 773x446 canvas (sorted by distance from center):
    let center_tiles = [(384, 192), (384, 224), (352, 192), (352, 224)];

    let c_ref: (BigFloat, BigFloat) =
        serde_json::from_str(C_REF_JSON).expect("Valid c_ref JSON");
    let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

    // Full JSON round-trip (2x)
    let c_ref_rt = json_round_trip(&json_round_trip(&orbit.c_ref));
    let orbit_data_rt = json_round_trip(&json_round_trip(&orbit.orbit));
    let derivative_rt = json_round_trip(&json_round_trip(&orbit.derivative));
    let escaped_at_rt = json_round_trip(&json_round_trip(&orbit.escaped_at));

    let orbit = ReferenceOrbit {
        c_ref: c_ref_rt,
        orbit: orbit_data_rt,
        derivative: derivative_rt,
        escaped_at: escaped_at_rt,
    };

    let bla_table = if bla_enabled && bla_useful {
        Some(BlaTable::compute(&orbit, dc_max))
    } else {
        None
    };

    println!("=== ALL 4 CENTER TILES (PRODUCTION CODE PATH) ===");
    println!("Orbit: {} points", orbit.orbit.len());
    if let Some(ref table) = bla_table {
        println!("BLA: {} entries, {} levels", table.entries.len(), table.num_levels);
    } else {
        println!("BLA: disabled");
    }
    println!();

    let delta_c_step = calculate_delta_c_step(&viewport, canvas_size);
    let delta_c_step = double_json_round_trip_bigfloat_tuple(&delta_c_step);

    for (tile_x, tile_y) in center_tiles {
        let delta_c_origin = calculate_delta_c_origin(tile_x, tile_y, &viewport, canvas_size);
        let delta_c_origin = double_json_round_trip_bigfloat_tuple(&delta_c_origin);

        let input = TileRenderInput {
            delta_c_origin,
            delta_c_step: delta_c_step.clone(),
            tile_width: 1,
            tile_height: 1,
            max_iterations,
            tau_sq,
            bla_enabled, // Production passes config value; render_tile handles bla_table=None
            force_hdr_float: false,
        };

        let results = render_tile(&orbit, bla_table.as_ref(), &input);
        if let Some(ComputeData::Mandelbrot(result)) = results.first() {
            println!(
                "Tile ({}, {}): escaped={}, iter={}, glitched={}",
                tile_x, tile_y, result.escaped, result.iterations, result.glitched
            );
        }
    }
}
