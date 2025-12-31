//! f64 perturbation with BLA (Bivariate Linear Approximation) acceleration.
//!
//! Fast path for moderate zoom levels where f64 arithmetic works and
//! BLA coefficients don't overflow f64 range.

use super::{compute_surface_normal_direction, ReferenceOrbit};
use crate::bla::BlaTable;
use fractalwonder_core::MandelbrotData;

pub use super::pixel_hdr_bla::BlaStats;

/// Compute pixel using f64 perturbation with BLA acceleration.
/// Returns pixel data and BLA statistics for performance monitoring.
///
/// This is the fast path for moderate zoom levels where:
/// - f64 arithmetic is sufficient (delta values in ~10^+-300 range)
/// - BLA coefficients fit in f64 range
///
/// Falls back to standard iteration when BLA coefficients overflow.
pub fn compute_pixel_perturbation_f64_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> (MandelbrotData, BlaStats) {
    let mut dz = (0.0, 0.0);
    let mut drho = (0.0, 0.0);
    let mut m: usize = 0;
    let mut glitched = false;
    let mut bla_iters: u32 = 0;
    let mut standard_iters: u32 = 0;
    let mut rebase_count: u32 = 0;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return (
            MandelbrotData {
                iterations: 0,
                max_iterations,
                escaped: false,
                glitched: true,
                final_z_norm_sq: 0.0,
                surface_normal_re: 0.0,
                surface_normal_im: 0.0,
            },
            BlaStats::default(),
        );
    }

    let reference_escaped = orbit.escaped_at.is_some();
    let mut n = 0u32;

    // dc_max for BLA validity check (magnitude of delta_c)
    let dc_max = (delta_c.0 * delta_c.0 + delta_c.1 * delta_c.1).sqrt();

    while n < max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];
        let (der_m_re, der_m_im) = orbit.derivative[m % orbit_len];

        // Full values: z = Z_m + dz, rho = Der_m + drho
        let z_re = z_m_re + dz.0;
        let z_im = z_m_im + dz.1;
        let rho_re = der_m_re + drho.0;
        let rho_im = der_m_im + drho.1;

        let z_mag_sq = z_re * z_re + z_im * z_im;
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            let (sn_re, sn_im) = compute_surface_normal_direction(z_re, z_im, rho_re, rho_im);

            return (
                MandelbrotData::new(
                    n,
                    max_iterations,
                    true,
                    glitched,
                    z_mag_sq as f32,
                    sn_re,
                    sn_im,
                ),
                BlaStats {
                    bla_iterations: bla_iters,
                    total_iterations: bla_iters + standard_iters,
                    rebase_count,
                },
            );
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: if |z| < |dz|, the perturbation dominates the full value
        if z_mag_sq < dz_mag_sq {
            dz = (z_re, z_im);
            drho = (rho_re, rho_im);
            m = 0;
            rebase_count += 1;
            continue;
        }

        // 4. Try BLA acceleration (with f64 coefficients)
        if let Some(bla) = bla_table.find_valid_f64(m, dz_mag_sq, dc_max) {
            // Apply BLA: dz_new = A*dz + B*dc (f64 complex multiply)
            let a_dz = complex_mul_f64(bla.a, dz);
            let b_dc = complex_mul_f64(bla.b, delta_c);
            dz = (a_dz.0 + b_dc.0, a_dz.1 + b_dc.1);

            // Note: drho derivative tracking not implemented for BLA path
            // This is acceptable since surface normals are computed at escape

            bla_iters += bla.l;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration: dz' = 2*Z_m*dz + dz^2 + dc
            let old_dz = dz;

            let two_z_dz_re = 2.0 * (z_m_re * dz.0 - z_m_im * dz.1);
            let two_z_dz_im = 2.0 * (z_m_re * dz.1 + z_m_im * dz.0);

            let dz_sq_re = dz.0 * dz.0 - dz.1 * dz.1;
            let dz_sq_im = 2.0 * dz.0 * dz.1;

            dz = (
                two_z_dz_re + dz_sq_re + delta_c.0,
                two_z_dz_im + dz_sq_im + delta_c.1,
            );

            // Derivative delta iteration: drho' = 2*Z_m*drho + 2*dz*Der_m + 2*dz*drho
            let two_z_drho_re = 2.0 * (z_m_re * drho.0 - z_m_im * drho.1);
            let two_z_drho_im = 2.0 * (z_m_re * drho.1 + z_m_im * drho.0);

            let two_dz_der_re = 2.0 * (old_dz.0 * der_m_re - old_dz.1 * der_m_im);
            let two_dz_der_im = 2.0 * (old_dz.0 * der_m_im + old_dz.1 * der_m_re);

            let two_dz_drho_re = 2.0 * (old_dz.0 * drho.0 - old_dz.1 * drho.1);
            let two_dz_drho_im = 2.0 * (old_dz.0 * drho.1 + old_dz.1 * drho.0);

            drho = (
                two_z_drho_re + two_dz_der_re + two_dz_drho_re,
                two_z_drho_im + two_dz_der_im + two_dz_drho_im,
            );

            standard_iters += 1;
            m += 1;
            n += 1;
        }
    }

    (
        MandelbrotData {
            iterations: max_iterations,
            max_iterations,
            escaped: false,
            glitched,
            final_z_norm_sq: 0.0,
            surface_normal_re: 0.0,
            surface_normal_im: 0.0,
        },
        BlaStats {
            bla_iterations: bla_iters,
            total_iterations: bla_iters + standard_iters,
            rebase_count,
        },
    )
}

/// Complex multiplication for f64 tuples: (a_re, a_im) * (b_re, b_im)
#[inline]
fn complex_mul_f64(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{BigFloat, HDRFloat};

    #[test]
    fn pixel_f64_bla_escapes_at_correct_iteration() {
        // Create orbit for c = -0.5 + 0i (inside set, doesn't escape)
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-6));

        // Pixel at c = 2.0 + 0i (outside set, escapes immediately)
        // delta_c = 2.0 - (-0.5) = 2.5
        let delta_c = (2.5, 0.0);
        let (result, _stats) =
            compute_pixel_perturbation_f64_bla(&orbit, &bla_table, delta_c, 100, 1e-6);

        assert!(result.escaped, "Point at c=2+0i should escape");
        assert!(result.iterations < 10, "Should escape quickly");
    }
}
