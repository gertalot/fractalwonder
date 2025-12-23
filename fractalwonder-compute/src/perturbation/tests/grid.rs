use super::helpers::{compute_direct, TEST_TAU_SQ};
use crate::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{BigFloat, ComplexDelta, F64Complex};

/// Test that perturbation results match direct computation for a grid of points.
/// This catches the "mosaic tile" bug where nearby pixels at the same iteration
/// get different results due to numerical issues in the rebase logic.
#[test]
fn perturbation_matches_direct_for_grid_at_1x_zoom() {
    // Simulate 1x zoom: reference at center (-0.5, 0), viewport width ~4
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // Grid of points across the viewport (simulating pixel positions)
    let grid_size = 20;
    let viewport_width = 4.0;
    let step = viewport_width / grid_size as f64;

    let mut mismatches = Vec::new();

    for iy in 0..grid_size {
        for ix in 0..grid_size {
            // delta_c from reference point to pixel
            let delta_re = -2.0 + (ix as f64 + 0.5) * step; // range: -2 to 2
            let delta_im = -2.0 + (iy as f64 + 0.5) * step;
            let delta_c = (delta_re, delta_im);

            // Perturbation result
            let perturb = compute_pixel_perturbation(
                &orbit,
                F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                1000,
                TEST_TAU_SQ,
            );

            // Direct computation at same point (c = c_ref + delta_c)
            let c = (
                BigFloat::with_precision(-0.5 + delta_re, 128),
                BigFloat::with_precision(delta_im, 128),
            );
            let direct = compute_direct(&c, 1000);

            // Compare
            if perturb.escaped != direct.escaped {
                mismatches.push((
                    ix,
                    iy,
                    delta_c,
                    "escaped mismatch",
                    perturb.iterations,
                    direct.iterations,
                ));
            } else if perturb.escaped {
                let diff = (perturb.iterations as i32 - direct.iterations as i32).abs();
                if diff > 1 {
                    mismatches.push((
                        ix,
                        iy,
                        delta_c,
                        "iteration diff > 1",
                        perturb.iterations,
                        direct.iterations,
                    ));
                }
            }
        }
    }

    // Allow some small mismatches due to floating point, but not systematic patterns
    let max_allowed = (grid_size * grid_size) / 50; // 2% tolerance
    assert!(
        mismatches.len() <= max_allowed,
        "Too many perturbation vs direct mismatches ({} > {}): {:?}",
        mismatches.len(),
        max_allowed,
        &mismatches[..mismatches.len().min(10)]
    );
}

/// Debug helper: trace through one pixel to see exactly where iterations diverge
#[test]
fn debug_single_pixel_iteration_trace() {
    // Use one of the failing pixels from the grid test
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    // delta_c = (-0.1, -1.9) gives c = (-0.6, -1.9)
    let delta_c = (-0.1, -1.9);
    let c = (-0.5 + delta_c.0, delta_c.1);
    println!("Testing c = ({}, {})", c.0, c.1);

    // Trace direct computation
    println!("\n=== Direct computation ===");
    let mut z = (0.0_f64, 0.0_f64);
    for n in 0..20 {
        let z_mag_sq = z.0 * z.0 + z.1 * z.1;
        println!("n={}: z=({:.4}, {:.4}), |z|²={:.4}", n, z.0, z.1, z_mag_sq);
        if z_mag_sq > 65536.0 {
            println!("Escaped at n={}", n);
            break;
        }
        let new_z = (z.0 * z.0 - z.1 * z.1 + c.0, 2.0 * z.0 * z.1 + c.1);
        z = new_z;
    }

    // Trace perturbation computation
    println!("\n=== Perturbation computation ===");
    let mut dz = (0.0_f64, 0.0_f64);
    let mut m: usize = 0;
    for n in 0..20u32 {
        let z_m = orbit.orbit[m % orbit.orbit.len()];
        let z = (z_m.0 + dz.0, z_m.1 + dz.1);
        let z_mag_sq = z.0 * z.0 + z.1 * z.1;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        println!(
            "n={}, m={}: Z_m=({:.4}, {:.4}), dz=({:.4}, {:.4}), z=({:.4}, {:.4}), |z|²={:.4}, |dz|²={:.4}",
            n, m, z_m.0, z_m.1, dz.0, dz.1, z.0, z.1, z_mag_sq, dz_mag_sq
        );

        if z_mag_sq > 65536.0 {
            println!("Escaped at n={}", n);
            break;
        }

        // Rebase check
        if z_mag_sq < dz_mag_sq {
            println!("  -> REBASE triggered (|z|² < |dz|²)");
            dz = z;
            m = 0;
            continue;
        }

        // Delta iteration
        let two_z_dz = (
            2.0 * (z_m.0 * dz.0 - z_m.1 * dz.1),
            2.0 * (z_m.0 * dz.1 + z_m.1 * dz.0),
        );
        let dz_sq = (dz.0 * dz.0 - dz.1 * dz.1, 2.0 * dz.0 * dz.1);
        dz = (
            two_z_dz.0 + dz_sq.0 + delta_c.0,
            two_z_dz.1 + dz_sq.1 + delta_c.1,
        );
        m += 1;
    }
}
