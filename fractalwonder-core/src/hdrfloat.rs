//! High Dynamic Range Float: ~48-bit mantissa precision with extended exponent.
//!
//! Uses double-single arithmetic where the value = (head + tail) × 2^exp.
//! This provides ~48 bits of mantissa precision using two f32 values,
//! enabling deep GPU zoom without f64 dependency.

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
                exp: self.exp + e,
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
            exp: self.exp + exp_adjust,
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
            exp: self.exp + other.exp,
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
            exp: self.exp * 2,
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

        let exp_diff = self.exp - other.exp;

        // If difference > ~48 bits, smaller value is negligible
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
}

impl HDRComplex {
    /// Zero constant.
    pub const ZERO: Self = Self {
        re: HDRFloat::ZERO,
        im: HDRFloat::ZERO,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_f32_zero_gives_zero() {
        let h = HDRFloat::from_f32(0.0);
        assert!(h.is_zero());
        assert_eq!(h.head, 0.0);
        assert_eq!(h.tail, 0.0);
        assert_eq!(h.exp, 0);
    }

    #[test]
    fn from_f32_one_normalized() {
        let h = HDRFloat::from_f32(1.0);
        assert!(!h.is_zero());
        // 1.0 = 0.5 × 2^1, so head should be 0.5, exp should be 1
        assert!((h.head - 0.5).abs() < 1e-7);
        assert_eq!(h.tail, 0.0);
        assert_eq!(h.exp, 1);
    }

    #[test]
    fn from_f32_preserves_value() {
        let values = [1.0f32, -1.0, 0.5, 2.0, 1e10, 1e-10, -std::f32::consts::PI];
        for v in values {
            let h = HDRFloat::from_f32(v);
            let back = h.to_f32();
            assert!(
                (back - v).abs() < v.abs() * 1e-6 + 1e-38,
                "from_f32({}) -> to_f32() = {}, expected {}",
                v,
                back,
                v
            );
        }
    }

    #[test]
    fn normalize_handles_range_one_to_two() {
        // Values in [1.0, 2.0) should be normalized to [0.5, 1.0)
        let h = HDRFloat {
            head: 1.5,
            tail: 0.0,
            exp: 0,
        };
        let normalized = h.normalize();
        assert!((normalized.head - 0.75).abs() < 1e-7);
        assert_eq!(normalized.exp, 1);
    }

    #[test]
    fn from_f64_captures_more_precision_than_f32() {
        // Value with more precision than f32 can represent
        let val: f64 = 1.0 + 1e-10;
        let h = HDRFloat::from_f64(val);

        // Converting back should preserve more precision than direct f32 cast
        let back = h.to_f64();
        let direct = val as f32 as f64;

        let error_hdr = (back - val).abs();
        let error_direct = (direct - val).abs();

        assert!(
            error_hdr < error_direct,
            "HDRFloat error {} should be less than direct f32 error {}",
            error_hdr,
            error_direct
        );
    }

    #[test]
    fn from_f64_preserves_value() {
        let values = [1.0f64, -1.0, 0.5, 2.0, 1e10, 1e-10, std::f64::consts::PI];
        for v in values {
            let h = HDRFloat::from_f64(v);
            let back = h.to_f64();
            // Should preserve ~48 bits of precision
            assert!(
                (back - v).abs() < v.abs() * 1e-14 + 1e-300,
                "from_f64({}) -> to_f64() = {}, diff = {}",
                v,
                back,
                (back - v).abs()
            );
        }
    }

    #[test]
    fn mul_basic() {
        let a = HDRFloat::from_f64(2.0);
        let b = HDRFloat::from_f64(3.0);
        let c = a.mul(&b);
        assert!((c.to_f64() - 6.0).abs() < 1e-14);
    }

    #[test]
    fn mul_by_zero() {
        let a = HDRFloat::from_f64(5.0);
        let z = HDRFloat::ZERO;
        assert!(a.mul(&z).is_zero());
        assert!(z.mul(&a).is_zero());
    }

    #[test]
    fn mul_small_values() {
        let a = HDRFloat::from_f64(1e-20);
        let b = HDRFloat::from_f64(1e-20);
        let c = a.mul(&b);
        // Result is 1e-40, within HDRFloat range
        assert!((c.to_f64() - 1e-40).abs() < 1e-54);
    }

    #[test]
    fn mul_preserves_precision() {
        // Two values that require full precision
        let a = HDRFloat::from_f64(1.0 + 1e-10);
        let b = HDRFloat::from_f64(1.0 + 2e-10);
        let c = a.mul(&b);
        let expected = (1.0 + 1e-10) * (1.0 + 2e-10);
        assert!(
            (c.to_f64() - expected).abs() < expected * 1e-14,
            "mul precision: got {}, expected {}",
            c.to_f64(),
            expected
        );
    }

    #[test]
    fn add_basic() {
        let a = HDRFloat::from_f64(2.0);
        let b = HDRFloat::from_f64(3.0);
        assert!((a.add(&b).to_f64() - 5.0).abs() < 1e-14);
    }

    #[test]
    fn add_zero() {
        let a = HDRFloat::from_f64(5.0);
        let z = HDRFloat::ZERO;
        assert!((a.add(&z).to_f64() - 5.0).abs() < 1e-14);
        assert!((z.add(&a).to_f64() - 5.0).abs() < 1e-14);
    }

    #[test]
    fn add_different_exponents() {
        // 1e10 + 1e-10 should be approximately 1e10
        let big = HDRFloat::from_f64(1e10);
        let small = HDRFloat::from_f64(1e-10);
        let sum = big.add(&small);
        assert!((sum.to_f64() - 1e10).abs() < 1.0);
    }

    #[test]
    fn add_cancellation() {
        // Test catastrophic cancellation: 1.0 - (1.0 - 1e-10)
        // Note: 1e-15 is beyond f64 precision difference from 1.0, so we use 1e-10
        let a = HDRFloat::from_f64(1.0);
        let b = HDRFloat::from_f64(1.0 - 1e-10);
        let diff = a.sub(&b);
        let expected = 1e-10;
        assert!(
            (diff.to_f64() - expected).abs() < expected * 1e-6,
            "Cancellation: got {}, expected {}",
            diff.to_f64(),
            expected
        );
    }

    #[test]
    fn sub_basic() {
        let a = HDRFloat::from_f64(5.0);
        let b = HDRFloat::from_f64(3.0);
        assert!((a.sub(&b).to_f64() - 2.0).abs() < 1e-14);
    }
}
