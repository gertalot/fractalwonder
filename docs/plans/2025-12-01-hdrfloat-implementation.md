# HDRFloat Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace FloatExp with HDRFloat (~48-bit precision) across CPU and GPU codebases.

**Architecture:** HDRFloat uses double-single arithmetic (head + tail f32s) with extended i32 exponent. FMA-based error tracking preserves precision through arithmetic operations.

**Tech Stack:** Rust (fractalwonder-core), WGSL shaders (fractalwonder-gpu), bytemuck for GPU buffer transfer.

---

## Task 1: Create HDRFloat Core Module

**Files:**
- Create: `fractalwonder-core/src/hdrfloat.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Create hdrfloat.rs with struct and constants**

```rust
// fractalwonder-core/src/hdrfloat.rs

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
    /// Primary mantissa, normalized to [0.5, 2.0)
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
}

impl HDRComplex {
    /// Zero constant.
    pub const ZERO: Self = Self {
        re: HDRFloat::ZERO,
        im: HDRFloat::ZERO,
    };
}
```

**Step 2: Run cargo check to verify syntax**

```bash
cargo check -p fractalwonder-core
```

Expected: Compiles (module not yet exported)

**Step 3: Export module from lib.rs**

In `fractalwonder-core/src/lib.rs`, add:

```rust
pub mod hdrfloat;
pub use hdrfloat::{HDRComplex, HDRFloat};
```

**Step 4: Run cargo check**

```bash
cargo check -p fractalwonder-core
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add HDRFloat struct with zero constant"
```

---

## Task 2: Add HDRFloat Construction Methods

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for from_f32**

Add to `hdrfloat.rs`:

```rust
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
        let values = [1.0f32, -1.0, 0.5, 2.0, 1e10, 1e-10, -3.14159];
        for v in values {
            let h = HDRFloat::from_f32(v);
            let back = h.to_f32();
            assert!(
                (back - v).abs() < v.abs() * 1e-6 + 1e-38,
                "from_f32({}) -> to_f32() = {}, expected {}",
                v, back, v
            );
        }
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p fractalwonder-core from_f32 -- --nocapture
```

Expected: FAIL - `from_f32` not found

**Step 3: Implement from_f32, to_f32, and normalize**

Add to `impl HDRFloat`:

```rust
impl HDRFloat {
    // ... existing code ...

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
            return Self { head: val, tail: 0.0, exp: 0 }.normalize();
        }

        // Normal number: extract exponent, set mantissa to [0.5, 1.0)
        let exp = biased_exp - 126; // -126 gives [0.5, 1.0) range
        let mantissa_bits = (bits & 0x007F_FFFF) | 0x3F00_0000 | sign;
        let head = f32::from_bits(mantissa_bits);

        Self { head, tail: 0.0, exp }
    }

    /// Convert to f32 (may lose precision or overflow/underflow).
    pub fn to_f32(&self) -> f32 {
        if self.head == 0.0 {
            return 0.0;
        }
        let mantissa = self.head + self.tail;
        // Handle extreme exponents
        if self.exp > 127 {
            return if mantissa > 0.0 { f32::INFINITY } else { f32::NEG_INFINITY };
        }
        if self.exp < -149 {
            return 0.0;
        }
        mantissa * exp2_i32(self.exp)
    }

    /// Normalize head to [0.5, 2.0) range.
    #[inline]
    pub fn normalize(self) -> Self {
        if self.head == 0.0 {
            return Self::ZERO;
        }

        let abs_head = self.head.abs();
        // Fast path: already in [0.5, 2.0)
        if abs_head >= 0.5 && abs_head < 2.0 {
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
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core from_f32 -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat from_f32, to_f32, normalize"
```

---

## Task 3: Add HDRFloat from_f64 (Split Precision)

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for from_f64**

Add to tests:

```rust
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
        error_hdr, error_direct
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
            v, back, (back - v).abs()
        );
    }
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core from_f64 -- --nocapture
```

Expected: FAIL - `from_f64` not found

**Step 3: Implement from_f64 and to_f64**

Add to `impl HDRFloat`:

```rust
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
```

Add helper function:

```rust
/// Extract mantissa and exponent from f64: val = mantissa × 2^exp, mantissa in [0.5, 1.0)
#[inline]
fn frexp_f64(val: f64) -> (f64, i32) {
    if val == 0.0 {
        return (0.0, 0);
    }
    let (m, e) = libm::frexp(val);
    (m, e as i32)
}
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core from_f64 -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat from_f64, to_f64 with precision split"
```

---

## Task 4: Add HDRFloat Multiplication

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for mul**

Add to tests:

```rust
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
        "mul precision: got {}, expected {}", c.to_f64(), expected
    );
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core mul -- --nocapture
```

Expected: FAIL - `mul` not found

**Step 3: Implement mul and square**

Add to `impl HDRFloat`:

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core mul -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat mul and square with FMA error tracking"
```

---

## Task 5: Add HDRFloat Addition

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for add**

Add to tests:

```rust
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
    // Test catastrophic cancellation: 1.0 - (1.0 - 1e-15)
    let a = HDRFloat::from_f64(1.0);
    let b = HDRFloat::from_f64(1.0 - 1e-15);
    let diff = a.sub(&b);
    assert!(
        (diff.to_f64() - 1e-15).abs() < 1e-29,
        "Cancellation: got {}, expected 1e-15", diff.to_f64()
    );
}

#[test]
fn sub_basic() {
    let a = HDRFloat::from_f64(5.0);
    let b = HDRFloat::from_f64(3.0);
    assert!((a.sub(&b).to_f64() - 2.0).abs() < 1e-14);
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core add -- --nocapture
```

Expected: FAIL - `add` not found

**Step 3: Implement add, sub, neg**

Add to `impl HDRFloat`:

```rust
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
        (self.head, self.tail, other.head * scale, other.tail * scale, self.exp)
    } else {
        let scale = exp2_i32(exp_diff);
        (self.head * scale, self.tail * scale, other.head, other.tail, other.exp)
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
```

Add helper function:

```rust
/// Compute error term from addition: a + b = sum + err (Knuth's two-sum)
#[inline]
fn two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    let b_roundoff = b - b_virtual;
    let a_roundoff = a - a_virtual;
    a_roundoff + b_roundoff
}
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core add sub -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat add, sub, neg with two-sum error tracking"
```

---

## Task 6: Add BigFloat Conversion

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for from_bigfloat**

Add to tests:

```rust
#[test]
fn from_bigfloat_f64_range() {
    let bf = BigFloat::with_precision(1.234567, 128);
    let h = HDRFloat::from_bigfloat(&bf);
    assert!((h.to_f64() - 1.234567).abs() < 1e-10);
}

#[test]
fn from_bigfloat_zero() {
    let bf = BigFloat::zero(128);
    let h = HDRFloat::from_bigfloat(&bf);
    assert!(h.is_zero());
}

#[test]
fn from_bigfloat_extreme_small() {
    // 10^-100 is beyond f64 range but within HDRFloat
    let bf = BigFloat::from_string("1e-100", 512).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    assert!(!h.is_zero(), "Should not underflow to zero");
    // Exponent should be approximately -100 * log2(10) ≈ -332
    assert!(h.exp < -300, "Exponent {} should be < -300", h.exp);
    assert!(h.exp > -400, "Exponent {} should be > -400", h.exp);
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core from_bigfloat -- --nocapture
```

Expected: FAIL - `from_bigfloat` not found

**Step 3: Implement from_bigfloat**

Add to `impl HDRFloat`:

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core from_bigfloat -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat from_bigfloat conversion"
```

---

## Task 7: Add HDRComplex Operations

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write tests for HDRComplex**

Add to tests:

```rust
#[test]
fn complex_add() {
    let a = HDRComplex {
        re: HDRFloat::from_f64(1.0),
        im: HDRFloat::from_f64(2.0),
    };
    let b = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let c = a.add(&b);
    assert!((c.re.to_f64() - 4.0).abs() < 1e-14);
    assert!((c.im.to_f64() - 6.0).abs() < 1e-14);
}

#[test]
fn complex_mul() {
    // (1 + 2i) * (3 + 4i) = (1*3 - 2*4) + (1*4 + 2*3)i = -5 + 10i
    let a = HDRComplex {
        re: HDRFloat::from_f64(1.0),
        im: HDRFloat::from_f64(2.0),
    };
    let b = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let c = a.mul(&b);
    assert!((c.re.to_f64() - (-5.0)).abs() < 1e-14);
    assert!((c.im.to_f64() - 10.0).abs() < 1e-14);
}

#[test]
fn complex_norm_sq() {
    // |3 + 4i|² = 9 + 16 = 25
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    assert!((c.norm_sq() - 25.0).abs() < 1e-14);
}

#[test]
fn complex_square() {
    // (3 + 4i)² = 9 - 16 + 24i = -7 + 24i
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let sq = c.square();
    assert!((sq.re.to_f64() - (-7.0)).abs() < 1e-14);
    assert!((sq.im.to_f64() - 24.0).abs() < 1e-14);
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core complex -- --nocapture
```

Expected: FAIL - methods not found

**Step 3: Implement HDRComplex methods**

Add to `impl HDRComplex`:

```rust
impl HDRComplex {
    // ... existing ZERO constant ...

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
        let two_re_im = self.re.mul(&self.im);
        Self {
            re: re_sq.sub(&im_sq),
            im: two_re_im.add(&two_re_im), // 2 * re * im
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
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core complex -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRComplex arithmetic operations"
```

---

## Task 8: Add HDRFloat mul_f64 for Reference Orbit

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`

**Step 1: Write test for mul_f64**

Add to tests:

```rust
#[test]
fn mul_f64_basic() {
    let h = HDRFloat::from_f64(2.0);
    let result = h.mul_f64(3.0);
    assert!((result.to_f64() - 6.0).abs() < 1e-14);
}

#[test]
fn mul_f64_preserves_precision() {
    // Multiply HDRFloat by f64 reference orbit value
    let h = HDRFloat::from_f64(1e-50);
    let z_m = 0.123456789012345; // Reference orbit value
    let result = h.mul_f64(z_m);
    let expected = 1e-50 * z_m;
    assert!(
        (result.to_f64() - expected).abs() < expected.abs() * 1e-14,
        "mul_f64: got {}, expected {}", result.to_f64(), expected
    );
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test -p fractalwonder-core mul_f64 -- --nocapture
```

Expected: FAIL

**Step 3: Implement mul_f64**

Add to `impl HDRFloat`:

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p fractalwonder-core mul_f64 -- --nocapture
```

Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): add HDRFloat mul_f64 for reference orbit multiplication"
```

---

## Task 9: Create WGSL HDRFloat Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/hdrfloat.wgsl`

**Step 1: Create the shader file**

```wgsl
// HDRFloat: High Dynamic Range Float for GPU.
// Value = (head + tail) × 2^exp
// Provides ~48-bit mantissa precision using two f32 values.

struct HDRFloat {
    head: f32,  // Primary mantissa [0.5, 2.0)
    tail: f32,  // Error term
    exp: i32,   // Binary exponent
}

struct HDRComplex {
    re: HDRFloat,
    im: HDRFloat,
}

const HDR_ZERO: HDRFloat = HDRFloat(0.0, 0.0, 0);
const HDR_COMPLEX_ZERO: HDRComplex = HDRComplex(HDRFloat(0.0, 0.0, 0), HDRFloat(0.0, 0.0, 0));

// Compute 2^n for integer n
fn hdr_exp2(n: i32) -> f32 {
    if n < -149 { return 0.0; }
    if n > 127 { return bitcast<f32>(0x7F800000u); } // +inf
    if n >= -126 {
        return bitcast<f32>(u32(n + 127) << 23u);
    }
    return bitcast<f32>(1u << u32(n + 149));
}

// Two-sum error computation (Knuth)
fn hdr_two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    return (a - a_virtual) + (b - b_virtual);
}

// Normalize head to [0.5, 2.0)
fn hdr_normalize(x: HDRFloat) -> HDRFloat {
    if x.head == 0.0 { return HDR_ZERO; }

    let abs_head = abs(x.head);
    if abs_head >= 0.5 && abs_head < 2.0 {
        return x;
    }

    let bits = bitcast<u32>(x.head);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    let exp_adjust = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let new_head = bitcast<f32>(new_mantissa_bits | sign);
    let scale = hdr_exp2(-exp_adjust);
    let new_tail = x.tail * scale;

    return HDRFloat(new_head, new_tail, x.exp + exp_adjust);
}

// Negate
fn hdr_neg(a: HDRFloat) -> HDRFloat {
    return HDRFloat(-a.head, -a.tail, a.exp);
}

// Multiply with FMA error tracking
fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 || b.head == 0.0 { return HDR_ZERO; }

    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;

    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));
}

// Square (optimized)
fn hdr_square(a: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return HDR_ZERO; }

    let p = a.head * a.head;
    let err = fma(a.head, a.head, -p);
    let tail = err + 2.0 * a.head * a.tail;

    return hdr_normalize(HDRFloat(p, tail, a.exp * 2));
}

// Add with exponent alignment
fn hdr_add(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return b; }
    if b.head == 0.0 { return a; }

    let exp_diff = a.exp - b.exp;
    if exp_diff > 48 { return a; }
    if exp_diff < -48 { return b; }

    var ah: f32; var at: f32; var bh: f32; var bt: f32; var result_exp: i32;

    if exp_diff >= 0 {
        let scale = hdr_exp2(-exp_diff);
        ah = a.head; at = a.tail;
        bh = b.head * scale; bt = b.tail * scale;
        result_exp = a.exp;
    } else {
        let scale = hdr_exp2(exp_diff);
        ah = a.head * scale; at = a.tail * scale;
        bh = b.head; bt = b.tail;
        result_exp = b.exp;
    }

    let sum = ah + bh;
    let err = hdr_two_sum_err(ah, bh, sum);
    let tail = err + at + bt;

    return hdr_normalize(HDRFloat(sum, tail, result_exp));
}

// Subtract
fn hdr_sub(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    return hdr_add(a, hdr_neg(b));
}

// Multiply HDRFloat by f32 (for reference orbit values)
fn hdr_mul_f32(a: HDRFloat, b: f32) -> HDRFloat {
    if a.head == 0.0 || b == 0.0 { return HDR_ZERO; }

    let p = a.head * b;
    let err = fma(a.head, b, -p);
    let tail = err + a.tail * b;

    return hdr_normalize(HDRFloat(p, tail, a.exp));
}

// Convert HDRFloat to f32 (for escape check)
fn hdr_to_f32(x: HDRFloat) -> f32 {
    if x.head == 0.0 { return 0.0; }
    let mantissa = x.head + x.tail;
    let clamped_exp = clamp(x.exp, -126, 127);
    return mantissa * hdr_exp2(clamped_exp);
}

// Complex multiplication
fn hdr_complex_mul(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(
        hdr_sub(hdr_mul(a.re, b.re), hdr_mul(a.im, b.im)),
        hdr_add(hdr_mul(a.re, b.im), hdr_mul(a.im, b.re))
    );
}

// Complex addition
fn hdr_complex_add(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(hdr_add(a.re, b.re), hdr_add(a.im, b.im));
}

// Complex subtraction
fn hdr_complex_sub(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(hdr_sub(a.re, b.re), hdr_sub(a.im, b.im));
}

// Complex square: (a + bi)² = (a² - b²) + 2abi
fn hdr_complex_square(a: HDRComplex) -> HDRComplex {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let two_re_im = hdr_mul(a.re, a.im);
    return HDRComplex(
        hdr_sub(re_sq, im_sq),
        hdr_add(two_re_im, two_re_im)
    );
}

// Complex squared magnitude |a|² = re² + im²
fn hdr_complex_norm_sq(a: HDRComplex) -> f32 {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let sum = hdr_add(re_sq, im_sq);
    return hdr_to_f32(sum);
}

// Create HDRFloat from mantissa and exponent (for uniforms)
fn hdr_from_parts(head: f32, tail: f32, exp: i32) -> HDRFloat {
    return HDRFloat(head, tail, exp);
}
```

**Step 2: Verify WGSL syntax by including in a test pipeline**

```bash
cargo check -p fractalwonder-gpu
```

Expected: PASS (shader not used yet, but syntax checked when eventually included)

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/hdrfloat.wgsl
git commit -m "feat(gpu): add WGSL HDRFloat shader with double-single arithmetic"
```

---

## Task 10: Update GPU Buffer Types for HDRFloat

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add HDR uniform structs**

Add after existing imports:

```rust
/// Uniform data for direct HDRFloat compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DirectHDRUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,

    // c_origin as HDRFloat: (head, tail, exp) for re and im
    pub c_origin_re_head: f32,
    pub c_origin_re_tail: f32,
    pub c_origin_re_exp: i32,
    pub _pad1: u32,
    pub c_origin_im_head: f32,
    pub c_origin_im_tail: f32,
    pub c_origin_im_exp: i32,
    pub _pad2: u32,

    // c_step as HDRFloat
    pub c_step_re_head: f32,
    pub c_step_re_tail: f32,
    pub c_step_re_exp: i32,
    pub _pad3: u32,
    pub c_step_im_head: f32,
    pub c_step_im_tail: f32,
    pub c_step_im_exp: i32,

    pub adam7_step: u32,
}

/// Uniform data for perturbation HDRFloat compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PerturbationHDRUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,
    pub _pad0: u32,

    // dc_origin as HDRFloat
    pub dc_origin_re_head: f32,
    pub dc_origin_re_tail: f32,
    pub dc_origin_re_exp: i32,
    pub _pad1: u32,
    pub dc_origin_im_head: f32,
    pub dc_origin_im_tail: f32,
    pub dc_origin_im_exp: i32,
    pub _pad2: u32,

    // dc_step as HDRFloat
    pub dc_step_re_head: f32,
    pub dc_step_re_tail: f32,
    pub dc_step_re_exp: i32,
    pub _pad3: u32,
    pub dc_step_im_head: f32,
    pub dc_step_im_tail: f32,
    pub dc_step_im_exp: i32,

    pub adam7_step: u32,
    pub reference_escaped: u32,
    pub orbit_len: u32,
    pub _pad4: u32,
}
```

**Step 2: Add impl blocks for new uniforms**

```rust
impl DirectHDRUniforms {
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        c_origin: ((f32, f32, i32), (f32, f32, i32)), // ((re_head, re_tail, re_exp), (im_head, im_tail, im_exp))
        c_step: ((f32, f32, i32), (f32, f32, i32)),
        adam7_step: u32,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0,
            c_origin_re_head: c_origin.0.0,
            c_origin_re_tail: c_origin.0.1,
            c_origin_re_exp: c_origin.0.2,
            _pad1: 0,
            c_origin_im_head: c_origin.1.0,
            c_origin_im_tail: c_origin.1.1,
            c_origin_im_exp: c_origin.1.2,
            _pad2: 0,
            c_step_re_head: c_step.0.0,
            c_step_re_tail: c_step.0.1,
            c_step_re_exp: c_step.0.2,
            _pad3: 0,
            c_step_im_head: c_step.1.0,
            c_step_im_tail: c_step.1.1,
            c_step_im_exp: c_step.1.2,
            adam7_step,
        }
    }
}

impl PerturbationHDRUniforms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        adam7_step: u32,
        reference_escaped: bool,
        orbit_len: u32,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0,
            tau_sq,
            _pad0: 0,
            dc_origin_re_head: dc_origin.0.0,
            dc_origin_re_tail: dc_origin.0.1,
            dc_origin_re_exp: dc_origin.0.2,
            _pad1: 0,
            dc_origin_im_head: dc_origin.1.0,
            dc_origin_im_tail: dc_origin.1.1,
            dc_origin_im_exp: dc_origin.1.2,
            _pad2: 0,
            dc_step_re_head: dc_step.0.0,
            dc_step_re_tail: dc_step.0.1,
            dc_step_re_exp: dc_step.0.2,
            _pad3: 0,
            dc_step_im_head: dc_step.1.0,
            dc_step_im_tail: dc_step.1.1,
            dc_step_im_exp: dc_step.1.2,
            adam7_step,
            reference_escaped: if reference_escaped { 1 } else { 0 },
            orbit_len,
            _pad4: 0,
        }
    }
}
```

**Step 3: Run cargo check**

```bash
cargo check -p fractalwonder-gpu
```

Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add DirectHDRUniforms and PerturbationHDRUniforms"
```

---

## Task 11: Create Delta Iteration HDR Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl`

**Step 1: Create the shader**

```wgsl
// Delta iteration using HDRFloat for perturbation rendering.
// Computes δz' = 2·Z_m·δz + δz² + δc for each pixel.

// Include HDRFloat library (will be concatenated at build time)
// --- BEGIN hdrfloat.wgsl content ---
// (Include all content from hdrfloat.wgsl here)
// --- END hdrfloat.wgsl content ---

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    _pad0: u32,

    dc_origin_re_head: f32,
    dc_origin_re_tail: f32,
    dc_origin_re_exp: i32,
    _pad1: u32,
    dc_origin_im_head: f32,
    dc_origin_im_tail: f32,
    dc_origin_im_exp: i32,
    _pad2: u32,

    dc_step_re_head: f32,
    dc_step_re_tail: f32,
    dc_step_re_exp: i32,
    _pad3: u32,
    dc_step_im_head: f32,
    dc_step_im_tail: f32,
    dc_step_im_exp: i32,

    adam7_step: u32,
    reference_escaped: u32,
    orbit_len: u32,
    _pad4: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

// Adam7 interlacing pattern
fn adam7_coords(pass: u32) -> vec2<u32> {
    switch pass {
        case 1u: { return vec2<u32>(0u, 0u); }
        case 2u: { return vec2<u32>(4u, 0u); }
        case 3u: { return vec2<u32>(0u, 4u); }
        case 4u: { return vec2<u32>(2u, 0u); }
        case 5u: { return vec2<u32>(0u, 2u); }
        case 6u: { return vec2<u32>(1u, 0u); }
        case 7u: { return vec2<u32>(0u, 1u); }
        default: { return vec2<u32>(0u, 0u); }
    }
}

fn adam7_step(pass: u32) -> vec2<u32> {
    switch pass {
        case 1u: { return vec2<u32>(8u, 8u); }
        case 2u: { return vec2<u32>(8u, 8u); }
        case 3u: { return vec2<u32>(4u, 8u); }
        case 4u: { return vec2<u32>(4u, 4u); }
        case 5u: { return vec2<u32>(2u, 4u); }
        case 6u: { return vec2<u32>(2u, 2u); }
        case 7u: { return vec2<u32>(1u, 2u); }
        default: { return vec2<u32>(1u, 1u); }
    }
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if x >= uniforms.width || y >= uniforms.height {
        return;
    }

    let pixel_idx = y * uniforms.width + x;

    // Adam7 filtering
    if uniforms.adam7_step > 0u {
        let offset = adam7_coords(uniforms.adam7_step);
        let step = adam7_step(uniforms.adam7_step);
        if (x % step.x) != offset.x || (y % step.y) != offset.y {
            results[pixel_idx] = SENTINEL_NOT_COMPUTED;
            glitch_flags[pixel_idx] = 0u;
            z_norm_sq[pixel_idx] = 0.0;
            return;
        }
    }

    // Construct δc for this pixel
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    // δc = dc_origin + pixel_pos * dc_step
    let x_hdr = HDRFloat(f32(x), 0.0, 0);
    let y_hdr = HDRFloat(f32(y), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    var dc = HDRComplex(dc_re, dc_im);

    // δz starts at origin
    var dz = HDR_COMPLEX_ZERO;
    var m: u32 = 0u;
    var glitched: bool = false;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n = n + 1u) {
        // Reference exhaustion detection
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        // Get Z_m from reference orbit
        let z_m = reference_orbit[m % orbit_len];
        let z_m_re = z_m.x;
        let z_m_im = z_m.y;

        // Full z = Z_m + δz (convert Z_m to HDRFloat for addition)
        let z_m_hdr_re = HDRFloat(z_m_re, 0.0, 0);
        let z_m_hdr_im = HDRFloat(z_m_im, 0.0, 0);
        let z_re = hdr_add(z_m_hdr_re, dz.re);
        let z_im = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re, z_im);

        // Magnitudes
        let z_mag_sq = hdr_complex_norm_sq(z);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = hdr_complex_norm_sq(dz);

        // 1. Escape check
        if z_mag_sq > uniforms.escape_radius_sq {
            results[pixel_idx] = n;
            glitch_flags[pixel_idx] = select(0u, 1u, glitched);
            z_norm_sq[pixel_idx] = z_mag_sq;
            return;
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // 2·Z_m·δz
        let two_z_dz_re = hdr_mul_f32(hdr_sub(hdr_mul_f32(dz.re, z_m_re), hdr_mul_f32(dz.im, z_m_im)), 2.0);
        let two_z_dz_im = hdr_mul_f32(hdr_add(hdr_mul_f32(dz.re, z_m_im), hdr_mul_f32(dz.im, z_m_re)), 2.0);

        // δz²
        let dz_sq = hdr_complex_square(dz);

        // δz' = 2·Z·δz + δz² + δc
        dz = HDRComplex(
            hdr_add(hdr_add(two_z_dz_re, dz_sq.re), dc.re),
            hdr_add(hdr_add(two_z_dz_im, dz_sq.im), dc.im)
        );

        m = m + 1u;
    }

    // Reached max iterations
    results[pixel_idx] = uniforms.max_iterations;
    glitch_flags[pixel_idx] = select(0u, 1u, glitched);
    z_norm_sq[pixel_idx] = 0.0;
}
```

**Note:** The actual file needs the hdrfloat.wgsl content included at the top. You can either:
1. Concatenate at build time
2. Copy-paste the content inline

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl
git commit -m "feat(gpu): add delta_iteration_hdr.wgsl shader"
```

---

## Task 12: Delete FloatExp Files

**Files:**
- Delete: `fractalwonder-core/src/floatexp.rs`
- Delete: `fractalwonder-gpu/src/shaders/floatexp.wgsl`
- Delete: `fractalwonder-gpu/src/shaders/direct_floatexp.wgsl`
- Delete: `fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl`
- Modify: `fractalwonder-core/src/lib.rs` - remove FloatExp export
- Modify: `fractalwonder-gpu/src/lib.rs` - update exports

**Step 1: Remove FloatExp from core lib.rs**

In `fractalwonder-core/src/lib.rs`, remove:
```rust
pub mod floatexp;
pub use floatexp::FloatExp;
```

**Step 2: Delete floatexp.rs**

```bash
rm fractalwonder-core/src/floatexp.rs
```

**Step 3: Delete FloatExp shaders**

```bash
rm fractalwonder-gpu/src/shaders/floatexp.wgsl
rm fractalwonder-gpu/src/shaders/direct_floatexp.wgsl
rm fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl
```

**Step 4: Run cargo check (will fail - that's expected)**

```bash
cargo check --workspace
```

Expected: FAIL with errors about missing FloatExp

**Step 5: Commit the deletion**

```bash
git add -u
git commit -m "refactor: remove FloatExp in favor of HDRFloat"
```

---

## Task 13: Update perturbation.rs to Use HDRFloat

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Replace FloatExp imports with HDRFloat**

Change:
```rust
use fractalwonder_core::{BigFloat, FloatExp, MandelbrotData};
```
To:
```rust
use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat, MandelbrotData};
```

**Step 2: Replace compute_pixel_perturbation_floatexp**

Rename function and update signature:
```rust
/// Compute pixel using perturbation with HDRFloat deltas.
/// ~48-bit precision for deep zoom.
pub fn compute_pixel_perturbation_hdr(
    orbit: &ReferenceOrbit,
    delta_c: HDRComplex,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let mut dz = HDRComplex::ZERO;
    let mut m: usize = 0;
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
            final_z_norm_sq: 0.0,
        };
    }

    let reference_escaped = orbit.escaped_at.is_some();

    for n in 0..max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];

        // z = Z_m + δz
        let z_re = HDRFloat::from_f64(z_m_re).add(&dz.re);
        let z_im = HDRFloat::from_f64(z_m_im).add(&dz.im);

        // Magnitudes
        let z_mag_sq = z_re.square().add(&z_im.square()).to_f64();
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = dz.norm_sq();

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
                final_z_norm_sq: z_mag_sq as f32,
            };
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = HDRComplex { re: z_re, im: z_im };
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z·δz + δz² + δc
        let two_z_dz_re = dz.re.mul_f64(z_m_re).sub(&dz.im.mul_f64(z_m_im)).mul_f64(2.0);
        let two_z_dz_im = dz.re.mul_f64(z_m_im).add(&dz.im.mul_f64(z_m_re)).mul_f64(2.0);

        let dz_sq = dz.square();

        dz = HDRComplex {
            re: two_z_dz_re.add(&dz_sq.re).add(&delta_c.re),
            im: two_z_dz_im.add(&dz_sq.im).add(&delta_c.im),
        };

        m += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
        final_z_norm_sq: 0.0,
    }
}
```

**Step 3: Update tests to use HDRFloat**

Update the test that uses FloatExp:
```rust
#[test]
fn hdr_matches_f64_at_shallow_zoom() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 500);

    let test_deltas = [(0.01, 0.01), (-0.005, 0.002), (0.1, -0.05)];

    for (dx, dy) in test_deltas {
        let f64_result = compute_pixel_perturbation(&orbit, (dx, dy), 500, TEST_TAU_SQ);

        let delta_c = HDRComplex {
            re: HDRFloat::from_f64(dx),
            im: HDRFloat::from_f64(dy),
        };
        let hdr_result = compute_pixel_perturbation_hdr(&orbit, delta_c, 500, TEST_TAU_SQ);

        assert_eq!(f64_result.escaped, hdr_result.escaped);
        assert_eq!(f64_result.iterations, hdr_result.iterations);
    }
}
```

**Step 4: Run cargo check**

```bash
cargo check -p fractalwonder-compute
```

Expected: PASS (once all FloatExp references are replaced)

**Step 5: Run tests**

```bash
cargo test -p fractalwonder-compute -- --nocapture
```

Expected: PASS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "refactor(compute): replace FloatExp with HDRFloat in perturbation"
```

---

## Task 14: Update GPU Pipeline and Renderer

**Files:**
- Rename: `perturbation_floatexp_pipeline.rs` → `perturbation_hdr_pipeline.rs`
- Rename: `perturbation_floatexp_renderer.rs` → `perturbation_hdr_renderer.rs`
- Rename: `direct_pipeline.rs` → `direct_hdr_pipeline.rs`
- Rename: `direct_renderer.rs` → `direct_hdr_renderer.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Rename files**

```bash
cd fractalwonder-gpu/src
mv perturbation_floatexp_pipeline.rs perturbation_hdr_pipeline.rs
mv perturbation_floatexp_renderer.rs perturbation_hdr_renderer.rs
mv direct_pipeline.rs direct_hdr_pipeline.rs
mv direct_renderer.rs direct_hdr_renderer.rs
```

**Step 2: Update perturbation_hdr_pipeline.rs**

Replace `delta_iteration_floatexp.wgsl` with `delta_iteration_hdr.wgsl`:
```rust
source: wgpu::ShaderSource::Wgsl(
    include_str!("shaders/delta_iteration_hdr.wgsl").into(),
),
```

Rename struct and labels:
- `PerturbationFloatExpPipeline` → `PerturbationHDRPipeline`

**Step 3: Update perturbation_hdr_renderer.rs**

Update imports:
```rust
use crate::buffers::{PerturbationHDRBuffers, PerturbationHDRUniforms};
use crate::perturbation_hdr_pipeline::PerturbationHDRPipeline;
```

Rename:
- `GpuPerturbationFloatExpRenderer` → `GpuPerturbationHDRRenderer`
- `GpuPerturbationFloatExpResult` → `GpuPerturbationHDRResult`

Update render signature to accept HDRFloat tuples:
```rust
pub async fn render(
    &mut self,
    orbit: &[(f64, f64)],
    orbit_id: u32,
    dc_origin: ((f32, f32, i32), (f32, f32, i32)), // HDRFloat format
    dc_step: ((f32, f32, i32), (f32, f32, i32)),
    // ... rest unchanged
)
```

**Step 4: Update lib.rs exports**

```rust
mod perturbation_hdr_pipeline;
mod perturbation_hdr_renderer;
mod direct_hdr_pipeline;
mod direct_hdr_renderer;

pub use perturbation_hdr_pipeline::PerturbationHDRPipeline;
pub use perturbation_hdr_renderer::{GpuPerturbationHDRRenderer, GpuPerturbationHDRResult};
pub use direct_hdr_pipeline::DirectHDRPipeline;
pub use direct_hdr_renderer::{DirectHDRResult, GpuDirectHDRRenderer};
```

**Step 5: Run cargo check**

```bash
cargo check -p fractalwonder-gpu
```

**Step 6: Commit**

```bash
git add -A fractalwonder-gpu/
git commit -m "refactor(gpu): rename FloatExp pipeline/renderer to HDR"
```

---

## Task 15: Update UI Integration

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`
- Other files that import FloatExp types

**Step 1: Search for remaining FloatExp references**

```bash
grep -r "FloatExp" fractalwonder-ui/
grep -r "FloatExp" fractalwonder-gpu/
grep -r "floatexp" --include="*.rs" .
```

**Step 2: Update each file found**

Replace imports and type names:
- `FloatExp` → `HDRFloat`
- `PerturbationFloatExpUniforms` → `PerturbationHDRUniforms`
- `GpuPerturbationFloatExpRenderer` → `GpuPerturbationHDRRenderer`
- etc.

**Step 3: Run full workspace check**

```bash
cargo check --workspace
```

**Step 4: Run all tests**

```bash
cargo test --workspace -- --nocapture
```

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: complete HDRFloat migration across workspace"
```

---

## Task 16: Final Verification

**Step 1: Run full test suite**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

**Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Step 3: Run cargo fmt**

```bash
cargo fmt --all
```

**Step 4: Build release**

```bash
trunk build --release
```

**Step 5: Manual testing**

Open the application and verify:
- Shallow zoom renders correctly
- Deep zoom (10^50+) renders without artifacts
- No performance regression

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: HDRFloat migration complete - all tests passing"
```

---

## Summary

**Total Tasks:** 16
**Estimated Commits:** ~16

**Key Changes:**
1. Created `HDRFloat` and `HDRComplex` in fractalwonder-core with ~48-bit precision
2. Created WGSL `hdrfloat.wgsl` shader library
3. Created `delta_iteration_hdr.wgsl` for GPU perturbation
4. Deleted all FloatExp code
5. Updated GPU buffers, pipelines, and renderers
6. Updated perturbation.rs to use HDRFloat
7. Updated all imports across workspace
