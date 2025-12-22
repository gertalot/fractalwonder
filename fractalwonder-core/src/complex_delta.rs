//! Complex delta types for perturbation arithmetic.
//!
//! Provides a trait abstraction over f64, HDRFloat, and BigFloat complex numbers,
//! enabling a single generic perturbation function with zero runtime overhead.

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
}
