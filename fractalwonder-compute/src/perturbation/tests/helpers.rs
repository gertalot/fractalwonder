use fractalwonder_core::{BigFloat, MandelbrotData};

/// Standard tau_sq threshold for tests (τ = 10⁻³)
pub const TEST_TAU_SQ: f64 = 1e-6;

/// Helper for direct computation comparison
/// Uses escape radius 256 (65536 squared) to match perturbation algorithm
pub fn compute_direct(c: &(BigFloat, BigFloat), max_iter: u32) -> MandelbrotData {
    let precision = c.0.precision_bits();
    let mut x = BigFloat::zero(precision);
    let mut y = BigFloat::zero(precision);
    let escape_radius_sq = BigFloat::with_precision(65536.0, precision); // 256² for smooth coloring

    for n in 0..max_iter {
        let x_sq = x.mul(&x);
        let y_sq = y.mul(&y);
        let z_mag_sq_bf = x_sq.add(&y_sq);
        if z_mag_sq_bf.gt(&escape_radius_sq) {
            let z_mag_sq = z_mag_sq_bf.to_f64();
            return MandelbrotData {
                iterations: n,
                max_iterations: max_iter,
                escaped: true,
                glitched: false,
                final_z_norm_sq: z_mag_sq as f32,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
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
        final_z_norm_sq: 0.0,
        final_z_re: 0.0,
        final_z_im: 0.0,
        final_derivative_re: 0.0,
        final_derivative_im: 0.0,
    }
}
