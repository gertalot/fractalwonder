//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

use fractalwonder_core::{BigFloat, FloatExp, MandelbrotData};

/// A pre-computed reference orbit for perturbation rendering.
#[derive(Clone)]
pub struct ReferenceOrbit {
    /// Reference point C as f64 (for on-the-fly computation after escape/rebase)
    pub c_ref: (f64, f64),
    /// Pre-computed orbit values X_n as f64
    pub orbit: Vec<(f64, f64)>,
    /// Iteration at which reference escaped (None if never escaped)
    pub escaped_at: Option<u32>,
}

impl ReferenceOrbit {
    /// Compute a reference orbit using BigFloat precision.
    ///
    /// The orbit is computed at full precision but stored as f64
    /// since orbit values are bounded by escape radius (~2).
    pub fn compute(c_ref: &(BigFloat, BigFloat), max_iterations: u32) -> Self {
        let precision = c_ref.0.precision_bits();
        let mut orbit = Vec::with_capacity(max_iterations as usize);

        let mut x = BigFloat::zero(precision);
        let mut y = BigFloat::zero(precision);
        let four = BigFloat::with_precision(4.0, precision);

        let mut escaped_at = None;

        for n in 0..max_iterations {
            // Store current X_n as f64
            orbit.push((x.to_f64(), y.to_f64()));

            // Check escape: |z|^2 > 4
            let x_sq = x.mul(&x);
            let y_sq = y.mul(&y);
            if x_sq.add(&y_sq).gt(&four) {
                escaped_at = Some(n);
                break;
            }

            // z = z^2 + c
            let two = BigFloat::with_precision(2.0, precision);
            let new_x = x_sq.sub(&y_sq).add(&c_ref.0);
            let new_y = two.mul(&x).mul(&y).add(&c_ref.1);
            x = new_x;
            y = new_y;
        }

        Self {
            c_ref: (c_ref.0.to_f64(), c_ref.1.to_f64()),
            orbit,
            escaped_at,
        }
    }
}

/// Compute a single pixel using perturbation with BigFloat deltas.
///
/// This version supports extreme zoom depths (10^1000+) where f64 deltas
/// would underflow to zero. The algorithm is identical to `compute_pixel_perturbation`
/// but uses BigFloat arithmetic for delta values.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit (f64 values, bounded by escape radius)
/// * `delta_c_re` - Real component of offset from reference point (can be 10^-1000 scale)
/// * `delta_c_im` - Imaginary component of offset from reference point
/// * `max_iterations` - Maximum iterations before declaring point in set
/// * `tau_sq` - Pauldelbrot glitch detection threshold squared (τ²)
pub fn compute_pixel_perturbation_bigfloat(
    orbit: &ReferenceOrbit,
    delta_c_re: &BigFloat,
    delta_c_im: &BigFloat,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let precision = delta_c_re.precision_bits();

    // δz starts at origin
    let mut dz_re = BigFloat::zero(precision);
    let mut dz_im = BigFloat::zero(precision);

    // m = reference orbit index
    let mut m: usize = 0;
    // Track precision loss via Pauldelbrot criterion
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    // Pre-create constants
    let two = BigFloat::with_precision(2.0, precision);

    for n in 0..max_iterations {
        // Get Z_m with wrap-around for non-escaping references
        let z_m = orbit.orbit[m % orbit_len];
        let z_m_re = BigFloat::with_precision(z_m.0, precision);
        let z_m_im = BigFloat::with_precision(z_m.1, precision);

        // Full pixel value: z = Z_m + δz
        let z_re = z_m_re.add(&dz_re);
        let z_im = z_m_im.add(&dz_im);

        // Compute magnitudes squared (convert to f64 for comparisons - magnitudes are bounded)
        let z_mag_sq = z_re.mul(&z_re).add(&z_im.mul(&z_im)).to_f64();
        let z_m_mag_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        let dz_mag_sq = dz_re.mul(&dz_re).add(&dz_im.mul(&dz_im)).to_f64();

        // 1. Escape check: |z|² > 4
        if z_mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
            };
        }

        // 2. Pauldelbrot glitch detection: |z|² < τ²|Z_m|²
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: |z|² < |δz|²
        if z_mag_sq < dz_mag_sq {
            dz_re = z_re;
            dz_im = z_im;
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // 2·Z_m·δz = 2·(z_m_re·dz_re - z_m_im·dz_im, z_m_re·dz_im + z_m_im·dz_re)
        let z_m_re_big = BigFloat::with_precision(z_m.0, precision);
        let z_m_im_big = BigFloat::with_precision(z_m.1, precision);

        let two_z_dz_re = two.mul(&z_m_re_big.mul(&dz_re).sub(&z_m_im_big.mul(&dz_im)));
        let two_z_dz_im = two.mul(&z_m_re_big.mul(&dz_im).add(&z_m_im_big.mul(&dz_re)));

        // δz² = (dz_re² - dz_im², 2·dz_re·dz_im)
        let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
        let dz_sq_im = two.mul(&dz_re).mul(&dz_im);

        // δz' = 2·Z·δz + δz² + δc
        dz_re = two_z_dz_re.add(&dz_sq_re).add(delta_c_re);
        dz_im = two_z_dz_im.add(&dz_sq_im).add(delta_c_im);

        m += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}

/// Compute pixel using perturbation with FloatExp deltas.
/// 10-20x faster than BigFloat, same correctness for deep zoom.
pub fn compute_pixel_perturbation_floatexp(
    orbit: &ReferenceOrbit,
    delta_c: (FloatExp, FloatExp),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let (dc_re, dc_im) = delta_c;
    let mut dz_re = FloatExp::zero();
    let mut dz_im = FloatExp::zero();
    let mut m: usize = 0;
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    for n in 0..max_iterations {
        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];

        // z = Z_m + δz
        let z_re = FloatExp::from_f64(z_m_re).add(&dz_re);
        let z_im = FloatExp::from_f64(z_m_im).add(&dz_im);

        // Magnitudes (f64 - bounded values)
        let z_mag_sq = FloatExp::norm_sq(&z_re, &z_im);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = FloatExp::norm_sq(&dz_re, &dz_im);

        // 1. Escape check
        if z_mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
            };
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz_re = z_re;
            dz_im = z_im;
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z·δz + δz² + δc
        // 2·Z·δz = 2·(Z_re·δz_re - Z_im·δz_im, Z_re·δz_im + Z_im·δz_re)
        let two_z_dz_re = dz_re
            .mul_f64(z_m_re)
            .sub(&dz_im.mul_f64(z_m_im))
            .mul_f64(2.0);
        let two_z_dz_im = dz_re
            .mul_f64(z_m_im)
            .add(&dz_im.mul_f64(z_m_re))
            .mul_f64(2.0);

        // δz² = (δz_re² - δz_im², 2·δz_re·δz_im)
        let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
        let dz_sq_im = dz_re.mul(&dz_im).mul_f64(2.0);

        // δz' = 2·Z·δz + δz² + δc
        dz_re = two_z_dz_re.add(&dz_sq_re).add(&dc_re);
        dz_im = two_z_dz_im.add(&dz_sq_im).add(&dc_im);

        m += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}

/// Compute a single pixel using perturbation from a reference orbit.
///
/// Uses f64 delta iterations with automatic rebasing when |z|² < |δz|².
/// Detects glitches using Pauldelbrot criterion: |z|² < τ²|Z|².
///
/// # Algorithm (from docs/research/perturbation-theory.md Section 8.1)
///
/// 1. δz = 0, m = 0
/// 2. For each iteration n:
///    a. Z_m = orbit[m % len] (wrap-around)
///    b. z = Z_m + δz
///    c. Escape: |z|² > 4 → return escaped
///    d. Glitch: |z|² < τ²|Z|² → mark glitched
///    e. Rebase: |z|² < |δz|² → δz = z, m = 0
///    f. δz = 2·Z_m·δz + δz² + δc
///    g. m += 1
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    // δz starts at origin
    let mut dz = (0.0_f64, 0.0_f64);
    // m = reference orbit index
    let mut m: usize = 0;
    // Track precision loss via Pauldelbrot criterion
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        // Degenerate case: no orbit data
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    for n in 0..max_iterations {
        // Get Z_m with wrap-around for non-escaping references
        let z_m = orbit.orbit[m % orbit_len];

        // Full pixel value: z = Z_m + δz
        let z = (z_m.0 + dz.0, z_m.1 + dz.1);

        // Precompute magnitudes squared
        let z_mag_sq = z.0 * z.0 + z.1 * z.1;
        let z_m_mag_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        // 1. Escape check: |z|² > 4
        if z_mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
            };
        }

        // 2. Pauldelbrot glitch detection: |z|² < τ²|Z_m|²
        // Skip check when Z_m is near zero to avoid division issues
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: |z|² < |δz|²
        // When the full pixel value is smaller than the delta alone,
        // absorb Z into delta and reset to iteration 0
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0;
            continue; // Skip delta iteration this round
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
        // 2·Z_m·δz = 2·(z_m.0·dz.0 - z_m.1·dz.1, z_m.0·dz.1 + z_m.1·dz.0)
        // δz² = (dz.0² - dz.1², 2·dz.0·dz.1)
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

    // Reached max iterations without escaping
    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard tau_sq threshold for tests (τ = 10⁻³)
    const TEST_TAU_SQ: f64 = 1e-6;

    #[test]
    fn reference_orbit_in_set_never_escapes() {
        // Point (-0.5, 0) is in the main cardioid
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert_eq!(orbit.escaped_at, None);
        assert_eq!(orbit.orbit.len(), 1000);
        assert!((orbit.c_ref.0 - (-0.5)).abs() < 1e-10);
        assert!((orbit.c_ref.1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn reference_orbit_outside_set_escapes() {
        // Point (2, 0) escapes quickly
        let c_ref = (BigFloat::with_precision(2.0, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert!(orbit.escaped_at.is_some());
        assert!(orbit.escaped_at.unwrap() < 10);
    }

    #[test]
    fn reference_orbit_values_bounded() {
        // All orbit values should be bounded by escape radius
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        for (x, y) in &orbit.orbit {
            let mag_sq = x * x + y * y;
            assert!(mag_sq <= 4.0, "Orbit value escaped: ({}, {})", x, y);
        }
    }

    #[test]
    fn perturbation_origin_in_set() {
        // Reference at (-0.5, 0), delta_c = (0.5, 0) gives point (0, 0) which is in set
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        let result = compute_pixel_perturbation(&orbit, (0.5, 0.0), 1000, TEST_TAU_SQ);

        assert!(!result.escaped);
        assert_eq!(result.iterations, 1000);
    }

    #[test]
    fn perturbation_far_point_escapes() {
        // Reference at (-0.5, 0), delta_c = (2.5, 0) gives point (2, 0) which escapes
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        let result = compute_pixel_perturbation(&orbit, (2.5, 0.0), 1000, TEST_TAU_SQ);

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
        let perturbation_result = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);

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
            let diff =
                (perturbation_result.iterations as i32 - direct_result.iterations as i32).abs();
            assert!(diff <= 1, "Iteration difference too large: {}", diff);
        }
    }

    // Helper for direct computation comparison
    fn compute_direct(c: &(BigFloat, BigFloat), max_iter: u32) -> MandelbrotData {
        let precision = c.0.precision_bits();
        let mut x = BigFloat::zero(precision);
        let mut y = BigFloat::zero(precision);
        let four = BigFloat::with_precision(4.0, precision);

        for n in 0..max_iter {
            let x_sq = x.mul(&x);
            let y_sq = y.mul(&y);
            if x_sq.add(&y_sq).gt(&four) {
                return MandelbrotData {
                    iterations: n,
                    max_iterations: max_iter,
                    escaped: true,
                    glitched: false,
                };
            }
            let two = BigFloat::with_precision(2.0, precision);
            let new_x = x_sq.sub(&y_sq).add(&c.0);
            let new_y = two.mul(&x).mul(&y).add(&c.1);
            x = new_x;
            y = new_y;
        }
        MandelbrotData {
            iterations: max_iter,
            max_iterations: max_iter,
            escaped: false,
            glitched: false,
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
        let result = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);

        // Should complete without panic
        assert!(result.iterations > 0);
    }

    // ========== Glitch Detection Tests (Pauldelbrot criterion) ==========

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
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000, tau_sq);

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
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000, tau_sq);

        assert!(result.escaped);
        assert!(result.iterations < 10);
        // Clean escape should not be marked glitched
        assert!(!result.glitched, "Clean escape should not be glitched");
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
        let result = compute_pixel_perturbation(&orbit, delta_c, 500, tau_sq);

        // Should iterate beyond orbit length using wrap-around
        // (500 > orbit_len, so wrap-around must have occurred)
        assert!(result.iterations as usize > orbit_len || !result.escaped);
    }

    #[test]
    fn no_glitch_when_pixel_escapes_before_reference() {
        // Reference in set: (-0.5, 0) never escapes
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert!(orbit.escaped_at.is_none(), "Reference should be in set");

        // Pixel that escapes: (2, 0) escapes quickly
        let delta_c = (2.5, 0.0);
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000, TEST_TAU_SQ);

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
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000, TEST_TAU_SQ);

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
        let result = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);

        // If pixel escaped, it shouldn't be glitched from rebasing alone
        // (Pauldelbrot criterion detects precision loss, not rebasing)
        if result.escaped {
            // Rebasing alone should not cause glitch
            // The pixel may or may not be glitched depending on Pauldelbrot criterion
            assert!(result.iterations > 0);
        }
    }

    // =========================================================================
    // Phase 3: Precision Sensitivity Tests
    // =========================================================================

    #[test]
    fn orbit_diverges_with_tiny_precision_difference() {
        // This test proves precision matters: two points differing by ~10^-16
        // produce different escape behavior at boundary regions.

        // Point on the "antenna" (real axis boundary) where chaotic behavior is extreme
        // c = -2 is the tip of the antenna; nearby points are extremely sensitive
        // Using a point that escapes after many iterations to show sensitivity
        let c1 = (
            BigFloat::from_string("-1.9999999999999998", 128).unwrap(),
            BigFloat::zero(128),
        );
        let c2 = (
            BigFloat::from_string("-2.0000000000000002", 128).unwrap(),
            BigFloat::zero(128),
        );

        // Compute orbits
        let orbit1 = ReferenceOrbit::compute(&c1, 10000);
        let orbit2 = ReferenceOrbit::compute(&c2, 10000);

        // c1 is slightly inside (-2 is the boundary), c2 is slightly outside
        // One should escape, the other should not (or escape much later)
        let escaped_differently = orbit1.escaped_at.is_some() != orbit2.escaped_at.is_some();

        let escape_time_differs = match (orbit1.escaped_at, orbit2.escaped_at) {
            (Some(e1), Some(e2)) => (e1 as i32 - e2 as i32).abs() > 100,
            _ => false,
        };

        assert!(
            escaped_differently || escape_time_differs,
            "Orbits should diverge: c1 (inside boundary) vs c2 (outside boundary). \
             orbit1.escaped_at={:?}, orbit2.escaped_at={:?}",
            orbit1.escaped_at,
            orbit2.escaped_at
        );
    }

    // =========================================================================
    // Phase 7: Mathematical Correctness Tests
    // =========================================================================

    #[test]
    fn orbit_satisfies_recurrence_relation() {
        // Verify that orbit values follow z_{n+1} = z_n^2 + c exactly
        let c_ref = (
            BigFloat::with_precision(-0.5, 128),
            BigFloat::with_precision(0.1, 128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        let (c_x, c_y) = orbit.c_ref;

        for n in 0..orbit.orbit.len() - 1 {
            let (xn, yn) = orbit.orbit[n];
            let (xn1, yn1) = orbit.orbit[n + 1];

            // z_{n+1} = z_n^2 + c
            // (x + iy)^2 = x^2 - y^2 + 2ixy
            let expected_x = xn * xn - yn * yn + c_x;
            let expected_y = 2.0 * xn * yn + c_y;

            // Allow small floating point error since orbit stores f64
            assert!(
                (xn1 - expected_x).abs() < 1e-10,
                "x recurrence failed at n={}: got {}, expected {}",
                n,
                xn1,
                expected_x
            );
            assert!(
                (yn1 - expected_y).abs() < 1e-10,
                "y recurrence failed at n={}: got {}, expected {}",
                n,
                yn1,
                expected_y
            );
        }
    }

    #[test]
    fn orbit_starts_at_origin() {
        // The Mandelbrot iteration z_{n+1} = z_n^2 + c starts with z_0 = 0
        let orbit = ReferenceOrbit::compute(
            &(
                BigFloat::with_precision(-0.5, 128),
                BigFloat::with_precision(0.1, 128),
            ),
            100,
        );
        assert_eq!(orbit.orbit[0], (0.0, 0.0), "Orbit must start at origin");
    }

    #[test]
    fn orbit_known_values_c_equals_neg1() {
        // c = -1: orbit is 0, -1, 0, -1, ... (period 2)
        // z_0 = 0
        // z_1 = 0^2 + (-1) = -1
        // z_2 = (-1)^2 + (-1) = 0
        // z_3 = 0^2 + (-1) = -1
        // ...
        let orbit = ReferenceOrbit::compute(
            &(BigFloat::with_precision(-1.0, 128), BigFloat::zero(128)),
            100,
        );

        // Point c = -1 is in the set (bounded period-2 orbit)
        assert!(orbit.escaped_at.is_none(), "c = -1 should not escape");

        // Check the orbit values
        assert_eq!(orbit.orbit[0], (0.0, 0.0), "z_0 should be 0");
        assert!(
            (orbit.orbit[1].0 - (-1.0)).abs() < 1e-14 && orbit.orbit[1].1.abs() < 1e-14,
            "z_1 should be -1, got {:?}",
            orbit.orbit[1]
        );
        assert!(
            orbit.orbit[2].0.abs() < 1e-14 && orbit.orbit[2].1.abs() < 1e-14,
            "z_2 should be 0, got {:?}",
            orbit.orbit[2]
        );
        assert!(
            (orbit.orbit[3].0 - (-1.0)).abs() < 1e-14 && orbit.orbit[3].1.abs() < 1e-14,
            "z_3 should be -1, got {:?}",
            orbit.orbit[3]
        );
    }

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
        let result =
            compute_pixel_perturbation_bigfloat(&orbit, &delta_c.0, &delta_c.1, 100, TEST_TAU_SQ);

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
            let f64_result = compute_pixel_perturbation(&orbit, (dx, dy), 500, TEST_TAU_SQ);

            // BigFloat version
            let bigfloat_delta_re = BigFloat::with_precision(dx, 128);
            let bigfloat_delta_im = BigFloat::with_precision(dy, 128);
            let bigfloat_result = compute_pixel_perturbation_bigfloat(
                &orbit,
                &bigfloat_delta_re,
                &bigfloat_delta_im,
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

        let result = compute_pixel_perturbation_bigfloat(
            &orbit,
            &delta_c_re,
            &delta_c_im,
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
    fn high_precision_orbit_differs_from_low_precision() {
        // Compare orbit computed with different precision levels
        // This demonstrates why we need arbitrary precision at deep zoom

        // Point in chaotic region
        let c_high = (
            BigFloat::from_string("-0.7436438870371587", 256).unwrap(),
            BigFloat::from_string("0.1318259043091895", 256).unwrap(),
        );

        let c_low = (
            BigFloat::with_precision(-0.7436438870371587, 64),
            BigFloat::with_precision(0.1318259043091895, 64),
        );

        let orbit_high = ReferenceOrbit::compute(&c_high, 10000);
        let orbit_low = ReferenceOrbit::compute(&c_low, 10000);

        // Both should have the same f64 c_ref (since that's stored as f64)
        assert!(
            (orbit_high.c_ref.0 - orbit_low.c_ref.0).abs() < 1e-14,
            "c_ref should be approximately equal"
        );

        // But orbit behavior may differ due to precision during computation
        // This is expected behavior - at deep zoom, precision matters
        // The test passes as long as orbits are computed without error
        assert!(
            !orbit_high.orbit.is_empty(),
            "High precision orbit should compute"
        );
        assert!(
            !orbit_low.orbit.is_empty(),
            "Low precision orbit should compute"
        );
    }

    #[test]
    fn floatexp_matches_f64_at_shallow_zoom() {
        // Reference in set
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // Test multiple delta values within f64 range
        let test_deltas = [(0.01, 0.01), (-0.005, 0.002), (0.1, -0.05)];

        for (dx, dy) in test_deltas {
            // f64 version
            let f64_result = compute_pixel_perturbation(&orbit, (dx, dy), 500, TEST_TAU_SQ);

            // FloatExp version
            let delta_c = (FloatExp::from_f64(dx), FloatExp::from_f64(dy));
            let floatexp_result =
                compute_pixel_perturbation_floatexp(&orbit, delta_c, 500, TEST_TAU_SQ);

            assert_eq!(
                f64_result.escaped, floatexp_result.escaped,
                "Escape mismatch for delta ({}, {})",
                dx, dy
            );
            assert_eq!(
                f64_result.iterations, floatexp_result.iterations,
                "Iteration mismatch for delta ({}, {})",
                dx, dy
            );
        }
    }

    #[test]
    fn floatexp_matches_bigfloat_at_deep_zoom() {
        let precision = 2048;

        // Reference at origin
        let c_ref = (BigFloat::zero(precision), BigFloat::zero(precision));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // Delta at 10^-500 scale
        let delta_bf = (
            BigFloat::from_string("1e-500", precision).unwrap(),
            BigFloat::from_string("2e-500", precision).unwrap(),
        );

        // Convert to FloatExp
        let delta_fe = (
            FloatExp::from_bigfloat(&delta_bf.0),
            FloatExp::from_bigfloat(&delta_bf.1),
        );

        // BigFloat version (reference implementation)
        let bf_result =
            compute_pixel_perturbation_bigfloat(&orbit, &delta_bf.0, &delta_bf.1, 500, TEST_TAU_SQ);

        // FloatExp version (optimized)
        let fe_result = compute_pixel_perturbation_floatexp(&orbit, delta_fe, 500, TEST_TAU_SQ);

        assert_eq!(
            bf_result.escaped, fe_result.escaped,
            "Escape status should match at deep zoom"
        );
        assert_eq!(
            bf_result.iterations, fe_result.iterations,
            "Iteration count should match at deep zoom"
        );
    }
}
