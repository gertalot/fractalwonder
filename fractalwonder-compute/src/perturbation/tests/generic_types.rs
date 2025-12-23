use super::helpers::TEST_TAU_SQ;
use crate::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{
    BigFloat, BigFloatComplex, ComplexDelta, F64Complex, HDRComplex, HDRFloat,
};

#[test]
fn generic_f64_matches_original_escaped() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);
    let delta_c = (0.5, 0.0);

    let original = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        TEST_TAU_SQ,
    );
    let generic = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        1000,
        TEST_TAU_SQ,
    );

    assert_eq!(original.iterations, generic.iterations);
    assert_eq!(original.escaped, generic.escaped);
    assert_eq!(original.glitched, generic.glitched);
}

#[test]
fn generic_f64_matches_original_in_set() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);
    let delta_c = (0.01, 0.01);

    let original = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        TEST_TAU_SQ,
    );
    let generic = compute_pixel_perturbation(
        &orbit,
        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
        500,
        TEST_TAU_SQ,
    );

    assert_eq!(original.iterations, generic.iterations);
    assert_eq!(original.escaped, generic.escaped);
}

#[test]
fn generic_hdr_matches_original() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Test several delta values
    let test_deltas = [(0.1, 0.05), (0.01, 0.01), (-0.05, 0.1), (0.5, 0.0)];

    for (dx, dy) in test_deltas {
        let delta_hdr = HDRComplex {
            re: HDRFloat::from_f64(dx),
            im: HDRFloat::from_f64(dy),
        };

        let original = compute_pixel_perturbation(&orbit, delta_hdr, 500, TEST_TAU_SQ);
        let generic = compute_pixel_perturbation(&orbit, delta_hdr, 500, TEST_TAU_SQ);

        assert_eq!(
            original.iterations, generic.iterations,
            "Iteration mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            original.escaped, generic.escaped,
            "Escaped mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            original.glitched, generic.glitched,
            "Glitched mismatch for delta ({}, {})",
            dx, dy
        );
    }
}

#[test]
fn generic_bigfloat_matches_original() {
    let precision = 256;
    let c_ref = (
        BigFloat::with_precision(-0.5, precision),
        BigFloat::zero(precision),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    let test_deltas = [(0.1, 0.05), (0.01, 0.01), (-0.05, 0.1), (0.5, 0.0)];

    for (dx, dy) in test_deltas {
        let delta_re = BigFloat::with_precision(dx, precision);
        let delta_im = BigFloat::with_precision(dy, precision);

        let original = compute_pixel_perturbation(
            &orbit,
            BigFloatComplex::new(delta_re.clone(), delta_im.clone()),
            500,
            TEST_TAU_SQ,
        );
        let generic = compute_pixel_perturbation(
            &orbit,
            BigFloatComplex::new(delta_re.clone(), delta_im.clone()),
            500,
            TEST_TAU_SQ,
        );

        assert_eq!(
            original.iterations, generic.iterations,
            "Iteration mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            original.escaped, generic.escaped,
            "Escaped mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            original.glitched, generic.glitched,
            "Glitched mismatch for delta ({}, {})",
            dx, dy
        );
    }
}
