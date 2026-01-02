//! Generic perturbation pixel computation.
//!
//! Provides a single generic implementation for f64, HDRFloat, and BigFloat
//! delta types via the `ComplexDelta` trait.

use super::{compute_surface_normal_direction, ReferenceOrbit};
use fractalwonder_core::{ComplexDelta, HDRFloat, MandelbrotData};

/// Generic perturbation iteration for any ComplexDelta type.
pub fn compute_pixel_perturbation<D: ComplexDelta>(
    orbit: &ReferenceOrbit,
    delta_c: D,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
            final_z_norm_sq: 0.0,
            surface_normal_re: 0.0,
            surface_normal_im: 0.0,
        };
    }

    let reference_escaped = orbit.escaped_at.is_some();
    let mut dz = delta_c.zero();
    let mut drho = delta_c.zero();
    let mut m: usize = 0;
    let mut n: u32 = 0;
    let mut glitched = false;

    while n < max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let z_m = orbit.orbit[m % orbit_len];
        let der_m = orbit.derivative[m % orbit_len];
        let z_m_complex = D::from_f64_pair(z_m.0, z_m.1);
        let der_m_complex = D::from_f64_pair(der_m.0, der_m.1);

        let z = z_m_complex.add(&dz);
        let z_norm_sq = z.norm_sq();
        let rho = der_m_complex.add(&drho);

        // Escape check
        if z_norm_sq > 65536.0 {
            let (z_re, z_im) = z.to_f64_pair();
            let (rho_re, rho_im) = rho.to_f64_pair();
            let (sn_re, sn_im) = compute_surface_normal_direction(
                &HDRFloat::from_f64(z_re),
                &HDRFloat::from_f64(z_im),
                &HDRFloat::from_f64(rho_re),
                &HDRFloat::from_f64(rho_im),
            );
            return MandelbrotData::new(
                n,
                max_iterations,
                true,
                glitched,
                z_norm_sq as f32,
                sn_re,
                sn_im,
            );
        }

        // Pauldelbrot glitch detection
        let z_m_norm_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        if z_m_norm_sq > 1e-20 && z_norm_sq < tau_sq * z_m_norm_sq {
            glitched = true;
        }

        // Rebase check
        let dz_norm_sq = dz.norm_sq();
        if z_norm_sq < dz_norm_sq {
            dz = z;
            drho = rho;
            m = 0;
            continue;
        }

        // Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        let old_dz = dz.clone();
        let two_z_dz = z_m_complex.mul(&dz).scale(2.0);
        let dz_sq = dz.square();
        dz = two_z_dz.add(&dz_sq).add(&delta_c);

        // Derivative iteration: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
        let term1 = z_m_complex.mul(&drho).scale(2.0);
        let term2 = old_dz.mul(&der_m_complex).scale(2.0);
        let term3 = old_dz.mul(&drho).scale(2.0);
        drho = term1.add(&term2).add(&term3);

        m += 1;
        n += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
        final_z_norm_sq: 0.0,
        surface_normal_re: 0.0,
        surface_normal_im: 0.0,
    }
}
