//! Extended-range floating point for perturbation arithmetic.
//!
//! FloatExp = f64 mantissa + i64 exponent, providing unlimited range
//! with 53-bit precision. 10-20x faster than BigFloat for delta iteration.

use serde::{Deserialize, Serialize};

/// Extended-range floating point: f64 mantissa + i64 exponent.
/// Value = mantissa × 2^exp (or 0 if mantissa == 0).
/// Mantissa normalized to [0.5, 1.0) for non-zero values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatExp {
    mantissa: f64,
    exp: i64,
}

impl FloatExp {
    /// Zero value.
    pub fn zero() -> Self {
        Self {
            mantissa: 0.0,
            exp: 0,
        }
    }

    /// Create from f64 (normalizes automatically).
    pub fn from_f64(val: f64) -> Self {
        if val == 0.0 {
            return Self::zero();
        }
        // frexp returns (mantissa, exponent) where mantissa is in [0.5, 1.0)
        let (mantissa, exp) = libm::frexp(val);
        Self {
            mantissa,
            exp: exp as i64,
        }
    }

    /// Convert to f64 (may overflow/underflow for extreme exponents).
    pub fn to_f64(&self) -> f64 {
        if self.mantissa == 0.0 {
            return 0.0;
        }
        // Handle extreme exponents
        if self.exp > 1023 {
            return if self.mantissa > 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
        }
        if self.exp < -1074 {
            return 0.0;
        }
        libm::ldexp(self.mantissa, self.exp as i32)
    }

    /// Check if zero.
    pub fn is_zero(&self) -> bool {
        self.mantissa == 0.0
    }

    /// Multiply two FloatExp values.
    pub fn mul(&self, other: &Self) -> Self {
        if self.mantissa == 0.0 || other.mantissa == 0.0 {
            return Self::zero();
        }
        Self {
            mantissa: self.mantissa * other.mantissa,
            exp: self.exp + other.exp,
        }
        .normalize()
    }

    /// Multiply by f64 scalar (for 2·Z·δz where Z is f64).
    pub fn mul_f64(&self, scalar: f64) -> Self {
        if self.mantissa == 0.0 || scalar == 0.0 {
            return Self::zero();
        }
        Self {
            mantissa: self.mantissa * scalar,
            exp: self.exp,
        }
        .normalize()
    }

    /// Add two FloatExp values.
    pub fn add(&self, other: &Self) -> Self {
        if self.mantissa == 0.0 {
            return *other;
        }
        if other.mantissa == 0.0 {
            return *self;
        }

        let exp_diff = self.exp - other.exp;

        // If difference > 53 bits, smaller value is negligible
        if exp_diff > 53 {
            return *self;
        }
        if exp_diff < -53 {
            return *other;
        }

        // Align to larger exponent, add mantissas
        let (mantissa, exp) = if exp_diff >= 0 {
            let scaled_other = other.mantissa * libm::exp2(-exp_diff as f64);
            (self.mantissa + scaled_other, self.exp)
        } else {
            let scaled_self = self.mantissa * libm::exp2(exp_diff as f64);
            (scaled_self + other.mantissa, other.exp)
        };

        Self { mantissa, exp }.normalize()
    }

    /// Subtract other from self.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Negate value.
    pub fn neg(&self) -> Self {
        Self {
            mantissa: -self.mantissa,
            exp: self.exp,
        }
    }

    /// Squared magnitude of complex number (re, im).
    /// Returns f64 since result is bounded for escape testing (|z|² compared to 4).
    pub fn norm_sq(re: &FloatExp, im: &FloatExp) -> f64 {
        let re_sq = re.mul(re);
        let im_sq = im.mul(im);
        re_sq.add(&im_sq).to_f64()
    }

    /// Normalize mantissa to [0.5, 1.0).
    fn normalize(self) -> Self {
        if self.mantissa == 0.0 {
            return Self::zero();
        }
        let (m, e) = libm::frexp(self.mantissa);
        Self {
            mantissa: m,
            exp: self.exp + e as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_zero() {
        let z = FloatExp::zero();
        assert_eq!(z.to_f64(), 0.0);
        assert!(z.is_zero());
    }

    #[test]
    fn from_f64_preserves_value() {
        let values = [1.0, -1.0, 0.5, 2.0, 1e10, 1e-10, -std::f64::consts::PI];
        for v in values {
            let fe = FloatExp::from_f64(v);
            let back = fe.to_f64();
            assert!(
                (back - v).abs() < 1e-14 * v.abs().max(1.0),
                "from_f64({}) -> to_f64() = {}, expected {}",
                v,
                back,
                v
            );
        }
    }

    #[test]
    fn from_f64_zero_gives_zero() {
        let fe = FloatExp::from_f64(0.0);
        assert!(fe.is_zero());
        assert_eq!(fe.to_f64(), 0.0);
    }

    #[test]
    fn mantissa_normalized_to_half_one() {
        // Non-zero values should have mantissa in [0.5, 1.0) or (-1.0, -0.5]
        let values = [1.0, 2.0, 0.25, 100.0, 0.001];
        for v in values {
            let fe = FloatExp::from_f64(v);
            let m = fe.mantissa.abs();
            assert!(
                (0.5..1.0).contains(&m) || fe.mantissa == 0.0,
                "mantissa {} not normalized for input {}",
                fe.mantissa,
                v
            );
        }
    }

    #[test]
    fn mul_basic() {
        let a = FloatExp::from_f64(2.0);
        let b = FloatExp::from_f64(3.0);
        let c = a.mul(&b);
        assert!((c.to_f64() - 6.0).abs() < 1e-14);
    }

    #[test]
    fn mul_by_zero() {
        let a = FloatExp::from_f64(5.0);
        let z = FloatExp::zero();
        assert!(a.mul(&z).is_zero());
        assert!(z.mul(&a).is_zero());
    }

    #[test]
    fn mul_negative() {
        let a = FloatExp::from_f64(-2.0);
        let b = FloatExp::from_f64(3.0);
        assert!((a.mul(&b).to_f64() - (-6.0)).abs() < 1e-14);
    }

    #[test]
    fn mul_small_values() {
        let a = FloatExp::from_f64(1e-100);
        let b = FloatExp::from_f64(1e-100);
        let c = a.mul(&b);
        // Result is 1e-200, well within FloatExp range
        assert!((c.to_f64() - 1e-200).abs() < 1e-214);
    }

    #[test]
    fn add_basic() {
        let a = FloatExp::from_f64(2.0);
        let b = FloatExp::from_f64(3.0);
        assert!((a.add(&b).to_f64() - 5.0).abs() < 1e-14);
    }

    #[test]
    fn add_zero() {
        let a = FloatExp::from_f64(5.0);
        let z = FloatExp::zero();
        assert!((a.add(&z).to_f64() - 5.0).abs() < 1e-14);
        assert!((z.add(&a).to_f64() - 5.0).abs() < 1e-14);
    }

    #[test]
    fn add_different_magnitudes() {
        // Adding 1e10 + 1e-10 should be approximately 1e10
        let big = FloatExp::from_f64(1e10);
        let small = FloatExp::from_f64(1e-10);
        let sum = big.add(&small);
        assert!((sum.to_f64() - 1e10).abs() < 1.0); // Small value negligible
    }

    #[test]
    fn add_very_different_exponents_returns_larger() {
        // When exponent difference > 53, smaller value is negligible
        let big = FloatExp::from_f64(1.0);
        let tiny = FloatExp {
            mantissa: 0.5,
            exp: -100,
        }; // 2^-101
        let sum = big.add(&tiny);
        assert!((sum.to_f64() - 1.0).abs() < 1e-14);
    }

    #[test]
    fn sub_basic() {
        let a = FloatExp::from_f64(5.0);
        let b = FloatExp::from_f64(3.0);
        assert!((a.sub(&b).to_f64() - 2.0).abs() < 1e-14);
    }

    #[test]
    fn neg_basic() {
        let a = FloatExp::from_f64(5.0);
        assert!((a.neg().to_f64() - (-5.0)).abs() < 1e-14);
    }

    #[test]
    fn norm_sq_basic() {
        // |3 + 4i|² = 9 + 16 = 25
        let re = FloatExp::from_f64(3.0);
        let im = FloatExp::from_f64(4.0);
        let norm = FloatExp::norm_sq(&re, &im);
        assert!((norm - 25.0).abs() < 1e-14);
    }

    #[test]
    fn norm_sq_zero() {
        let z = FloatExp::zero();
        assert_eq!(FloatExp::norm_sq(&z, &z), 0.0);
    }

    #[test]
    fn norm_sq_pure_real() {
        let re = FloatExp::from_f64(5.0);
        let im = FloatExp::zero();
        assert!((FloatExp::norm_sq(&re, &im) - 25.0).abs() < 1e-14);
    }
}
