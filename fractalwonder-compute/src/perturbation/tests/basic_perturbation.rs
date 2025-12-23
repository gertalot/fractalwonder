use super::helpers::{compute_direct, TEST_TAU_SQ};
use crate::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{BigFloat, ComplexDelta, F64Complex};

#[test]
fn perturbation_origin_in_set() {
    // Reference at (-0.5, 0), delta_c = (0.5, 0) gives point (0, 0) which is in set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(0.5, 0.0),
        1000,
        TEST_TAU_SQ,
    );

    assert!(!result.escaped);
    assert_eq!(result.iterations, 1000);
}

#[test]
fn perturbation_far_point_escapes() {
    // Reference at (-0.5, 0), delta_c = (2.5, 0) gives point (2, 0) which escapes
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(2.5, 0.0),
        1000,
        TEST_TAU_SQ,
    );

    assert!(result.escaped);
    assert!(result.iterations < 10);
}

#[test]
fn perturbation_matches_direct_for_nearby_point() {
    // Compare perturbation result with direct BigFloat computation
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Small delta: pixel at (-0.49, 0.01)
    let delta_c = (0.01, 0.01);
    let perturbation_result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        TEST_TAU_SQ,
    );

    // Direct computation at same point
    let pixel_c = (
        BigFloat::with_precision(-0.49, 128),
        BigFloat::with_precision(0.01, 128),
    );
    let direct_result = compute_direct(&pixel_c, 500);

    // Results should match (both escaped or both didn't, similar iteration count)
    assert_eq!(perturbation_result.escaped, direct_result.escaped);
    if perturbation_result.escaped {
        // Allow small difference due to floating point
        let diff = (perturbation_result.iterations as i32 - direct_result.iterations as i32).abs();
        assert!(diff <= 1, "Iteration difference too large: {}", diff);
    }
}

#[test]
fn perturbation_handles_rebasing() {
    // Use a reference point where rebasing will be triggered
    // Point on boundary has chaotic behavior
    let c_ref = (
        BigFloat::with_precision(-0.75, 128),
        BigFloat::with_precision(0.1, 128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Offset that should trigger rebasing
    let delta_c = (0.1, 0.05);
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        TEST_TAU_SQ,
    );

    // Should complete without panic
    assert!(result.iterations > 0);
}

#[test]
fn wrap_around_works_for_long_iterations() {
    // Reference with short orbit (escapes early)
    let c_ref = (BigFloat::with_precision(0.3, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    // Reference should escape relatively quickly
    assert!(orbit.escaped_at.is_some());
    let orbit_len = orbit.orbit.len();

    // Pixel in the set that needs many iterations
    let delta_c = (-0.8, 0.0); // Point at (-0.5, 0) is in set
    let tau_sq = 1e-6;
    let result = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        tau_sq,
    );

    // Should iterate beyond orbit length using wrap-around
    // (500 > orbit_len, so wrap-around must have occurred)
    assert!(result.iterations as usize > orbit_len || !result.escaped);
}
