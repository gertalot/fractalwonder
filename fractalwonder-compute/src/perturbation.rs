//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

use fractalwonder_core::{BigFloat, MandelbrotData};

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

/// Compute a single pixel using perturbation from a reference orbit.
///
/// Uses f64 delta iterations with automatic rebasing when delta grows too large.
/// Falls back to on-the-fly computation when reference orbit escapes or after rebasing.
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
) -> MandelbrotData {
    let mut dx = 0.0;
    let mut dy = 0.0;

    // For on-the-fly mode after rebasing or reference escape
    let mut x = 0.0;
    let mut y = 0.0;
    let mut on_the_fly = false;

    let orbit_len = orbit.orbit.len() as u32;
    let reference_escaped = orbit.escaped_at.unwrap_or(u32::MAX);

    for n in 0..max_iterations {
        // Get X_n from orbit or compute on-the-fly
        let (xn, yn) = if !on_the_fly && n < orbit_len && n < reference_escaped {
            orbit.orbit[n as usize]
        } else {
            if !on_the_fly {
                // Switching to on-the-fly mode
                on_the_fly = true;
                // Initialize x, y from last known Z = X + delta
                if n > 0 && n <= orbit_len {
                    let prev_n = (n - 1) as usize;
                    if prev_n < orbit.orbit.len() {
                        x = orbit.orbit[prev_n].0 + dx;
                        y = orbit.orbit[prev_n].1 + dy;
                    }
                }
                dx = 0.0;
                dy = 0.0;
            }
            // Compute next X on-the-fly using the pixel's actual c value
            let pixel_c = (orbit.c_ref.0 + delta_c.0, orbit.c_ref.1 + delta_c.1);
            let new_x = x * x - y * y + pixel_c.0;
            let new_y = 2.0 * x * y + pixel_c.1;
            x = new_x;
            y = new_y;
            (x, y)
        };

        // Escape check: |X_n + delta_n|^2 > 4
        let zx = xn + dx;
        let zy = yn + dy;
        let mag_sq = zx * zx + zy * zy;

        if mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
            };
        }

        // Rebase check: |delta|^2 > 0.25 * |X|^2 (threshold 0.5)
        if !on_the_fly {
            let delta_mag_sq = dx * dx + dy * dy;
            let x_mag_sq = xn * xn + yn * yn;

            if delta_mag_sq > 0.25 * x_mag_sq && x_mag_sq > 1e-20 {
                // Rebase: switch to on-the-fly with Z as new reference
                x = zx;
                y = zy;
                dx = 0.0;
                dy = 0.0;
                on_the_fly = true;
                continue;
            }
        }

        // Delta iteration: delta_{n+1} = 2*X_n*delta_n + delta_n^2 + delta_c
        if !on_the_fly {
            let new_dx = 2.0 * (xn * dx - yn * dy) + dx * dx - dy * dy + delta_c.0;
            let new_dy = 2.0 * (xn * dy + yn * dx) + 2.0 * dx * dy + delta_c.1;
            dx = new_dx;
            dy = new_dy;
        }
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let result = compute_pixel_perturbation(&orbit, (0.5, 0.0), 1000);

        assert!(!result.escaped);
        assert_eq!(result.iterations, 1000);
    }

    #[test]
    fn perturbation_far_point_escapes() {
        // Reference at (-0.5, 0), delta_c = (2.5, 0) gives point (2, 0) which escapes
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        let result = compute_pixel_perturbation(&orbit, (2.5, 0.0), 1000);

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
        let perturbation_result = compute_pixel_perturbation(&orbit, delta_c, 500);

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
        let result = compute_pixel_perturbation(&orbit, delta_c, 500);

        // Should complete without panic
        assert!(result.iterations > 0);
    }
}
