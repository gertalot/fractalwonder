//! Bivariate Linear Approximation for iteration skipping.

/// Single BLA entry: skips `l` iterations.
/// Applies: δz_new = A·δz + B·δc
#[derive(Clone, Debug, PartialEq)]
pub struct BlaEntry {
    pub a_re: f64,
    pub a_im: f64,
    pub b_re: f64,
    pub b_im: f64,
    pub l: u32,
    pub r_sq: f64,
}

impl BlaEntry {
    /// Create a single-iteration BLA from a reference orbit point Z = (z_re, z_im).
    pub fn from_orbit_point(z_re: f64, z_im: f64) -> Self {
        let epsilon = 2.0_f64.powi(-53);
        let z_mag = (z_re * z_re + z_im * z_im).sqrt();
        let r = epsilon * z_mag;

        Self {
            a_re: 2.0 * z_re,
            a_im: 2.0 * z_im,
            b_re: 1.0,
            b_im: 0.0,
            l: 1,
            r_sq: r * r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bla_entry_from_orbit_point() {
        // Z = (1.0, 0.5), ε = 2^-53
        // A = 2Z = (2.0, 1.0)
        // B = 1
        // r = ε·|Z| = ε·√(1 + 0.25) ≈ ε·1.118
        let entry = BlaEntry::from_orbit_point(1.0, 0.5);

        assert!((entry.a_re - 2.0).abs() < 1e-14);
        assert!((entry.a_im - 1.0).abs() < 1e-14);
        assert!((entry.b_re - 1.0).abs() < 1e-14);
        assert!((entry.b_im - 0.0).abs() < 1e-14);
        assert_eq!(entry.l, 1);

        let z_mag = (1.0_f64 * 1.0 + 0.5 * 0.5).sqrt();
        let epsilon = 2.0_f64.powi(-53);
        let expected_r_sq = (epsilon * z_mag).powi(2);
        assert!((entry.r_sq - expected_r_sq).abs() < 1e-40);
    }
}
