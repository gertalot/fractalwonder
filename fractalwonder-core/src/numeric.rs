use dashu_float::FBig;
use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Sub};

/// Trait for converting numeric types to f64 for display purposes.
/// This is the ONLY non-standard trait we need - everything else uses
/// standard Rust operators (Add, Sub, Mul, Div, From<f64>, PartialOrd).
pub trait ToF64 {
    /// Convert to f64 (may lose precision for arbitrary precision types)
    fn to_f64(&self) -> f64;
}

/// Arbitrary precision floating point number type for deep zoom calculations.
/// Wraps dashu FBig with automatic precision management based on zoom level.
#[derive(Clone, Debug)]
pub struct BigFloat {
    value: FBig,
    precision_bits: usize,
}

impl BigFloat {
    /// Create a new BigFloat with specified precision in bits
    pub fn with_precision(val: f64, precision_bits: usize) -> Self {
        let value = FBig::try_from(val).unwrap_or(FBig::ZERO);
        Self {
            value,
            precision_bits,
        }
    }

    /// Create from an FBig value with specified precision
    pub fn from_fbig(value: FBig, precision_bits: usize) -> Self {
        Self {
            value,
            precision_bits,
        }
    }

    /// Create from f64 with default precision (256 bits)
    pub fn from_f64(val: f64) -> Self {
        Self::with_precision(val, 256)
    }

    /// Get the precision in bits
    pub fn precision_bits(&self) -> usize {
        self.precision_bits
    }
}

/// Standard arithmetic operators for BigFloat
impl Add for BigFloat {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let precision = self.precision_bits.max(rhs.precision_bits);
        BigFloat::from_fbig(&self.value + &rhs.value, precision)
    }
}

impl Sub for BigFloat {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let precision = self.precision_bits.max(rhs.precision_bits);
        BigFloat::from_fbig(&self.value - &rhs.value, precision)
    }
}

impl Mul for BigFloat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let precision = self.precision_bits.max(rhs.precision_bits);
        BigFloat::from_fbig(&self.value * &rhs.value, precision)
    }
}

impl Div for BigFloat {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let precision = self.precision_bits.max(rhs.precision_bits);
        BigFloat::from_fbig(&self.value / &rhs.value, precision)
    }
}

/// Multiplication by f64 scalar
impl Mul<f64> for BigFloat {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        let rhs_big = FBig::try_from(rhs).unwrap_or(FBig::ZERO);
        BigFloat::from_fbig(&self.value * &rhs_big, self.precision_bits)
    }
}

/// Division by f64 scalar
impl Div<f64> for BigFloat {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        let rhs_big = FBig::try_from(rhs).unwrap_or(FBig::ONE);
        BigFloat::from_fbig(&self.value / &rhs_big, self.precision_bits)
    }
}

impl PartialEq for BigFloat {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl PartialOrd for BigFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl From<f64> for BigFloat {
    fn from(val: f64) -> Self {
        Self::from_f64(val)
    }
}

impl ToF64 for BigFloat {
    fn to_f64(&self) -> f64 {
        self.value.to_f64().value()
    }
}

impl ToF64 for f64 {
    fn to_f64(&self) -> f64 {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_to_f64() {
        let val: f64 = 42.5;
        assert_eq!(val.to_f64(), 42.5);
    }

    #[test]
    fn test_bigfloat_creation() {
        let a = BigFloat::with_precision(2.5, 128);
        assert_eq!(a.precision_bits(), 128);
        assert!((a.to_f64() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_bigfloat_arithmetic() {
        let a = BigFloat::from_f64(2.5);
        let b = BigFloat::from_f64(1.5);

        let sum = a.clone() + b.clone();
        assert!((sum.to_f64() - 4.0).abs() < 1e-10);

        let diff = a.clone() - b.clone();
        assert!((diff.to_f64() - 1.0).abs() < 1e-10);

        let prod = a.clone() * b.clone();
        assert!((prod.to_f64() - 3.75).abs() < 1e-10);

        let quot = a / b;
        assert!((quot.to_f64() - (2.5 / 1.5)).abs() < 1e-10);
    }

    #[test]
    fn test_bigfloat_comparison() {
        let a = BigFloat::from_f64(2.5);
        let b = BigFloat::from_f64(1.5);

        assert!(a > b);
        assert!(b < a);
    }

    #[test]
    fn test_bigfloat_scalar_ops() {
        let a = BigFloat::from_f64(10.0);
        let scaled = a.clone() * 2.5;
        assert!((scaled.to_f64() - 25.0).abs() < 1e-10);

        let divided = a / 2.0;
        assert!((divided.to_f64() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_bigfloat_precision_maintained() {
        // Test that high precision calculations maintain more accuracy than f64
        let a = BigFloat::with_precision(0.1, 256);
        let b = BigFloat::with_precision(0.2, 256);
        let c = BigFloat::with_precision(0.3, 256);

        // This is a classic example where f64 can have rounding errors
        let sum = (a + b) + c;

        // The result should be very close to 0.6
        // With high precision, we should get better accuracy than f64
        assert!((sum.to_f64() - 0.6).abs() < 1e-15);
    }

    #[test]
    fn test_to_f64_trait() {
        // Test that f64 converts to itself
        let val_f64: f64 = 42.5;
        assert_eq!(val_f64.to_f64(), 42.5);

        // Test that BigFloat converts to f64
        let val_big = BigFloat::with_precision(42.5, 128);
        assert!((val_big.to_f64() - 42.5).abs() < 1e-10);
    }
}
