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

    /// Merge two BLAs: x (first) then y (second).
    /// Result skips l_x + l_y iterations.
    pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: f64) -> BlaEntry {
        // A_merged = A_y * A_x (complex multiplication)
        let a_re = y.a_re * x.a_re - y.a_im * x.a_im;
        let a_im = y.a_re * x.a_im + y.a_im * x.a_re;

        // B_merged = A_y * B_x + B_y
        let b_re = (y.a_re * x.b_re - y.a_im * x.b_im) + y.b_re;
        let b_im = (y.a_re * x.b_im + y.a_im * x.b_re) + y.b_im;

        // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
        let r_x = x.r_sq.sqrt();
        let r_y = y.r_sq.sqrt();
        let b_x_mag = (x.b_re * x.b_re + x.b_im * x.b_im).sqrt();
        let a_x_mag = (x.a_re * x.a_re + x.a_im * x.a_im).sqrt();

        let r_adjusted = (r_y - b_x_mag * dc_max).max(0.0) / a_x_mag.max(1e-300);
        let r = r_x.min(r_adjusted);

        BlaEntry {
            a_re,
            a_im,
            b_re,
            b_im,
            l: x.l + y.l,
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

    #[test]
    fn bla_entry_merge_two_single_iterations() {
        // Two single-iteration BLAs should merge into one that skips 2
        let x = BlaEntry::from_orbit_point(1.0, 0.0);  // Z = 1
        let y = BlaEntry::from_orbit_point(0.5, 0.0);  // Z = 0.5

        let dc_max = 0.001;  // Small delta_c
        let merged = BlaEntry::merge(&x, &y, dc_max);

        // l should be 2
        assert_eq!(merged.l, 2);

        // A_merged = A_y * A_x = (1.0, 0) * (2.0, 0) = (2.0, 0)
        // (complex multiply: (1)(2) - (0)(0) = 2, (1)(0) + (0)(2) = 0)
        assert!((merged.a_re - 2.0).abs() < 1e-14);
        assert!((merged.a_im - 0.0).abs() < 1e-14);

        // B_merged = A_y * B_x + B_y = (1,0)*(1,0) + (1,0) = (2,0)
        assert!((merged.b_re - 2.0).abs() < 1e-14);
        assert!((merged.b_im - 0.0).abs() < 1e-14);
    }
}
