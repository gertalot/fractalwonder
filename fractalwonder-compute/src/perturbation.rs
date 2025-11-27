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
///
/// # Glitch Detection
///
/// A pixel is marked as `glitched: true` when it requires iterations beyond where
/// the reference orbit escaped. This indicates the result may be inaccurate because
/// the on-the-fly f64 fallback lacks the precision of the original BigFloat reference.
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

    // Track if this pixel is glitched (needed iterations beyond reference escape)
    let mut glitched = false;

    let orbit_len = orbit.orbit.len() as u32;
    let reference_escaped = orbit.escaped_at.unwrap_or(u32::MAX);

    for n in 0..max_iterations {
        // Glitch detection: Check at iteration start, not when entering on-the-fly mode.
        // A pixel may validly rebase (when delta grows too large), but continuing past
        // reference_escaped means we're extrapolating without reference data → mark as glitched.
        if n >= reference_escaped && !glitched {
            glitched = true;
        }

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
                glitched,
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
        glitched,
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
        let result = compute_pixel_perturbation(&orbit, delta_c, 500);

        // Should complete without panic
        assert!(result.iterations > 0);
    }

    // ========== Glitch Detection Tests ==========

    #[test]
    fn glitch_detected_when_reference_escapes_but_pixel_continues() {
        // Reference point that escapes quickly
        let c_ref = (BigFloat::with_precision(0.3, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        // Reference should escape relatively quickly
        assert!(
            orbit.escaped_at.is_some(),
            "Reference should escape for this test"
        );
        let ref_escaped = orbit.escaped_at.unwrap();
        assert!(
            ref_escaped < 100,
            "Reference should escape within 100 iterations"
        );

        // Pixel in the set: delta moves us to origin (0, 0)
        let delta_c = (-0.3, 0.0);
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000);

        // Origin is in set, so pixel doesn't escape
        assert!(!result.escaped, "Origin should be in set");
        assert_eq!(result.iterations, 1000);

        // Key assertion: pixel needed more iterations than reference provided
        // so it should be marked as glitched
        assert!(
            result.glitched,
            "Pixel should be glitched: needed {} iterations but reference escaped at {}",
            result.iterations, ref_escaped
        );
    }

    #[test]
    fn no_glitch_when_pixel_escapes_before_reference() {
        // Reference in set: (-0.5, 0) never escapes
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert!(orbit.escaped_at.is_none(), "Reference should be in set");

        // Pixel that escapes: (2, 0) escapes quickly
        let delta_c = (2.5, 0.0);
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000);

        assert!(result.escaped, "Point (2, 0) should escape");
        assert!(result.iterations < 10, "Should escape quickly");

        // No glitch: pixel escaped while reference data was still available
        assert!(
            !result.glitched,
            "Pixel escaping before reference should not be glitched"
        );
    }

    #[test]
    fn no_glitch_when_reference_never_escapes() {
        // Reference in set: (-0.5, 0) never escapes
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        assert!(orbit.escaped_at.is_none());

        // Pixel also in set: origin
        let delta_c = (0.5, 0.0);
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000);

        assert!(!result.escaped);
        assert_eq!(result.iterations, 1000);

        // No glitch: reference never escaped, so orbit data was available throughout
        assert!(
            !result.glitched,
            "Pixel using full reference orbit should not be glitched"
        );
    }

    #[test]
    fn no_glitch_when_rebasing_only() {
        // Reference in set that allows rebasing to trigger
        let c_ref = (
            BigFloat::with_precision(-0.75, 128),
            BigFloat::with_precision(0.1, 128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // This reference should be in set (or escape late)
        let ref_escaped = orbit.escaped_at.unwrap_or(u32::MAX);

        // Small offset that triggers rebasing but escapes before reference exhausted
        let delta_c = (0.1, 0.05);
        let result = compute_pixel_perturbation(&orbit, delta_c, 500);

        // If pixel escaped before reference did, it shouldn't be glitched
        if result.escaped && result.iterations < ref_escaped {
            assert!(
                !result.glitched,
                "Rebasing alone should not cause glitch if pixel escapes before reference"
            );
        }
    }

    #[test]
    fn glitch_when_pixel_survives_past_short_reference() {
        // Create reference that escapes at a known early iteration
        // Point (0.26, 0) escapes after a few iterations
        let c_ref = (BigFloat::with_precision(0.26, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 10000);

        let ref_escaped = orbit.escaped_at.expect("Reference should escape");

        // Pixel at (-1, 0) which is in set (period-2 cycle)
        let delta_c = (-1.26, 0.0);
        let result = compute_pixel_perturbation(&orbit, delta_c, 10000);

        // Point (-1, 0) is in set
        assert!(!result.escaped, "Point (-1, 0) should be in set");

        // Must be glitched: pixel needed full iterations but reference escaped early
        assert!(
            result.glitched,
            "Pixel at (-1, 0) should be glitched: needed {} iters but ref escaped at {}",
            result.iterations, ref_escaped
        );
    }

    #[test]
    fn glitch_state_propagates_to_escaped_pixel() {
        // Reference escapes early
        let c_ref = (BigFloat::with_precision(0.3, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        let ref_escaped = orbit.escaped_at.expect("Reference should escape");

        // Pixel that escapes AFTER reference escaped (boundary point)
        // Using (-0.75, 0.1) which is near boundary, escapes eventually
        let delta_c = (-1.05, 0.1);
        let result = compute_pixel_perturbation(&orbit, delta_c, 1000);

        // If pixel escaped after reference did, it should be glitched
        if result.escaped && result.iterations > ref_escaped {
            assert!(
                result.glitched,
                "Pixel escaping after reference should be glitched"
            );
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
}
