//! HDR perturbation with BLA (Bivariate Linear Approximation) acceleration.
//!
//! Specialized for deep zoom rendering where HDRFloat prevents underflow
//! and BLA skips iterations for performance.

use super::{compute_surface_normal_direction, ReferenceOrbit};
use crate::bla::BlaTable;
use fractalwonder_core::{HDRComplex, HDRFloat, MandelbrotData};

/// BLA statistics for a single pixel computation.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlaStats {
    /// Iterations skipped via BLA.
    pub bla_iterations: u32,
    /// Total iterations (BLA + standard).
    pub total_iterations: u32,
}

/// Compute pixel using perturbation with HDRFloat deltas and BLA acceleration.
/// Returns pixel data and BLA statistics for performance monitoring.
pub fn compute_pixel_perturbation_hdr_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: HDRComplex,
    max_iterations: u32,
    tau_sq: f64,
) -> (MandelbrotData, BlaStats) {
    let mut dz = HDRComplex::ZERO;
    let mut drho = HDRComplex::ZERO;
    let mut m: usize = 0;
    let mut glitched = false;
    let mut bla_iters: u32 = 0;
    let mut standard_iters: u32 = 0;

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

    while n < max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];
        let (der_m_re, der_m_im) = orbit.derivative[m % orbit_len];

        // Full values: z = Z_m + δz, ρ = Der_m + δρ
        let z_re = HDRFloat::from_f64(z_m_re).add(&dz.re);
        let z_im = HDRFloat::from_f64(z_m_im).add(&dz.im);
        let rho_re = HDRFloat::from_f64(der_m_re).add(&drho.re);
        let rho_im = HDRFloat::from_f64(der_m_im).add(&drho.im);

        let z_mag_sq_hdr = z_re.square().add(&z_im.square());
        let z_mag_sq = z_mag_sq_hdr.to_f64();
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        // Use HDRFloat for dz_mag_sq to prevent f64 underflow at deep zoom
        let dz_mag_sq = dz.norm_sq_hdr();

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            let (sn_re, sn_im) = compute_surface_normal_direction(
                z_re.to_f64(),
                z_im.to_f64(),
                rho_re.to_f64(),
                rho_im.to_f64(),
            );

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
                },
            );
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: if |z| < |δz|, the perturbation dominates the full value
        // Use HDRFloat comparison to correctly handle underflow at deep zoom
        if z_mag_sq_hdr.sub(&dz_mag_sq).is_negative() {
            dz = HDRComplex { re: z_re, im: z_im };
            drho = HDRComplex {
                re: rho_re,
                im: rho_im,
            };
            m = 0;
            continue;
        }

        // 4. Try BLA acceleration
        let bla_entry = bla_table.find_valid(m, &dz_mag_sq, bla_table.dc_max());

        if let Some(bla) = bla_entry {
            // Apply BLA: δz_new = A·δz + B·δc
            let a_dz = bla.a.mul(&dz);
            let b_dc = bla.b.mul(&delta_c);
            dz = a_dz.add(&b_dc);

            bla_iters += bla.l;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration
            let old_dz = dz;

            let two_z_dz_re = dz
                .re
                .mul_f64(z_m_re)
                .sub(&dz.im.mul_f64(z_m_im))
                .mul_f64(2.0);
            let two_z_dz_im = dz
                .re
                .mul_f64(z_m_im)
                .add(&dz.im.mul_f64(z_m_re))
                .mul_f64(2.0);

            let dz_sq = dz.square();

            dz = HDRComplex {
                re: two_z_dz_re.add(&dz_sq.re).add(&delta_c.re),
                im: two_z_dz_im.add(&dz_sq.im).add(&delta_c.im),
            };

            // Derivative delta iteration: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
            let two_z_drho_re = drho
                .re
                .mul_f64(z_m_re)
                .sub(&drho.im.mul_f64(z_m_im))
                .mul_f64(2.0);
            let two_z_drho_im = drho
                .re
                .mul_f64(z_m_im)
                .add(&drho.im.mul_f64(z_m_re))
                .mul_f64(2.0);

            let two_dz_der_re = old_dz
                .re
                .mul_f64(der_m_re)
                .sub(&old_dz.im.mul_f64(der_m_im))
                .mul_f64(2.0);
            let two_dz_der_im = old_dz
                .re
                .mul_f64(der_m_im)
                .add(&old_dz.im.mul_f64(der_m_re))
                .mul_f64(2.0);

            let two_dz_drho_re = old_dz
                .re
                .mul(&drho.re)
                .sub(&old_dz.im.mul(&drho.im))
                .mul_f64(2.0);
            let two_dz_drho_im = old_dz
                .re
                .mul(&drho.im)
                .add(&old_dz.im.mul(&drho.re))
                .mul_f64(2.0);

            drho = HDRComplex {
                re: two_z_drho_re.add(&two_dz_der_re).add(&two_dz_drho_re),
                im: two_z_drho_im.add(&two_dz_der_im).add(&two_dz_drho_im),
            };

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
        },
    )
}
