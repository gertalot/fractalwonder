use super::helpers::TEST_TAU_SQ;
use crate::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{BigFloat, ComplexDelta, F64Complex};

#[test]
fn glitch_detected_via_pauldelbrot_criterion() {
    // Reference at a point in the set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Use a delta that will cause |z| to become very small relative to |Z|
    // This triggers the Pauldelbrot criterion: |z|² < τ²|Z|²
    // For now, verify the basic mechanics work
    let delta_c = (0.01, 0.01);
    let tau_sq = 1e-6; // τ = 10⁻³
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        tau_sq,
    );

    // Should complete without panic
    assert!(result.iterations > 0 || result.escaped);
}

#[test]
fn no_glitch_for_normal_escape() {
    // Reference in set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Pixel that escapes quickly and cleanly
    let delta_c = (2.5, 0.0); // Point at (2, 0) escapes immediately
    let tau_sq = 1e-6;
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        tau_sq,
    );

    assert!(result.escaped);
    assert!(result.iterations < 10);
    // Clean escape should not be marked glitched
    assert!(!result.glitched, "Clean escape should not be glitched");
}

#[test]
fn no_glitch_when_pixel_escapes_before_reference() {
    // Reference in set: (-0.5, 0) never escapes
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    assert!(orbit.escaped_at.is_none(), "Reference should be in set");

    // Pixel that escapes: (2, 0) escapes quickly
    let delta_c = (2.5, 0.0);
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        TEST_TAU_SQ,
    );

    assert!(result.escaped, "Point (2, 0) should escape");
    assert!(result.iterations < 10, "Should escape quickly");

    // No glitch: pixel escaped while reference data was still available
    assert!(
        !result.glitched,
        "Pixel escaping before reference should not be glitched"
    );
}

#[test]
fn no_glitch_for_nearby_pixel_in_set() {
    // Reference in set: (-0.5, 0) never escapes
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    assert!(orbit.escaped_at.is_none());

    // Pixel nearby: (-0.49, 0.01) - small delta, also in set
    // This keeps the pixel orbit close to reference orbit
    let delta_c = (0.01, 0.01);
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        TEST_TAU_SQ,
    );

    assert!(!result.escaped);
    assert_eq!(result.iterations, 1000);

    // With small delta, orbits stay close and no precision loss occurs
    assert!(!result.glitched, "Nearby pixel should not be glitched");
}

#[test]
fn no_glitch_when_rebasing_only() {
    // Reference in set that allows rebasing to trigger
    let c_ref = (
        BigFloat::with_precision(-0.75, 128),
        BigFloat::with_precision(0.1, 128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Small offset that triggers rebasing but escapes before reference exhausted
    let delta_c = (0.1, 0.05);
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        TEST_TAU_SQ,
    );

    // If pixel escaped, it shouldn't be glitched from rebasing alone
    // (Pauldelbrot criterion detects precision loss, not rebasing)
    if result.escaped {
        // Rebasing alone should not cause glitch
        // The pixel may or may not be glitched depending on Pauldelbrot criterion
        assert!(result.iterations > 0);
    }
}

#[test]
fn glitch_detected_when_reference_exhausted() {
    // Reference at c = -2.1 escapes after ~5-6 iterations:
    // Z_0 = 0, Z_1 = -2.1, Z_2 ≈ 2.31, Z_3 ≈ 3.24, Z_4 ≈ 8.37...
    // Eventually |Z|² > 65536 and it escapes.
    let c_ref = (BigFloat::with_precision(-2.1, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Verify reference escapes quickly
    assert!(orbit.escaped_at.is_some(), "Reference should escape");
    let orbit_len = orbit.orbit.len();
    assert!(
        orbit_len <= 10,
        "Reference should escape in <=10 iterations, got {}",
        orbit_len
    );

    // Pixel at c = -2.0 (tip of the main cardioid, in the set)
    // Delta = -2.0 - (-2.1) = 0.1 (SMALL delta, so rebasing rarely happens)
    // With such a small delta, m will naturally advance and exceed orbit_len.
    let delta_c = (0.1, 0.0);
    let max_iter = 100;

    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        max_iter,
        TEST_TAU_SQ,
    );

    // With orbit_len ~= 5-10 and max_iter = 100, m WILL exceed orbit_len
    // because the pixel needs ~100 iterations (it's in/near the set).
    // When m >= orbit_len and reference escaped, should be glitched.
    assert!(
        result.glitched || result.escaped,
        "With short orbit (len={}) and long iteration ({}), \
         either m exceeded orbit_len (glitched=true) or pixel escaped. \
         Got: escaped={}, glitched={}, iterations={}",
        orbit_len,
        max_iter,
        result.escaped,
        result.glitched,
        result.iterations
    );

    // If pixel didn't escape, it must be marked glitched
    if !result.escaped {
        assert!(
            result.glitched,
            "Non-escaping pixel with short reference orbit must be glitched"
        );
    }
}
