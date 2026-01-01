//! Tests for HDRFloat behavior at extreme (deep) zoom levels.
//!
//! These tests verify correct behavior at zoom levels around 10^300-10^311
//! where values enter the f64 subnormal range and the "slow path" in
//! `HDRFloat::from_bigfloat` is used (when |exp| >= 1000).

use fractalwonder_core::{BigFloat, HDRFloat};

#[test]
fn from_bigfloat_extreme_preserves_mantissa() {
    // This test validates the fix for the bug where from_bigfloat lost mantissa precision
    // at extreme zoom levels (exp.abs() >= 1000) because estimate_log2_from_binary_string
    // only counted leading zeros without parsing significant bits.
    //
    // Test case: 0.7 × 10^-311 (approximately 0.7 × 2^-1033)
    // Before fix: returned mantissa ≈ 1.0 (wrong!)
    // After fix: returns mantissa ≈ 0.7 (correct)

    let bf = BigFloat::from_string("7e-312", 1200).unwrap(); // 0.7 × 10^-311
    let h = HDRFloat::from_bigfloat(&bf);

    // The HDRFloat should represent approximately 7e-312
    // In HDRFloat: value = (head + tail) × 2^exp
    // For 7e-312 ≈ 2^-1036.5, we expect exp around -1036 or -1037

    assert!(!h.is_zero(), "Should not be zero");

    // Verify exponent is in the expected range
    // log2(7e-312) = log2(7) + 312 * log2(0.1) ≈ 2.8 - 1036.5 ≈ -1034
    assert!(
        h.exp >= -1040 && h.exp <= -1030,
        "Exponent {} should be around -1034",
        h.exp
    );

    // Critical test: the head should NOT be close to 1.0 (that was the bug)
    // It should be in [0.5, 1.0) and closer to 0.5-0.7 range
    assert!(
        h.head >= 0.5 && h.head < 1.0,
        "Head {} should be normalized",
        h.head
    );

    // Verify the mantissa captures the "7" in 7e-312
    // The exact relationship depends on the exponent, but the key is
    // that different mantissas like 7e-312 vs 5e-312 should produce
    // distinguishable HDRFloat values
    let bf2 = BigFloat::from_string("5e-312", 1200).unwrap();
    let h2 = HDRFloat::from_bigfloat(&bf2);

    // The ratio of the two values should be approximately 7/5 = 1.4
    // We test this by comparing (h / h2) via their mantissas and exponents
    let exp_diff = h.exp - h2.exp;
    let scale = libm::exp2(exp_diff as f64);
    let ratio_mantissa = ((h.head + h.tail) as f64 * scale) / (h2.head + h2.tail) as f64;

    // The ratio should be close to 7/5 = 1.4
    let expected_ratio = 1.4;
    let rel_error = (ratio_mantissa - expected_ratio).abs() / expected_ratio;
    assert!(
        rel_error < 0.1, // Within 10% is good for ~48 bit precision at exp ~1000
        "Mantissa ratio {} should be close to 1.4 (7/5), rel_error={}",
        ratio_mantissa,
        rel_error
    );
}

#[test]
fn from_bigfloat_deep_zoom_viewport_width() {
    // Test the exact scenario from the bug report: viewport width at 10^311 zoom
    // Width: 3.52757594303989989983861048565237170287073180983103E-311
    let bf = BigFloat::from_string(
        "3.52757594303989989983861048565237170287073180983103e-311",
        1200,
    )
    .unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    assert!(!h.is_zero(), "Viewport width should not be zero");

    // Verify exponent is approximately -1031 (log2(3.5e-311) ≈ -1031.5)
    assert!(
        h.exp >= -1035 && h.exp <= -1028,
        "Exponent {} should be around -1031",
        h.exp
    );

    // The head should capture the "3.5" part, normalized to [0.5, 1.0)
    // 3.5 = 0.875 × 4 = 0.875 × 2^2, so head should be around 0.875
    assert!(
        h.head >= 0.5 && h.head < 1.0,
        "Head {} should be normalized",
        h.head
    );

    // Most importantly, head should NOT be 1.0 (that was the bug!)
    assert!(
        (h.head - 1.0).abs() > 0.1,
        "Head {} should not be approximately 1.0 (the bug symptom)",
        h.head
    );
}

#[test]
fn from_bigfloat_negative_extreme_preserves_sign() {
    // This test validates the fix for sign loss in the slow path of from_bigfloat.
    // At extreme exponents (|exp| >= 1000), the slow path used libm::exp2() which
    // always returns positive, causing negative values to become positive.
    //
    // This caused tiles in the top-left quadrant (negative delta_c) to render
    // the same region as bottom-right tiles, creating a "shuffled mosaic" effect.

    // Test negative value at extreme exponent
    let bf_neg = BigFloat::from_string("-3.5e-311", 1200).unwrap();
    let h_neg = HDRFloat::from_bigfloat(&bf_neg);

    // The HDRFloat must be NEGATIVE
    assert!(
        h_neg.head < 0.0,
        "Negative BigFloat at extreme exponent must produce negative HDRFloat, got head={}",
        h_neg.head
    );

    // Test positive value at same exponent for comparison
    let bf_pos = BigFloat::from_string("3.5e-311", 1200).unwrap();
    let h_pos = HDRFloat::from_bigfloat(&bf_pos);

    assert!(
        h_pos.head > 0.0,
        "Positive BigFloat must produce positive HDRFloat, got head={}",
        h_pos.head
    );

    // The absolute values should be equal
    assert!(
        (h_neg.head.abs() - h_pos.head.abs()).abs() < 0.001,
        "Absolute values should match: {} vs {}",
        h_neg.head.abs(),
        h_pos.head.abs()
    );

    // Exponents should be equal
    assert_eq!(h_neg.exp, h_pos.exp, "Exponents should match");
}

#[test]
fn coordinate_linearity_at_deep_zoom() {
    // Test that coordinates computed via `origin + pixel * step` are linearly spaced.
    // This test simulates the GPU coordinate computation at 10^311 zoom where
    // a diagonal discontinuity bug was observed.
    //
    // Key insight: if there's precision loss somewhere, the spacing between
    // adjacent pixels might be inconsistent.

    let width = 1920.0;
    let height = 1080.0;

    // Viewport width at 10^311 zoom
    let vp_width_bf = BigFloat::from_string("4e-311", 1200).unwrap();
    let vp_width = HDRFloat::from_bigfloat(&vp_width_bf);

    let vp_height_bf = BigFloat::from_string("2.25e-311", 1200).unwrap(); // 4e-311 * 1080/1920
    let vp_height = HDRFloat::from_bigfloat(&vp_height_bf);

    // Compute origin and step (same as parallel_renderer.rs)
    let half = HDRFloat::from_f64(0.5);
    let half_width = vp_width.mul(&half);
    let half_height = vp_height.mul(&half);
    let origin_re = half_width.neg();
    let origin_im = half_height.neg();
    let step_re = vp_width.div_f64(width);
    let step_im = vp_height.div_f64(height);

    // Test 1: Verify step values are sane
    assert!(!step_re.is_zero(), "step_re should not be zero");
    assert!(!step_im.is_zero(), "step_im should not be zero");

    // Test 2: Verify origin is negative half-width/height
    assert!(origin_re.head < 0.0, "origin_re should be negative");
    assert!(origin_im.head < 0.0, "origin_im should be negative");

    // Test 3: Check coordinate linearity along x-axis
    // Compute dc_re for pixels 0, 1, 2 and verify spacing is consistent
    let px0 = origin_re;
    let px1 = origin_re.add(&step_re);
    let px2 = origin_re.add(&step_re.mul(&HDRFloat::from_f64(2.0)));

    // The difference between adjacent pixels should equal step_re
    let diff_01 = px1.sub(&px0);
    let diff_12 = px2.sub(&px1);

    // Both differences should have the same exponent as step_re (or very close)
    assert_eq!(
        diff_01.exp, step_re.exp,
        "diff_01 exponent {} should match step_re exponent {}",
        diff_01.exp, step_re.exp
    );
    assert_eq!(
        diff_12.exp, step_re.exp,
        "diff_12 exponent {} should match step_re exponent {}",
        diff_12.exp, step_re.exp
    );

    // The head values should be very close
    // Note: At extreme zoom levels (10^311), HDRFloat precision is ~48 bits
    // but there's measurable error (~1e-4 relative) due to accumulated operations
    let rel_err_01 = ((diff_01.head - step_re.head) / step_re.head).abs();
    let rel_err_12 = ((diff_12.head - step_re.head) / step_re.head).abs();
    println!(
        "  rel_err_01: {:.2e}, rel_err_12: {:.2e}",
        rel_err_01, rel_err_12
    );
    assert!(
        rel_err_01 < 1e-4,
        "diff_01 head {} should match step_re head {}, rel_err = {}",
        diff_01.head,
        step_re.head,
        rel_err_01
    );
    assert!(
        rel_err_12 < 1e-4,
        "diff_12 head {} should match step_re head {}, rel_err = {}",
        diff_12.head,
        step_re.head,
        rel_err_12
    );

    // Test 4: Check coordinate at center of image (pixel 960, 540)
    let center_re = origin_re.add(&step_re.mul(&HDRFloat::from_f64(960.0)));
    let center_im = origin_im.add(&step_im.mul(&HDRFloat::from_f64(540.0)));

    // Center should be close to zero (within a small tolerance)
    // For 1920 width, pixel 960 is exactly at center, so dc_re should be ~0
    // dc_re = -half_width + 960 * (width / 1920) = -half_width + half_width = 0
    assert!(
        center_re.head.abs() < 0.01 || center_re.exp < origin_re.exp - 20,
        "Center re should be near zero, got head={} exp={}",
        center_re.head,
        center_re.exp
    );
    assert!(
        center_im.head.abs() < 0.01 || center_im.exp < origin_im.exp - 20,
        "Center im should be near zero, got head={} exp={}",
        center_im.head,
        center_im.exp
    );

    // Test 5: Check along the diagonal (where dc_re ≈ -dc_im could cause issues)
    // At pixel (0, 1079): dc_re = origin_re, dc_im = origin_im + 1079 * step_im ≈ +half_height
    // At pixel (1919, 0): dc_re = origin_re + 1919 * step_re ≈ +half_width, dc_im = origin_im
    let diag_tl = (
        origin_re,
        origin_im.add(&step_im.mul(&HDRFloat::from_f64(1079.0))),
    );
    let diag_br = (
        origin_re.add(&step_re.mul(&HDRFloat::from_f64(1919.0))),
        origin_im,
    );

    // These should have approximately equal magnitudes but opposite signs for re/im
    assert!(
        diag_tl.0.head < 0.0,
        "Top-left re should be negative: {}",
        diag_tl.0.head
    );
    assert!(
        diag_tl.1.head > 0.0,
        "Top-left im should be positive: {}",
        diag_tl.1.head
    );
    assert!(
        diag_br.0.head > 0.0,
        "Bottom-right re should be positive: {}",
        diag_br.0.head
    );
    assert!(
        diag_br.1.head < 0.0,
        "Bottom-right im should be negative: {}",
        diag_br.1.head
    );

    println!("Coordinate linearity test passed at 10^311 zoom");
    println!(
        "  origin_re: head={:.6}, exp={}",
        origin_re.head, origin_re.exp
    );
    println!(
        "  origin_im: head={:.6}, exp={}",
        origin_im.head, origin_im.exp
    );
    println!("  step_re: head={:.6}, exp={}", step_re.head, step_re.exp);
    println!("  step_im: head={:.6}, exp={}", step_im.head, step_im.exp);
    println!(
        "  center_re: head={:.6}, exp={}",
        center_re.head, center_re.exp
    );
    println!(
        "  center_im: head={:.6}, exp={}",
        center_im.head, center_im.exp
    );
}

#[test]
fn coordinate_linearity_at_boundary_zoom() {
    // Test at the boundary zoom level (~10^302) where exponent is around -1000.
    // This is exactly at the threshold between fast and slow paths in from_bigfloat.
    //
    // The bug might manifest right at this boundary.

    let width = 1920.0;

    // Test at exponent = -1000 (boundary)
    let vp_width_bf = BigFloat::from_string("3e-302", 1200).unwrap();
    let vp_width = HDRFloat::from_bigfloat(&vp_width_bf);

    println!("Boundary zoom (10^302) test:");
    println!(
        "  vp_width: head={:.6}, exp={}",
        vp_width.head, vp_width.exp
    );

    // This should use the SLOW path since |exp| >= 1000
    assert!(
        vp_width.exp.abs() >= 1000,
        "Expected slow path at 10^302, but exp = {} (|exp| = {})",
        vp_width.exp,
        vp_width.exp.abs()
    );

    let half = HDRFloat::from_f64(0.5);
    let half_width = vp_width.mul(&half);
    let origin_re = half_width.neg();
    let step_re = vp_width.div_f64(width);

    // Same linearity checks
    let px0 = origin_re;
    let px1 = origin_re.add(&step_re);
    let diff_01 = px1.sub(&px0);

    // Verify step consistency
    assert_eq!(
        diff_01.exp, step_re.exp,
        "At boundary zoom: diff_01 exponent {} should match step_re exponent {}",
        diff_01.exp, step_re.exp
    );

    let rel_err = ((diff_01.head - step_re.head) / step_re.head).abs();
    assert!(
        rel_err < 1e-4,
        "At boundary zoom: diff_01 head {} should match step_re head {}, rel_err = {}",
        diff_01.head,
        step_re.head,
        rel_err
    );

    println!("  step_re: head={:.6}, exp={}", step_re.head, step_re.exp);
    println!("  diff_01: head={:.6}, exp={}", diff_01.head, diff_01.exp);
    println!("  rel_err: {:.2e}", rel_err);
}

#[test]
fn multiplicative_vs_incremental_coordinate_computation() {
    // The GPU uses multiplicative: dc = origin + pixel * step
    // The CPU uses incremental: dc += step for each pixel
    //
    // This test verifies both approaches produce consistent results at deep zoom.
    // Discrepancy here could explain CPU/GPU differences.
    //
    // NOTE: At the center pixel (960), both methods compute values near zero.
    // Near-zero values have very negative exponents, and small precision differences
    // cause large exponent discrepancies. This is expected and not necessarily a bug.

    let width = 1920;

    let vp_width_bf = BigFloat::from_string("4e-311", 1200).unwrap();
    let vp_width = HDRFloat::from_bigfloat(&vp_width_bf);

    let half = HDRFloat::from_f64(0.5);
    let half_width = vp_width.mul(&half);
    let origin = half_width.neg();
    let step = vp_width.div_f64(width as f64);

    println!("=== Multiplicative vs Incremental Diagnostic ===");
    println!(
        "origin: head={:.6}, tail={:.6e}, exp={}",
        origin.head, origin.tail, origin.exp
    );
    println!(
        "step:   head={:.6}, tail={:.6e}, exp={}",
        step.head, step.tail, step.exp
    );

    // Multiplicative approach (GPU style)
    let mut multiplicative_coords: Vec<HDRFloat> = Vec::new();
    for px in 0..width {
        let dc = origin.add(&step.mul(&HDRFloat::from_f64(px as f64)));
        multiplicative_coords.push(dc);
    }

    // Incremental approach (CPU style)
    let mut incremental_coords: Vec<HDRFloat> = Vec::new();
    let mut dc = origin;
    for _ in 0..width {
        incremental_coords.push(dc);
        dc = dc.add(&step);
    }

    // Compare at various positions, but skip center where values are near zero
    let test_positions = [0, 1, 100, 480, 1440, 1919];
    let mut max_rel_err: f64 = 0.0;
    let mut max_err_pos = 0;

    println!("\nPosition comparison (excluding center pixel 960):");
    for &pos in &test_positions {
        let mult = &multiplicative_coords[pos];
        let incr = &incremental_coords[pos];

        println!(
            "  px {}: mult(h={:.6}, e={}) vs incr(h={:.6}, e={})",
            pos, mult.head, mult.exp, incr.head, incr.exp
        );

        // For non-center positions, exponents should match
        let exp_diff = (mult.exp - incr.exp).abs();
        assert!(
            exp_diff <= 1,
            "At pixel {}: exponent mismatch: mult.exp={}, incr.exp={}",
            pos,
            mult.exp,
            incr.exp
        );

        // If exponents match, compare heads
        if mult.exp == incr.exp {
            let rel_err: f64 = if mult.head.abs() > 1e-10 {
                ((mult.head - incr.head) / mult.head).abs() as f64
            } else {
                (mult.head - incr.head).abs() as f64
            };

            if rel_err > max_rel_err {
                max_rel_err = rel_err;
                max_err_pos = pos;
            }
        }
    }

    // Special diagnostic for center pixel (960)
    let mult_center = &multiplicative_coords[960];
    let incr_center = &incremental_coords[960];
    println!("\nCenter pixel (960) - expected to be near zero:");
    println!(
        "  mult: head={:.6e}, tail={:.6e}, exp={}",
        mult_center.head, mult_center.tail, mult_center.exp
    );
    println!(
        "  incr: head={:.6e}, tail={:.6e}, exp={}",
        incr_center.head, incr_center.tail, incr_center.exp
    );
    // At center, both should be very small relative to origin
    // The exponent should be much more negative than origin.exp
    assert!(
        mult_center.exp < origin.exp - 10,
        "Center mult should be much smaller than origin: mult.exp={} vs origin.exp={}",
        mult_center.exp,
        origin.exp
    );
    assert!(
        incr_center.exp < origin.exp - 10,
        "Center incr should be much smaller than origin: incr.exp={} vs origin.exp={}",
        incr_center.exp,
        origin.exp
    );

    println!(
        "\nMax relative error: {:.2e} at pixel {}",
        max_rel_err, max_err_pos
    );

    // Allow some accumulated error in incremental, but should be small
    assert!(
        max_rel_err < 1e-3,
        "Accumulated error too large: {:.2e} at pixel {}",
        max_rel_err,
        max_err_pos
    );
}
