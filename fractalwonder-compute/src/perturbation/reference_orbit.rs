//! Reference orbit computation for perturbation rendering.
//!
//! Computes reference orbits at high precision using BigFloat, storing
//! the results as f64 for fast delta iterations.

use fractalwonder_core::BigFloat;

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
            // Convert to f64 for storage
            let orbit_val = (x.to_f64(), y.to_f64());
            let der_val = (der_x.to_f64(), der_y.to_f64());

            // Check for f64 overflow in derivative (can exceed f64 range at deep zooms
            // where derivative grows as ~512^n after n iterations)
            if !der_val.0.is_finite() || !der_val.1.is_finite() {
                escaped_at = Some(n);
                break;
            }

            // Store current Z_n and Der_n
            orbit.push(orbit_val);
            derivative.push(der_val);

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
