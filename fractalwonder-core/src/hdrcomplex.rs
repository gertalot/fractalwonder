//! Complex number using HDRFloat components for extended range arithmetic.

use crate::{ComplexDelta, HDRFloat};

/// Complex number using HDRFloat components.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HDRComplex {
    pub re: HDRFloat,
    pub im: HDRFloat,
}

impl HDRComplex {
    /// Zero constant.
    pub const ZERO: Self = Self {
        re: HDRFloat::ZERO,
        im: HDRFloat::ZERO,
    };

    /// Add two complex numbers.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        Self {
            re: self.re.add(&other.re),
            im: self.im.add(&other.im),
        }
    }

    /// Subtract other from self.
    #[inline]
    pub fn sub(&self, other: &Self) -> Self {
        Self {
            re: self.re.sub(&other.re),
            im: self.im.sub(&other.im),
        }
    }

    /// Multiply two complex numbers: (a + bi)(c + di) = (ac - bd) + (ad + bc)i
    #[inline]
    pub fn mul(&self, other: &Self) -> Self {
        Self {
            re: self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            im: self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        }
    }

    /// Square: (a + bi)² = (a² - b²) + 2abi
    #[inline]
    pub fn square(&self) -> Self {
        let re_sq = self.re.square();
        let im_sq = self.im.square();
        let re_im = self.re.mul(&self.im);
        // Multiply by 2 exactly by incrementing exponent (no rounding error)
        let two_re_im = HDRFloat {
            head: re_im.head,
            tail: re_im.tail,
            exp: re_im.exp.saturating_add(1),
        };
        Self {
            re: re_sq.sub(&im_sq),
            im: two_re_im,
        }
    }

    /// Squared magnitude: |z|² = re² + im²
    /// Returns f64 since result is bounded for escape testing.
    #[inline]
    pub fn norm_sq(&self) -> f64 {
        let re_sq = self.re.square();
        let im_sq = self.im.square();
        re_sq.add(&im_sq).to_f64()
    }

    /// Squared magnitude returning HDRFloat (no f64 conversion).
    /// Use this for BLA calculations where values may exceed f64 range.
    #[inline]
    pub fn norm_sq_hdr(&self) -> HDRFloat {
        self.re.square().add(&self.im.square())
    }

    /// Magnitude returning HDRFloat.
    /// Use this for BLA calculations where values may exceed f64 range.
    #[inline]
    pub fn norm_hdr(&self) -> HDRFloat {
        self.norm_sq_hdr().sqrt()
    }

    /// Check if zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
}

impl ComplexDelta for HDRComplex {
    #[inline]
    fn zero(&self) -> Self {
        Self::ZERO
    }

    #[inline]
    fn from_f64_pair(re: f64, im: f64) -> Self {
        Self {
            re: HDRFloat::from_f64(re),
            im: HDRFloat::from_f64(im),
        }
    }

    #[inline]
    fn to_f64_pair(&self) -> (f64, f64) {
        (self.re.to_f64(), self.im.to_f64())
    }

    #[inline]
    fn add(&self, other: &Self) -> Self {
        Self {
            re: self.re.add(&other.re),
            im: self.im.add(&other.im),
        }
    }

    #[inline]
    fn sub(&self, other: &Self) -> Self {
        Self {
            re: self.re.sub(&other.re),
            im: self.im.sub(&other.im),
        }
    }

    #[inline]
    fn mul(&self, other: &Self) -> Self {
        Self {
            re: self.re.mul(&other.re).sub(&self.im.mul(&other.im)),
            im: self.re.mul(&other.im).add(&self.im.mul(&other.re)),
        }
    }

    #[inline]
    fn scale(&self, factor: f64) -> Self {
        Self {
            re: self.re.mul_f64(factor),
            im: self.im.mul_f64(factor),
        }
    }

    #[inline]
    fn square(&self) -> Self {
        Self {
            re: self.re.square().sub(&self.im.square()),
            im: self.re.mul(&self.im).mul_f64(2.0),
        }
    }

    #[inline]
    fn norm_sq(&self) -> f64 {
        self.re.square().add(&self.im.square()).to_f64()
    }
}
