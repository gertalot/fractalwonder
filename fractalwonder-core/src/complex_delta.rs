//! Complex delta types for perturbation arithmetic.
//!
//! Provides a trait abstraction over f64, HDRFloat, and BigFloat complex numbers,
//! enabling a single generic perturbation function with zero runtime overhead.

use crate::BigFloat;

/// Complex number type for perturbation delta arithmetic.
///
/// Abstracts operations needed for perturbation iteration, enabling
/// a single generic implementation for f64, HDRFloat, and BigFloat.
pub trait ComplexDelta: Clone + Sized {
    /// Returns the additive identity (zero) with the same precision as self.
    fn zero(&self) -> Self;

    /// Construct from f64 real/imaginary components.
    fn from_f64_pair(re: f64, im: f64) -> Self;

    /// Extract as f64 pair for output and comparisons.
    fn to_f64_pair(&self) -> (f64, f64);

    /// Complex addition.
    fn add(&self, other: &Self) -> Self;

    /// Complex subtraction.
    fn sub(&self, other: &Self) -> Self;

    /// Complex multiplication.
    fn mul(&self, other: &Self) -> Self;

    /// Multiply by f64 scalar.
    fn scale(&self, factor: f64) -> Self;

    /// Complex square (optimized).
    fn square(&self) -> Self;

    /// Magnitude squared as f64 (for escape/rebase checks).
    fn norm_sq(&self) -> f64;
}

/// Simple f64 complex number for perturbation arithmetic.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct F64Complex {
    pub re: f64,
    pub im: f64,
}

impl ComplexDelta for F64Complex {
    #[inline]
    fn zero(&self) -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    #[inline]
    fn from_f64_pair(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    #[inline]
    fn to_f64_pair(&self) -> (f64, f64) {
        (self.re, self.im)
    }

    #[inline]
    fn add(&self, other: &Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    #[inline]
    fn sub(&self, other: &Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    #[inline]
    fn mul(&self, other: &Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    #[inline]
    fn scale(&self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    #[inline]
    fn square(&self) -> Self {
        Self {
            re: self.re * self.re - self.im * self.im,
            im: 2.0 * self.re * self.im,
        }
    }

    #[inline]
    fn norm_sq(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

/// BigFloat complex number for ultra-deep zoom perturbation.
#[derive(Clone, Debug)]
pub struct BigFloatComplex {
    pub re: BigFloat,
    pub im: BigFloat,
}

impl BigFloatComplex {
    /// Create a new BigFloatComplex from BigFloat components.
    pub fn new(re: BigFloat, im: BigFloat) -> Self {
        Self { re, im }
    }
}

impl ComplexDelta for BigFloatComplex {
    fn zero(&self) -> Self {
        let precision = self.re.precision_bits();
        Self {
            re: BigFloat::zero(precision),
            im: BigFloat::zero(precision),
        }
    }

    fn from_f64_pair(re: f64, im: f64) -> Self {
        // Default to 128-bit precision; actual precision comes from zero() in practice
        Self {
            re: BigFloat::with_precision(re, 128),
            im: BigFloat::with_precision(im, 128),
        }
    }

    fn to_f64_pair(&self) -> (f64, f64) {
        (self.re.to_f64(), self.im.to_f64())
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            re: self.re.add(&other.re),
            im: self.im.add(&other.im),
        }
    }

    fn sub(&self, other: &Self) -> Self {
        Self {
            re: self.re.sub(&other.re),
            im: self.im.sub(&other.im),
        }
    }

    fn mul(&self, other: &Self) -> Self {
        Self {
            re: self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            im: self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        }
    }

    fn scale(&self, factor: f64) -> Self {
        let precision = self.re.precision_bits();
        let scale = BigFloat::with_precision(factor, precision);
        Self {
            re: self.re.mul(&scale),
            im: self.im.mul(&scale),
        }
    }

    fn square(&self) -> Self {
        Self {
            re: self.re.mul(&self.re).sub(&self.im.mul(&self.im)),
            im: self
                .re
                .mul(&self.im)
                .mul(&BigFloat::with_precision(2.0, self.re.precision_bits())),
        }
    }

    fn norm_sq(&self) -> f64 {
        self.re.mul(&self.re).add(&self.im.mul(&self.im)).to_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_complex_zero_returns_origin() {
        let c = F64Complex::from_f64_pair(1.0, 2.0);
        let z = c.zero();
        assert_eq!(z.re, 0.0);
        assert_eq!(z.im, 0.0);
    }

    #[test]
    fn f64_complex_add() {
        let a = F64Complex::from_f64_pair(1.0, 2.0);
        let b = F64Complex::from_f64_pair(3.0, 4.0);
        let c = a.add(&b);
        assert_eq!(c.to_f64_pair(), (4.0, 6.0));
    }

    #[test]
    fn f64_complex_mul() {
        // (1 + 2i) * (3 + 4i) = 3 + 4i + 6i + 8i² = 3 + 10i - 8 = -5 + 10i
        let a = F64Complex::from_f64_pair(1.0, 2.0);
        let b = F64Complex::from_f64_pair(3.0, 4.0);
        let c = a.mul(&b);
        assert_eq!(c.to_f64_pair(), (-5.0, 10.0));
    }

    #[test]
    fn f64_complex_square() {
        // (3 + 4i)² = 9 + 24i + 16i² = 9 + 24i - 16 = -7 + 24i
        let a = F64Complex::from_f64_pair(3.0, 4.0);
        let b = a.square();
        assert_eq!(b.to_f64_pair(), (-7.0, 24.0));
    }

    #[test]
    fn f64_complex_norm_sq() {
        // |3 + 4i|² = 9 + 16 = 25
        let a = F64Complex::from_f64_pair(3.0, 4.0);
        assert_eq!(a.norm_sq(), 25.0);
    }

    #[test]
    fn f64_complex_scale() {
        let a = F64Complex::from_f64_pair(1.0, 2.0);
        let b = a.scale(3.0);
        assert_eq!(b.to_f64_pair(), (3.0, 6.0));
    }

    #[test]
    fn f64_complex_sub() {
        let a = F64Complex::from_f64_pair(5.0, 7.0);
        let b = F64Complex::from_f64_pair(2.0, 3.0);
        let c = a.sub(&b);
        assert_eq!(c.to_f64_pair(), (3.0, 4.0));
    }

    #[test]
    fn bigfloat_complex_zero_preserves_precision() {
        use crate::BigFloat;
        let a = BigFloatComplex::new(
            BigFloat::with_precision(1.0, 256),
            BigFloat::with_precision(2.0, 256),
        );
        let z = a.zero();
        assert_eq!(z.re.to_f64(), 0.0);
        assert_eq!(z.im.to_f64(), 0.0);
        assert_eq!(z.re.precision_bits(), 256);
    }

    #[test]
    fn bigfloat_complex_add() {
        use crate::BigFloat;
        let a = BigFloatComplex::new(
            BigFloat::with_precision(1.0, 128),
            BigFloat::with_precision(2.0, 128),
        );
        let b = BigFloatComplex::new(
            BigFloat::with_precision(3.0, 128),
            BigFloat::with_precision(4.0, 128),
        );
        let c = a.add(&b);
        let (re, im) = c.to_f64_pair();
        assert!((re - 4.0).abs() < 1e-10);
        assert!((im - 6.0).abs() < 1e-10);
    }

    #[test]
    fn bigfloat_complex_mul() {
        use crate::BigFloat;
        // (1 + 2i) * (3 + 4i) = -5 + 10i
        let a = BigFloatComplex::new(
            BigFloat::with_precision(1.0, 128),
            BigFloat::with_precision(2.0, 128),
        );
        let b = BigFloatComplex::new(
            BigFloat::with_precision(3.0, 128),
            BigFloat::with_precision(4.0, 128),
        );
        let c = a.mul(&b);
        let (re, im) = c.to_f64_pair();
        assert!((re - (-5.0)).abs() < 1e-10);
        assert!((im - 10.0).abs() < 1e-10);
    }

    #[test]
    fn bigfloat_complex_norm_sq() {
        use crate::BigFloat;
        // |3 + 4i|² = 25
        let a = BigFloatComplex::new(
            BigFloat::with_precision(3.0, 128),
            BigFloat::with_precision(4.0, 128),
        );
        let norm = a.norm_sq();
        assert!((norm - 25.0).abs() < 1e-10);
    }
}
