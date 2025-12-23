use super::helpers::TEST_TAU_SQ;
use crate::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{
    BigFloat, BigFloatComplex, ComplexDelta, F64Complex, HDRComplex, HDRFloat,
};

#[test]
fn perturbation_with_bigfloat_deltas_no_underflow() {
    // At 10^500 zoom, f64 deltas would underflow to zero
    // This test verifies BigFloat deltas preserve the value

    let precision = 2048; // Enough for 10^500

    // Reference at origin (simple, in set)
    let c_ref = (BigFloat::zero(precision), BigFloat::zero(precision));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Delta at 10^-500 scale - would be 0.0 in f64
    let delta_c = (
        BigFloat::from_string("1e-500", precision).unwrap(),
        BigFloat::from_string("1e-500", precision).unwrap(),
    );

    // This should NOT underflow - delta_c should remain non-zero
    let log2_delta = delta_c.0.log2_approx();
    assert!(
        log2_delta > -2000.0,
        "Delta should not underflow: log2 = {}",
        log2_delta
    );
    assert!(
        log2_delta < -1600.0,
        "Delta should be around 10^-500: log2 = {}",
        log2_delta
    );

    // Compute pixel - should complete without panic
    let result = compute_pixel_perturbation(
        &orbit,
        BigFloatComplex::new(delta_c.0.clone(), delta_c.1.clone()),
        100,
        TEST_TAU_SQ,
    );

    // Point near origin with tiny offset should be in set
    assert!(!result.escaped, "Point near origin should be in set");
    assert_eq!(result.iterations, 100);
}

#[test]
fn bigfloat_matches_f64_for_shallow_zoom() {
    // At shallow zoom where f64 suffices, both versions should produce identical results
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Test multiple delta values within f64 range
    let test_deltas = [(0.01, 0.01), (-0.005, 0.002), (0.1, -0.05), (0.0, 0.001)];

    for (dx, dy) in test_deltas {
        // f64 version
        let f64_result =
            compute_pixel_perturbation(&orbit, F64Complex::from_f64_pair(dx, dy), 500, TEST_TAU_SQ);

        // BigFloat version
        let bigfloat_delta_re = BigFloat::with_precision(dx, 128);
        let bigfloat_delta_im = BigFloat::with_precision(dy, 128);
        let bigfloat_result = compute_pixel_perturbation(
            &orbit,
            BigFloatComplex::new(bigfloat_delta_re, bigfloat_delta_im),
            500,
            TEST_TAU_SQ,
        );

        assert_eq!(
            f64_result.escaped, bigfloat_result.escaped,
            "Escape status should match for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            f64_result.iterations, bigfloat_result.iterations,
            "Iteration count should match for delta ({}, {})",
            dx, dy
        );
    }
}

#[test]
fn bigfloat_handles_extreme_zoom_without_artifacts() {
    // At 10^1000 zoom, verify computation completes and produces sensible results
    let precision = 4096; // ~1200 decimal digits

    // Reference at a point known to be in the set
    let c_ref = (
        BigFloat::from_string("-0.5", precision).unwrap(),
        BigFloat::zero(precision),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Tiny delta - point should still be in set (near reference)
    let delta_c_re = BigFloat::from_string("1e-1000", precision).unwrap();
    let delta_c_im = BigFloat::from_string("1e-1000", precision).unwrap();

    let result = compute_pixel_perturbation(
        &orbit,
        BigFloatComplex::new(delta_c_re.clone(), delta_c_im.clone()),
        1000,
        TEST_TAU_SQ,
    );

    // Nearby point should have similar behavior to reference
    assert!(
        !result.escaped,
        "Point very close to reference should be in set"
    );
    assert_eq!(result.iterations, 1000, "Should reach max iterations");

    // Verify delta didn't underflow (would cause all points to behave identically)
    let log2_delta = delta_c_re.log2_approx();
    assert!(log2_delta.is_finite(), "Delta log2 should be finite");
    assert!(
        log2_delta < -3000.0,
        "Delta should be extremely small: {}",
        log2_delta
    );
}

#[test]
fn hdr_matches_f64_at_shallow_zoom() {
    // Reference in set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Test multiple delta values within f64 range
    let test_deltas = [(0.01, 0.01), (-0.005, 0.002), (0.1, -0.05)];

    for (dx, dy) in test_deltas {
        // f64 version
        let f64_result =
            compute_pixel_perturbation(&orbit, F64Complex::from_f64_pair(dx, dy), 500, TEST_TAU_SQ);

        // HDRFloat version
        let delta_c = HDRComplex {
            re: HDRFloat::from_f64(dx),
            im: HDRFloat::from_f64(dy),
        };
        let hdr_result = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);

        assert_eq!(
            f64_result.escaped, hdr_result.escaped,
            "Escape mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            f64_result.iterations, hdr_result.iterations,
            "Iteration mismatch for delta ({}, {})",
            dx, dy
        );
    }
}

#[test]
fn hdr_matches_bigfloat_at_deep_zoom() {
    let precision = 2048;

    // Reference at origin
    let c_ref = (BigFloat::zero(precision), BigFloat::zero(precision));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Delta at 10^-500 scale
    let delta_bf = (
        BigFloat::from_string("1e-500", precision).unwrap(),
        BigFloat::from_string("2e-500", precision).unwrap(),
    );

    // Convert to HDRFloat
    let delta_hdr = HDRComplex {
        re: HDRFloat::from_bigfloat(&delta_bf.0),
        im: HDRFloat::from_bigfloat(&delta_bf.1),
    };

    // BigFloat version (reference implementation)
    let bf_result = compute_pixel_perturbation(
        &orbit,
        BigFloatComplex::new(delta_bf.0.clone(), delta_bf.1.clone()),
        500,
        TEST_TAU_SQ,
    );

    // HDRFloat version (optimized)
    let hdr_result = compute_pixel_perturbation(&orbit, delta_hdr, 500, TEST_TAU_SQ);

    assert_eq!(
        bf_result.escaped, hdr_result.escaped,
        "Escape status should match at deep zoom"
    );
    assert_eq!(
        bf_result.iterations, hdr_result.iterations,
        "Iteration count should match at deep zoom"
    );
}
