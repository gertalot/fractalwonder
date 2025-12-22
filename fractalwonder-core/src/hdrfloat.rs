//! High Dynamic Range Float: ~48-bit mantissa precision with extended exponent.
//!
//! Uses double-single arithmetic where the value = (head + tail) × 2^exp.
//! This provides ~48 bits of mantissa precision using two f32 values,
//! enabling deep GPU zoom without f64 dependency.

use crate::BigFloat;

/// High Dynamic Range Float with ~48-bit mantissa precision.
/// Value = (head + tail) × 2^exp
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HDRFloat {
    /// Primary mantissa, normalized to [0.5, 1.0)
    pub head: f32,
    /// Error term, |tail| ≤ 0.5 × ulp(head)
    pub tail: f32,
    /// Binary exponent (base 2)
    pub exp: i32,
}

/// Complex number using HDRFloat components.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HDRComplex {
    pub re: HDRFloat,
    pub im: HDRFloat,
}

impl HDRFloat {
    /// Zero constant.
    pub const ZERO: Self = Self {
        head: 0.0,
        tail: 0.0,
        exp: 0,
    };

    /// Check if value is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.head == 0.0
    }

    /// Create from f32 value.
    pub fn from_f32(val: f32) -> Self {
        if val == 0.0 {
            return Self::ZERO;
        }
        // Extract mantissa and exponent using bit manipulation
        let bits = val.to_bits();
        let sign = bits & 0x8000_0000;
        let biased_exp = ((bits >> 23) & 0xFF) as i32;

        if biased_exp == 0 {
            // Subnormal - handle via normalize
            return Self {
                head: val,
                tail: 0.0,
                exp: 0,
            }
            .normalize();
        }

        // Normal number: extract exponent, set mantissa to [0.5, 1.0)
        let exp = biased_exp - 126; // -126 gives [0.5, 1.0) range
        let mantissa_bits = (bits & 0x007F_FFFF) | 0x3F00_0000 | sign;
        let head = f32::from_bits(mantissa_bits);

        Self {
            head,
            tail: 0.0,
            exp,
        }
    }

    /// Convert to f32 (may lose precision or overflow/underflow).
    pub fn to_f32(&self) -> f32 {
        if self.head == 0.0 {
            return 0.0;
        }
        let mantissa = self.head + self.tail;
        // Handle extreme exponents
        if self.exp > 127 {
            return if mantissa > 0.0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            };
        }
        if self.exp < -149 {
            return 0.0;
        }
        mantissa * exp2_i32(self.exp)
    }

    /// Create from f64, splitting into head + tail for ~48-bit precision.
    pub fn from_f64(val: f64) -> Self {
        if val == 0.0 {
            return Self::ZERO;
        }

        // Extract mantissa and exponent from f64
        let (mantissa, exp) = frexp_f64(val);

        // Split 53-bit mantissa into head (24 bits) + tail (remaining ~29 bits)
        let head = mantissa as f32;
        let tail = (mantissa - head as f64) as f32;

        Self { head, tail, exp }.normalize()
    }

    /// Convert to f64.
    pub fn to_f64(&self) -> f64 {
        if self.head == 0.0 {
            return 0.0;
        }
        let mantissa = self.head as f64 + self.tail as f64;
        libm::ldexp(mantissa, self.exp)
    }

    /// Convert from BigFloat, preserving ~48 bits of mantissa precision.
    pub fn from_bigfloat(bf: &BigFloat) -> Self {
        if bf.to_f64() == 0.0 && bf.log2_approx() == f64::NEG_INFINITY {
            return Self::ZERO;
        }

        // Get approximate log2 to determine exponent
        let log2_approx = bf.log2_approx();
        if !log2_approx.is_finite() {
            return Self::ZERO;
        }

        // Binary exponent (rounded)
        let exp = log2_approx.round() as i32;

        // Scale to [0.5, 2.0) range
        let mantissa_f64 = if exp.abs() < 1000 {
            // Fast path: exponent within f64 range
            let scale = libm::exp2(-exp as f64);
            bf.to_f64() * scale
        } else {
            // Slow path: compute via log2
            // mantissa = 2^(log2(bf) - exp)
            let mantissa_log2 = log2_approx - exp as f64;
            libm::exp2(mantissa_log2)
        };

        // Split f64 mantissa into head + tail
        let head = mantissa_f64 as f32;
        let tail = (mantissa_f64 - head as f64) as f32;

        Self { head, tail, exp }.normalize()
    }

    /// Normalize head to [0.5, 1.0) range.
    #[inline]
    pub fn normalize(self) -> Self {
        if self.head == 0.0 {
            // If head is zero but tail is not, promote tail to head
            if self.tail != 0.0 {
                return Self {
                    head: self.tail,
                    tail: 0.0,
                    exp: self.exp,
                }
                .normalize();
            }
            return Self::ZERO;
        }

        let abs_head = self.head.abs();
        // Fast path: already in [0.5, 1.0)
        if (0.5..1.0).contains(&abs_head) {
            return self;
        }

        // Extract exponent via bit manipulation
        let bits = self.head.to_bits();
        let sign = bits & 0x8000_0000;
        let biased_exp = ((bits >> 23) & 0xFF) as i32;

        if biased_exp == 0 {
            // Subnormal: use slower path
            let (m, e) = frexp_f32(self.head);
            let scale = exp2_i32(-e);
            return Self {
                head: m,
                tail: self.tail * scale,
                exp: self.exp.saturating_add(e),
            };
        }

        // Normal: adjust to [0.5, 1.0) range
        let exp_adjust = biased_exp - 126;
        let new_mantissa_bits = (bits & 0x807F_FFFF) | 0x3F00_0000;
        let new_head = f32::from_bits(new_mantissa_bits | sign);
        let scale = exp2_i32(-exp_adjust);
        let new_tail = self.tail * scale;

        Self {
            head: new_head,
            tail: new_tail,
            exp: self.exp.saturating_add(exp_adjust),
        }
    }

    /// Multiply two HDRFloat values with error tracking.
    #[inline]
    pub fn mul(&self, other: &Self) -> Self {
        if self.head == 0.0 || other.head == 0.0 {
            return Self::ZERO;
        }

        // Primary product
        let p = self.head * other.head;

        // Error from primary product using FMA: err = fma(a, b, -p) = a*b - p
        let err = self.head.mul_add(other.head, -p);

        // Cross terms: h1·t2 + t1·h2 (t1·t2 is negligible)
        let tail = err + self.head * other.tail + self.tail * other.head;

        Self {
            head: p,
            tail,
            exp: self.exp.saturating_add(other.exp),
        }
        .normalize()
    }

    /// Square value (optimized: fewer operations than mul).
    #[inline]
    pub fn square(&self) -> Self {
        if self.head == 0.0 {
            return Self::ZERO;
        }

        let p = self.head * self.head;
        let err = self.head.mul_add(self.head, -p);
        let tail = err + 2.0 * self.head * self.tail;

        Self {
            head: p,
            tail,
            exp: self.exp.saturating_mul(2),
        }
        .normalize()
    }

    /// Add two HDRFloat values with error tracking.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        if self.head == 0.0 {
            return *other;
        }
        if other.head == 0.0 {
            return *self;
        }

        // Use saturating subtraction to prevent overflow with extreme exponents
        let exp_diff = self.exp.saturating_sub(other.exp);

        // If difference > ~48 bits, smaller value is negligible
        // Also catches saturated values (i32::MAX or i32::MIN)
        if exp_diff > 48 {
            return *self;
        }
        if exp_diff < -48 {
            return *other;
        }

        // Align to larger exponent
        let (a_head, a_tail, b_head, b_tail, result_exp) = if exp_diff >= 0 {
            let scale = exp2_i32(-exp_diff);
            (
                self.head,
                self.tail,
                other.head * scale,
                other.tail * scale,
                self.exp,
            )
        } else {
            let scale = exp2_i32(exp_diff);
            (
                self.head * scale,
                self.tail * scale,
                other.head,
                other.tail,
                other.exp,
            )
        };

        // Two-sum: error-free addition of heads
        let sum = a_head + b_head;
        let err = two_sum_err(a_head, b_head, sum);

        // Combine tails with error term
        let tail = err + a_tail + b_tail;

        Self {
            head: sum,
            tail,
            exp: result_exp,
        }
        .normalize()
    }

    /// Subtract other from self.
    #[inline]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Negate value.
    #[inline]
    pub fn neg(&self) -> Self {
        Self {
            head: -self.head,
            tail: -self.tail,
            exp: self.exp,
        }
    }

    /// Multiply by f64 scalar (for 2·Z·δz where Z is f64 reference orbit value).
    #[inline]
    pub fn mul_f64(&self, scalar: f64) -> Self {
        if self.head == 0.0 || scalar == 0.0 {
            return Self::ZERO;
        }

        // Split scalar into head + tail
        let s_head = scalar as f32;
        let s_tail = (scalar - s_head as f64) as f32;

        // Full product with error tracking
        let p = self.head * s_head;
        let err = self.head.mul_add(s_head, -p);
        let tail = err + self.head * s_tail + self.tail * s_head;

        Self {
            head: p,
            tail,
            exp: self.exp,
        }
        .normalize()
    }

    /// Divide by f64 divisor (for computing pixel step = viewport_size / image_dimension).
    /// This preserves extended exponent range, unlike `to_f64() / divisor`.
    #[inline]
    pub fn div_f64(&self, divisor: f64) -> Self {
        if self.head == 0.0 {
            return Self::ZERO;
        }
        if divisor == 0.0 {
            // Division by zero: return infinity-like value
            return Self {
                head: if self.head > 0.0 {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                },
                tail: 0.0,
                exp: 0,
            };
        }

        // Extract mantissa and exponent from divisor: divisor = div_mantissa * 2^div_exp
        let (div_mantissa, div_exp) = frexp_f64(divisor);

        // Compute quotient in f64 for better precision, then split back
        // mantissa = (head + tail) / div_mantissa
        let self_mantissa = self.head as f64 + self.tail as f64;
        let quotient = self_mantissa / div_mantissa;

        // Split quotient into head + tail
        let q_head = quotient as f32;
        let q_tail = (quotient - q_head as f64) as f32;

        Self {
            head: q_head,
            tail: q_tail,
            exp: self.exp.saturating_sub(div_exp),
        }
        .normalize()
    }
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

    /// Check if zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
}

use crate::ComplexDelta;

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

/// Compute 2^n for integer n within f32 exponent range.
#[inline]
fn exp2_i32(n: i32) -> f32 {
    if n < -149 {
        return 0.0;
    }
    if n > 127 {
        return f32::INFINITY;
    }
    if n >= -126 {
        // Normal range
        f32::from_bits(((n + 127) as u32) << 23)
    } else {
        // Subnormal range
        f32::from_bits(1u32 << (n + 149))
    }
}

/// Extract mantissa and exponent: val = mantissa × 2^exp, mantissa in [0.5, 1.0)
#[inline]
fn frexp_f32(val: f32) -> (f32, i32) {
    if val == 0.0 {
        return (0.0, 0);
    }
    let bits = val.to_bits();
    let sign = bits & 0x8000_0000;
    let biased_exp = ((bits >> 23) & 0xFF) as i32;

    if biased_exp == 0 {
        // Subnormal: normalize first
        let normalized = val * (1u64 << 23) as f32;
        let (m, e) = frexp_f32(normalized);
        return (m, e - 23);
    }

    let exp = biased_exp - 126;
    let mantissa_bits = (bits & 0x007F_FFFF) | 0x3F00_0000 | sign;
    (f32::from_bits(mantissa_bits), exp)
}

/// Extract mantissa and exponent from f64: val = mantissa × 2^exp, mantissa in [0.5, 1.0)
#[inline]
fn frexp_f64(val: f64) -> (f64, i32) {
    if val == 0.0 {
        return (0.0, 0);
    }
    let (m, e) = libm::frexp(val);
    (m, e)
}

/// Compute error term from addition: a + b = sum + err (Knuth's two-sum)
#[inline]
fn two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    let b_roundoff = b - b_virtual;
    let a_roundoff = a - a_virtual;
    a_roundoff + b_roundoff
}
