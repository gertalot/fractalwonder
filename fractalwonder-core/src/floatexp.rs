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
}
