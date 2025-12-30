//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat, MandelbrotData};

/// Compute normalized z/ρ direction for 3D lighting.
/// Returns (re, im) of the unit vector, or (0, 0) if degenerate.
/// This works at any zoom level since we normalize to a unit vector.
#[inline]
fn compute_surface_normal_direction(z_re: f64, z_im: f64, rho_re: f64, rho_im: f64) -> (f32, f32) {
    // u = z / ρ (complex division)
    // u = z * conj(ρ) / |ρ|²
    let rho_norm_sq = rho_re * rho_re + rho_im * rho_im;
    if !rho_norm_sq.is_finite() || rho_norm_sq == 0.0 {
        return (0.0, 0.0);
    }

    let u_re = (z_re * rho_re + z_im * rho_im) / rho_norm_sq;
    let u_im = (z_im * rho_re - z_re * rho_im) / rho_norm_sq;

    // Normalize to unit vector
    let u_norm = (u_re * u_re + u_im * u_im).sqrt();
    if !u_norm.is_finite() || u_norm == 0.0 {
        return (0.0, 0.0);
    }

    ((u_re / u_norm) as f32, (u_im / u_norm) as f32)
}

/// A pre-computed reference orbit for perturbation rendering.
#[derive(Clone)]
pub struct ReferenceOrbit {
    /// Reference point C as f64 (for on-the-fly computation after escape/rebase)
    pub c_ref: (f64, f64),
    /// Pre-computed orbit values X_n as f64
    pub orbit: Vec<(f64, f64)>,
    /// Pre-computed derivative values Der_n = dZ_n/dC as f64
    pub derivative: Vec<(f64, f64)>,
    /// Iteration at which reference escaped (None if never escaped)
    pub escaped_at: Option<u32>,
}

impl ReferenceOrbit {
    /// Compute a reference orbit using BigFloat precision.
    ///
    /// The orbit is computed at full precision but stored as f64
    /// since orbit values are bounded by escape radius (256).
    pub fn compute(c_ref: &(BigFloat, BigFloat), max_iterations: u32) -> Self {
        let precision = c_ref.0.precision_bits();
        let mut orbit = Vec::with_capacity(max_iterations as usize);
        let mut derivative = Vec::with_capacity(max_iterations as usize);

        let mut x = BigFloat::zero(precision);
        let mut y = BigFloat::zero(precision);
        // Derivative: Der_0 = 0
        let mut der_x = BigFloat::zero(precision);
        let mut der_y = BigFloat::zero(precision);

        let escape_radius_sq = BigFloat::with_precision(65536.0, precision);
        let one = BigFloat::with_precision(1.0, precision);
        let two = BigFloat::with_precision(2.0, precision);

        let mut escaped_at = None;

        for n in 0..max_iterations {
            // Store current Z_n and Der_n as f64
            orbit.push((x.to_f64(), y.to_f64()));
            derivative.push((der_x.to_f64(), der_y.to_f64()));

            // Check escape: |z|^2 > 65536
            let x_sq = x.mul(&x);
            let y_sq = y.mul(&y);
            if x_sq.add(&y_sq).gt(&escape_radius_sq) {
                escaped_at = Some(n);
                break;
            }

            // Derivative update: Der' = 2*Z*Der + 1
            // (der_x + i*der_y)' = 2*(x + i*y)*(der_x + i*der_y) + 1
            // Real: 2*(x*der_x - y*der_y) + 1
            // Imag: 2*(x*der_y + y*der_x)
            let new_der_x = two.mul(&x.mul(&der_x).sub(&y.mul(&der_y))).add(&one);
            let new_der_y = two.mul(&x.mul(&der_y).add(&y.mul(&der_x)));

            // z = z^2 + c
            let new_x = x_sq.sub(&y_sq).add(&c_ref.0);
            let new_y = two.mul(&x).mul(&y).add(&c_ref.1);

            x = new_x;
            y = new_y;
            der_x = new_der_x;
            der_y = new_der_y;
        }

        Self {
            c_ref: (c_ref.0.to_f64(), c_ref.1.to_f64()),
            orbit,
            derivative,
            escaped_at,
        }
    }
}

/// Compute pixel using perturbation with HDRFloat deltas and BLA acceleration.
pub fn compute_pixel_perturbation_hdr_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: HDRComplex,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let mut dz = HDRComplex::ZERO;
    let mut drho = HDRComplex::ZERO; // Derivative delta
    let mut m: usize = 0;
    let mut glitched = false;

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

    // Check if reference escaped (short orbit that will wrap)
    let reference_escaped = orbit.escaped_at.is_some();

    let mut n = 0u32;

    while n < max_iterations {
        // Reference exhaustion detection: m exceeded orbit length
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

        let z_mag_sq = z_re.square().add(&z_im.square()).to_f64();
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        // Use HDRFloat for dz_mag_sq to prevent f64 underflow at deep zoom
        // where |δz|² can be as small as 10^-1800 (beyond f64's ~10^-308 limit)
        let dz_mag_sq = dz.norm_sq_hdr();

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            let (sn_re, sn_im) = compute_surface_normal_direction(
                z_re.to_f64(),
                z_im.to_f64(),
                rho_re.to_f64(),
                rho_im.to_f64(),
            );
            return MandelbrotData::new(
                n,
                max_iterations,
                true,
                glitched,
                z_mag_sq as f32,
                sn_re,
                sn_im,
            );
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        // NOTE: Rebasing is a precision technique, NOT a Mandelbrot iteration.
        // The iteration count n should NOT increment during rebase.
        // Compare using HDRFloat to handle deep zoom where values exceed f64 range
        let z_mag_sq_hdr = z_re.square().add(&z_im.square());
        if z_mag_sq_hdr.sub(&dz_mag_sq).is_negative() {
            dz = HDRComplex { re: z_re, im: z_im };
            drho = HDRComplex {
                re: rho_re,
                im: rho_im,
            }; // Also rebase derivative
            m = 0;
            // Do NOT increment n - rebase is not a real iteration
            continue;
        }

        // 4. Try BLA acceleration
        if let Some(bla) = bla_table.find_valid(m, &dz_mag_sq) {
            // Apply BLA: δz_new = A·δz + B·δc
            // Now uses HDRComplex multiplication - no f64 overflow possible
            let a_dz = bla.a.mul(&dz);
            let b_dc = bla.b.mul(&delta_c);
            dz = a_dz.add(&b_dc);

            // NOTE: During BLA skip, drho is NOT updated (less accurate but functional)
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration (no valid BLA)
            // CRITICAL: Store old dz before updating - needed for derivative calculation
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
            // Uses old_dz (the value BEFORE the update above)
            // Term 1: 2·Z_m·δρ (complex multiplication)
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

            // Term 2: 2·δz·Der_m (complex multiplication, using old_dz)
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

            // Term 3: 2·δz·δρ (complex multiplication, using old_dz)
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

            m += 1;
            n += 1;
        }
    }

    // Interior point - no surface normal needed
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

use fractalwonder_core::ComplexDelta;

/// Generic perturbation iteration for any ComplexDelta type.
///
/// Computes the Mandelbrot iteration using perturbation theory with
/// the provided delta type. The compiler monomorphizes this into
/// type-specific code with zero runtime overhead.
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

    // Check if reference escaped at iteration 0
    let reference_escaped = orbit.escaped_at.is_some();

    // Initialize deltas with matching precision
    let mut dz = delta_c.zero();
    let mut drho = delta_c.zero();

    let mut m: usize = 0;
    let mut n: u32 = 0;
    let mut glitched = false;

    while n < max_iterations {
        // Glitch if we've run past a finite orbit
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        // Get Z_m and Der_m with wrap-around
        let z_m = orbit.orbit[m % orbit_len];
        let der_m = orbit.derivative[m % orbit_len];
        let z_m_complex = D::from_f64_pair(z_m.0, z_m.1);
        let der_m_complex = D::from_f64_pair(der_m.0, der_m.1);

        // Full z = Z_m + δz
        let z = z_m_complex.add(&dz);
        let z_norm_sq = z.norm_sq();

        // Full derivative ρ = Der_m + δρ
        let rho = der_m_complex.add(&drho);

        // Escape check
        if z_norm_sq > 65536.0 {
            let (z_re, z_im) = z.to_f64_pair();
            let (rho_re, rho_im) = rho.to_f64_pair();
            let (sn_re, sn_im) = compute_surface_normal_direction(z_re, z_im, rho_re, rho_im);
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

    // Interior point - no surface normal needed
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

use crate::bla::BlaTable;

#[cfg(test)]
#[path = "perturbation/tests/mod.rs"]
mod tests;
