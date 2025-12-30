use super::helpers::TEST_TAU_SQ;
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, BlaTable, ReferenceOrbit,
};
use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat};

#[test]
fn bla_version_matches_non_bla_for_escaping_point() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Small delta that escapes
    let delta_c = HDRComplex {
        re: HDRFloat::from_f64(0.1),
        im: HDRFloat::from_f64(0.1),
    };
    let dc_max = HDRFloat::from_f64(0.15);
    let bla_table = BlaTable::compute(&orbit, &dc_max);

    // Non-BLA version
    let result_no_bla = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);

    // BLA version
    let (result_bla, _stats) =
        compute_pixel_perturbation_hdr_bla(&orbit, &bla_table, delta_c, 500, TEST_TAU_SQ);

    assert_eq!(result_no_bla.escaped, result_bla.escaped);
    assert_eq!(result_no_bla.iterations, result_bla.iterations);
}

#[test]
fn bla_version_matches_non_bla_for_in_set_point() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    let delta_c = HDRComplex {
        re: HDRFloat::from_f64(0.01),
        im: HDRFloat::from_f64(0.01),
    };
    let dc_max = HDRFloat::from_f64(0.02);
    let bla_table = BlaTable::compute(&orbit, &dc_max);

    let result_no_bla = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);
    let (result_bla, _stats) =
        compute_pixel_perturbation_hdr_bla(&orbit, &bla_table, delta_c, 500, TEST_TAU_SQ);

    assert_eq!(result_no_bla.escaped, result_bla.escaped);
    assert_eq!(result_no_bla.iterations, result_bla.iterations);
}

#[test]
fn bla_matches_non_bla_for_many_deltas() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    let test_deltas = [
        (0.01, 0.01),
        (-0.005, 0.002),
        (0.1, -0.05),
        (0.0, 0.001),
        (0.05, 0.05),
        (-0.02, 0.03),
    ];

    for (dx, dy) in test_deltas {
        let delta_c = HDRComplex {
            re: HDRFloat::from_f64(dx),
            im: HDRFloat::from_f64(dy),
        };
        let dc_max = HDRFloat::from_f64((dx.abs() + dy.abs()).max(0.001));
        let bla_table = BlaTable::compute(&orbit, &dc_max);

        let result_no_bla = compute_pixel_perturbation(&orbit, delta_c, 1000, TEST_TAU_SQ);
        let (result_bla, _stats) =
            compute_pixel_perturbation_hdr_bla(&orbit, &bla_table, delta_c, 1000, TEST_TAU_SQ);

        assert_eq!(
            result_no_bla.escaped, result_bla.escaped,
            "Escape mismatch for delta ({}, {})",
            dx, dy
        );
        assert_eq!(
            result_no_bla.iterations, result_bla.iterations,
            "Iteration mismatch for delta ({}, {}): no_bla={}, bla={}",
            dx, dy, result_no_bla.iterations, result_bla.iterations
        );
    }
}

#[test]
fn bla_handles_rebasing() {
    // Use a reference point where rebasing will be triggered
    // but with small enough deltas that BLA remains valid
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    // Small delta values that will stay within BLA validity
    let delta_c = HDRComplex {
        re: HDRFloat::from_f64(0.005),
        im: HDRFloat::from_f64(0.003),
    };
    let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

    let result_no_bla = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);
    let (result_bla, _stats) =
        compute_pixel_perturbation_hdr_bla(&orbit, &bla_table, delta_c, 500, TEST_TAU_SQ);

    assert_eq!(
        result_no_bla.escaped, result_bla.escaped,
        "Escape mismatch: no_bla={}, bla={}, no_bla_iters={}, bla_iters={}",
        result_no_bla.escaped, result_bla.escaped, result_no_bla.iterations, result_bla.iterations
    );
    assert_eq!(result_no_bla.iterations, result_bla.iterations);
}
