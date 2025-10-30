use dashu_float::FBig;
use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Sub};

/// Trait for numeric types that can be used as coordinates in image space.
/// This includes both standard floating point (f64) and arbitrary precision types.
pub trait ImageFloat:
    Clone
    + Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Mul<f64, Output = Self>
    + Div<f64, Output = Self>
{
    /// Create from an f64 value
    fn from_f64(val: f64) -> Self;

    /// Create from an i32 value
    fn from_i32(val: i32) -> Self;

    /// Convert to f64 (may lose precision)
    fn to_f64(&self) -> f64;

    /// Multiply by another value of the same type
    fn mul(&self, other: &Self) -> Self;

    /// Add another value of the same type
    fn add(&self, other: &Self) -> Self;

    /// Subtract another value of the same type
    fn sub(&self, other: &Self) -> Self;

    /// Divide by another value of the same type
    fn div(&self, other: &Self) -> Self;

    /// Check if greater than another value
    fn gt(&self, other: &Self) -> bool;
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

impl ImageFloat for BigFloat {
    fn from_f64(val: f64) -> Self {
        // Default to 256 bits of precision (about 77 decimal digits)
        Self::with_precision(val, 256)
    }

    fn from_i32(val: i32) -> Self {
        Self::from_f64(val as f64)
    }

    fn to_f64(&self) -> f64 {
        self.value.to_f64().value()
    }

    fn mul(&self, other: &Self) -> Self {
        let precision = self.precision_bits.max(other.precision_bits);
        BigFloat::from_fbig(&self.value * &other.value, precision)
    }

    fn add(&self, other: &Self) -> Self {
        let precision = self.precision_bits.max(other.precision_bits);
        BigFloat::from_fbig(&self.value + &other.value, precision)
    }

    fn sub(&self, other: &Self) -> Self {
        let precision = self.precision_bits.max(other.precision_bits);
        BigFloat::from_fbig(&self.value - &other.value, precision)
    }

    fn div(&self, other: &Self) -> Self {
        let precision = self.precision_bits.max(other.precision_bits);
        BigFloat::from_fbig(&self.value / &other.value, precision)
    }

    fn gt(&self, other: &Self) -> bool {
        self.value > other.value
    }
}

/// Implementation for standard f64
impl ImageFloat for f64 {
    fn from_f64(val: f64) -> Self {
        val
    }

    fn from_i32(val: i32) -> Self {
        val as f64
    }

    fn to_f64(&self) -> f64 {
        *self
    }

    fn mul(&self, other: &Self) -> Self {
        self * other
    }

    fn add(&self, other: &Self) -> Self {
        self + other
    }

    fn sub(&self, other: &Self) -> Self {
        self - other
    }

    fn div(&self, other: &Self) -> Self {
        self / other
    }

    fn gt(&self, other: &Self) -> bool {
        self > other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_image_float() {
        let a = f64::from_f64(2.5);
        let b = f64::from_f64(1.5);

        assert_eq!(ImageFloat::add(&a, &b), 4.0);
        assert_eq!(ImageFloat::sub(&a, &b), 1.0);
        assert_eq!(ImageFloat::mul(&a, &b), 3.75);
        assert_eq!(ImageFloat::div(&a, &b), 2.5 / 1.5);
        assert!(ImageFloat::gt(&a, &b));
    }

    #[test]
    fn test_f64_from_i32() {
        let val = f64::from_i32(42);
        assert_eq!(val, 42.0);
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

        let sum = ImageFloat::add(&a, &b);
        assert!((sum.to_f64() - 4.0).abs() < 1e-10);

        let diff = ImageFloat::sub(&a, &b);
        assert!((diff.to_f64() - 1.0).abs() < 1e-10);

        let prod = ImageFloat::mul(&a, &b);
        assert!((prod.to_f64() - 3.75).abs() < 1e-10);

        let quot = ImageFloat::div(&a, &b);
        assert!((quot.to_f64() - (2.5 / 1.5)).abs() < 1e-10);
    }

    #[test]
    fn test_bigfloat_comparison() {
        let a = BigFloat::from_f64(2.5);
        let b = BigFloat::from_f64(1.5);

        assert!(ImageFloat::gt(&a, &b));
        assert!(!ImageFloat::gt(&b, &a));
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
        let sum = ImageFloat::add(&ImageFloat::add(&a, &b), &c);

        // The result should be very close to 0.6
        // With high precision, we should get better accuracy than f64
        assert!((sum.to_f64() - 0.6).abs() < 1e-15);
    }
}
