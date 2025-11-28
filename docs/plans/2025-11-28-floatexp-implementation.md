# FloatExp Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace BigFloat delta arithmetic with FloatExp for 10-20x speedup in perturbation rendering.

**Architecture:** FloatExp = f64 mantissa (normalized [0.5, 1.0)) + i64 exponent. Provides unlimited range with 53-bit precision using fast hardware operations. Lives in `fractalwonder-core`, used by perturbation in `fractalwonder-compute`.

**Tech Stack:** Rust, libm (frexp/ldexp), serde

---

## Task 1: Create FloatExp module with zero and from_f64

**Files:**
- Create: `fractalwonder-core/src/floatexp.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Create `fractalwonder-core/src/floatexp.rs`:

```rust
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
        let values = [1.0, -1.0, 0.5, 2.0, 1e10, 1e-10, -3.14159];
        for v in values {
            let fe = FloatExp::from_f64(v);
            let back = fe.to_f64();
            assert!(
                (back - v).abs() < 1e-14 * v.abs().max(1.0),
                "from_f64({}) -> to_f64() = {}, expected {}",
                v, back, v
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
                fe.mantissa, v
            );
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: FAIL - `FloatExp::zero()`, `from_f64()`, etc. not implemented

**Step 3: Implement the constructors**

Add to `fractalwonder-core/src/floatexp.rs` before the tests module:

```rust
impl FloatExp {
    /// Zero value.
    pub fn zero() -> Self {
        Self { mantissa: 0.0, exp: 0 }
    }

    /// Create from f64 (normalizes automatically).
    pub fn from_f64(val: f64) -> Self {
        if val == 0.0 {
            return Self::zero();
        }
        // frexp returns (mantissa, exponent) where mantissa is in [0.5, 1.0)
        let (mantissa, exp) = libm::frexp(val);
        Self { mantissa, exp: exp as i64 }
    }

    /// Convert to f64 (may overflow/underflow for extreme exponents).
    pub fn to_f64(&self) -> f64 {
        if self.mantissa == 0.0 {
            return 0.0;
        }
        // Handle extreme exponents
        if self.exp > 1023 {
            return if self.mantissa > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
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
```

**Step 4: Add libm dependency**

Run: `cargo add libm -p fractalwonder-core`

**Step 5: Run test to verify it passes**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: PASS

**Step 6: Export from lib.rs**

Add to `fractalwonder-core/src/lib.rs`:

```rust
pub mod floatexp;
pub use floatexp::FloatExp;
```

**Step 7: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs fractalwonder-core/src/lib.rs fractalwonder-core/Cargo.toml
git commit -m "feat: add FloatExp type with zero and from_f64"
```

---

## Task 2: Add FloatExp multiplication

**Files:**
- Modify: `fractalwonder-core/src/floatexp.rs`

**Step 1: Write the failing test**

Add to the tests module in `floatexp.rs`:

```rust
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core mul_basic`

Expected: FAIL - `mul` not implemented

**Step 3: Implement multiplication**

Add to the `impl FloatExp` block:

```rust
    /// Multiply two FloatExp values.
    pub fn mul(&self, other: &Self) -> Self {
        if self.mantissa == 0.0 || other.mantissa == 0.0 {
            return Self::zero();
        }
        Self {
            mantissa: self.mantissa * other.mantissa,
            exp: self.exp + other.exp,
        }.normalize()
    }

    /// Multiply by f64 scalar (for 2·Z·δz where Z is f64).
    pub fn mul_f64(&self, scalar: f64) -> Self {
        if self.mantissa == 0.0 || scalar == 0.0 {
            return Self::zero();
        }
        Self {
            mantissa: self.mantissa * scalar,
            exp: self.exp,
        }.normalize()
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs
git commit -m "feat: add FloatExp multiplication"
```

---

## Task 3: Add FloatExp addition and subtraction

**Files:**
- Modify: `fractalwonder-core/src/floatexp.rs`

**Step 1: Write the failing tests**

Add to tests module:

```rust
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
        let tiny = FloatExp { mantissa: 0.5, exp: -100 }; // 2^-101
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core add_basic`

Expected: FAIL - `add` not implemented

**Step 3: Implement addition, subtraction, negation**

Add to `impl FloatExp`:

```rust
    /// Add two FloatExp values.
    pub fn add(&self, other: &Self) -> Self {
        if self.mantissa == 0.0 { return *other; }
        if other.mantissa == 0.0 { return *self; }

        let exp_diff = self.exp - other.exp;

        // If difference > 53 bits, smaller value is negligible
        if exp_diff > 53 { return *self; }
        if exp_diff < -53 { return *other; }

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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs
git commit -m "feat: add FloatExp add, sub, neg operations"
```

---

## Task 4: Add FloatExp::norm_sq for escape/rebase checks

**Files:**
- Modify: `fractalwonder-core/src/floatexp.rs`

**Step 1: Write the failing test**

Add to tests module:

```rust
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core norm_sq_basic`

Expected: FAIL - `norm_sq` not implemented

**Step 3: Implement norm_sq**

Add to `impl FloatExp`:

```rust
    /// Squared magnitude of complex number (re, im).
    /// Returns f64 since result is bounded for escape testing (|z|² compared to 4).
    pub fn norm_sq(re: &FloatExp, im: &FloatExp) -> f64 {
        let re_sq = re.mul(re);
        let im_sq = im.mul(im);
        re_sq.add(&im_sq).to_f64()
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs
git commit -m "feat: add FloatExp::norm_sq for escape/rebase checks"
```

---

## Task 5: Add FloatExp::from_bigfloat conversion

**Files:**
- Modify: `fractalwonder-core/src/floatexp.rs`

**Step 1: Write the failing test**

Add to tests module:

```rust
    #[test]
    fn from_bigfloat_f64_range() {
        use crate::BigFloat;
        let bf = BigFloat::with_precision(3.14159, 128);
        let fe = FloatExp::from_bigfloat(&bf);
        assert!((fe.to_f64() - 3.14159).abs() < 1e-10);
    }

    #[test]
    fn from_bigfloat_extreme_small() {
        use crate::BigFloat;
        // 10^-500 is far beyond f64 range
        let bf = BigFloat::from_string("1e-500", 2048).unwrap();
        let fe = FloatExp::from_bigfloat(&bf);

        // Value should not be zero (f64 underflow)
        assert!(!fe.is_zero(), "Should not underflow to zero");

        // Exponent should be approximately -500 * log2(10) ≈ -1661
        assert!(fe.exp < -1600, "Exponent {} should be < -1600", fe.exp);
        assert!(fe.exp > -1700, "Exponent {} should be > -1700", fe.exp);
    }

    #[test]
    fn from_bigfloat_zero() {
        use crate::BigFloat;
        let bf = BigFloat::zero(128);
        let fe = FloatExp::from_bigfloat(&bf);
        assert!(fe.is_zero());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core from_bigfloat`

Expected: FAIL - `from_bigfloat` not implemented

**Step 3: Implement from_bigfloat**

Add import at top of `floatexp.rs`:

```rust
use crate::BigFloat;
```

Add to `impl FloatExp`:

```rust
    /// Convert from BigFloat, extracting mantissa and exponent.
    ///
    /// For values within f64 range, uses direct conversion.
    /// For extreme values (|log2| > 1000), extracts exponent from BigFloat's
    /// internal representation to avoid f64 underflow/overflow.
    pub fn from_bigfloat(bf: &BigFloat) -> Self {
        // Try direct f64 conversion first (fast path)
        let f64_val = bf.to_f64();
        if f64_val != 0.0 && f64_val.is_finite() {
            return Self::from_f64(f64_val);
        }

        // Value is zero, infinity, or underflowed - check log2
        let log2 = bf.log2_approx();
        if log2 == f64::NEG_INFINITY {
            return Self::zero();
        }

        // Extreme value: reconstruct from log2 approximation
        // log2(mantissa × 2^exp) = log2(mantissa) + exp
        // With mantissa in [0.5, 1.0), log2(mantissa) is in [-1, 0)
        // So exp ≈ log2 rounded
        let exp = log2.round() as i64;

        // Mantissa approximation: we know the value is positive (from log2)
        // and the magnitude. We can estimate mantissa as 2^(log2 - exp)
        let mantissa_log2 = log2 - exp as f64;
        let mantissa = libm::exp2(mantissa_log2);

        Self { mantissa, exp }.normalize()
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core floatexp`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs
git commit -m "feat: add FloatExp::from_bigfloat for deep zoom conversion"
```

---

## Task 6: Add compute_pixel_perturbation_floatexp

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Write the failing test**

Add to tests module in `perturbation.rs`:

```rust
    #[test]
    fn floatexp_matches_f64_at_shallow_zoom() {
        use fractalwonder_core::FloatExp;

        // Reference in set
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // Test multiple delta values within f64 range
        let test_deltas = [(0.01, 0.01), (-0.005, 0.002), (0.1, -0.05)];

        for (dx, dy) in test_deltas {
            // f64 version
            let f64_result = compute_pixel_perturbation(&orbit, (dx, dy), 500, TEST_TAU_SQ);

            // FloatExp version
            let delta_c = (FloatExp::from_f64(dx), FloatExp::from_f64(dy));
            let floatexp_result = compute_pixel_perturbation_floatexp(&orbit, delta_c, 500, TEST_TAU_SQ);

            assert_eq!(
                f64_result.escaped, floatexp_result.escaped,
                "Escape mismatch for delta ({}, {})", dx, dy
            );
            assert_eq!(
                f64_result.iterations, floatexp_result.iterations,
                "Iteration mismatch for delta ({}, {})", dx, dy
            );
        }
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute floatexp_matches_f64`

Expected: FAIL - `compute_pixel_perturbation_floatexp` not implemented

**Step 3: Implement the function**

Add to `perturbation.rs` after the imports:

```rust
use fractalwonder_core::FloatExp;
```

Add the function:

```rust
/// Compute pixel using perturbation with FloatExp deltas.
/// 10-20x faster than BigFloat, same correctness for deep zoom.
pub fn compute_pixel_perturbation_floatexp(
    orbit: &ReferenceOrbit,
    delta_c: (FloatExp, FloatExp),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let (dc_re, dc_im) = delta_c;
    let mut dz_re = FloatExp::zero();
    let mut dz_im = FloatExp::zero();
    let mut m: usize = 0;
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
        };
    }

    for n in 0..max_iterations {
        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];

        // z = Z_m + δz
        let z_re = FloatExp::from_f64(z_m_re).add(&dz_re);
        let z_im = FloatExp::from_f64(z_m_im).add(&dz_im);

        // Magnitudes (f64 - bounded values)
        let z_mag_sq = FloatExp::norm_sq(&z_re, &z_im);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = FloatExp::norm_sq(&dz_re, &dz_im);

        // 1. Escape check
        if z_mag_sq > 4.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
            };
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz_re = z_re;
            dz_im = z_im;
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z·δz + δz² + δc
        // 2·Z·δz = 2·(Z_re·δz_re - Z_im·δz_im, Z_re·δz_im + Z_im·δz_re)
        let two_z_dz_re = dz_re.mul_f64(z_m_re).sub(&dz_im.mul_f64(z_m_im)).mul_f64(2.0);
        let two_z_dz_im = dz_re.mul_f64(z_m_im).add(&dz_im.mul_f64(z_m_re)).mul_f64(2.0);

        // δz² = (δz_re² - δz_im², 2·δz_re·δz_im)
        let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
        let dz_sq_im = dz_re.mul(&dz_im).mul_f64(2.0);

        // δz' = 2·Z·δz + δz² + δc
        dz_re = two_z_dz_re.add(&dz_sq_re).add(&dc_re);
        dz_im = two_z_dz_im.add(&dz_sq_im).add(&dc_im);

        m += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}
```

**Step 4: Export from lib.rs**

Modify `fractalwonder-compute/src/lib.rs` to add export:

```rust
pub use perturbation::{
    compute_pixel_perturbation, compute_pixel_perturbation_bigfloat,
    compute_pixel_perturbation_floatexp, ReferenceOrbit,
};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute floatexp`

Expected: PASS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs fractalwonder-compute/src/lib.rs
git commit -m "feat: add compute_pixel_perturbation_floatexp"
```

---

## Task 7: Add FloatExp cross-validation test with BigFloat

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Write the cross-validation test**

Add to tests module:

```rust
    #[test]
    fn floatexp_matches_bigfloat_at_deep_zoom() {
        use fractalwonder_core::FloatExp;

        let precision = 2048;

        // Reference at origin
        let c_ref = (BigFloat::zero(precision), BigFloat::zero(precision));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // Delta at 10^-500 scale
        let delta_bf = (
            BigFloat::from_string("1e-500", precision).unwrap(),
            BigFloat::from_string("2e-500", precision).unwrap(),
        );

        // Convert to FloatExp
        let delta_fe = (
            FloatExp::from_bigfloat(&delta_bf.0),
            FloatExp::from_bigfloat(&delta_bf.1),
        );

        // BigFloat version (reference implementation)
        let bf_result = compute_pixel_perturbation_bigfloat(
            &orbit, &delta_bf.0, &delta_bf.1, 500, TEST_TAU_SQ
        );

        // FloatExp version (optimized)
        let fe_result = compute_pixel_perturbation_floatexp(&orbit, delta_fe, 500, TEST_TAU_SQ);

        assert_eq!(
            bf_result.escaped, fe_result.escaped,
            "Escape status should match at deep zoom"
        );
        assert_eq!(
            bf_result.iterations, fe_result.iterations,
            "Iteration count should match at deep zoom"
        );
    }
```

**Step 2: Run test**

Run: `cargo test -p fractalwonder-compute floatexp_matches_bigfloat`

Expected: PASS (if Task 6 implementation is correct)

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test: cross-validate FloatExp against BigFloat at deep zoom"
```

---

## Task 8: Update worker to use FloatExp

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Update imports**

Change the import line to include FloatExp:

```rust
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_bigfloat,
    compute_pixel_perturbation_floatexp, MandelbrotRenderer, ReferenceOrbit,
    Renderer, TestImageRenderer,
};
use fractalwonder_core::{BigFloat, ComputeData, FloatExp, MainToWorker, Viewport, WorkerToMain};
```

**Step 2: Update RenderTilePerturbation handler**

Replace the existing handler (lines ~272-379) with three-tier dispatch:

```rust
        MainToWorker::RenderTilePerturbation {
            render_id,
            tile,
            orbit_id,
            delta_c_origin_json,
            delta_c_step_json,
            max_iterations,
            tau_sq,
            bigfloat_threshold_bits,
        } => {
            // Parse BigFloat deltas from JSON
            let delta_c_origin: (BigFloat, BigFloat) =
                match serde_json::from_str(&delta_c_origin_json) {
                    Ok(d) => d,
                    Err(e) => {
                        post_message(&WorkerToMain::Error {
                            message: format!("Failed to parse delta_c_origin: {}", e),
                        });
                        return;
                    }
                };

            let delta_c_step: (BigFloat, BigFloat) = match serde_json::from_str(&delta_c_step_json)
            {
                Ok(d) => d,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse delta_c_step: {}", e),
                    });
                    return;
                }
            };

            // Get cached orbit
            let cached = match state.orbit_cache.get(&orbit_id) {
                Some(c) => c,
                None => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Orbit {} not found in cache", orbit_id),
                    });
                    return;
                }
            };

            let orbit = cached.to_reference_orbit();
            let start_time = Date::now();
            let precision = delta_c_origin.0.precision_bits();

            let mut data = Vec::with_capacity((tile.width * tile.height) as usize);

            // Three-tier dispatch based on precision:
            // 1. precision <= 64: Use fast f64 path
            // 2. 64 < precision <= bigfloat_threshold_bits: Use FloatExp (10-20x faster than BigFloat)
            // 3. precision > bigfloat_threshold_bits: Use BigFloat (highest precision)

            if precision <= 64 {
                // Fast path: f64 arithmetic
                let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
                let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());

                let mut delta_c_row = delta_origin;

                for _py in 0..tile.height {
                    let mut delta_c = delta_c_row;

                    for _px in 0..tile.width {
                        let result =
                            compute_pixel_perturbation(&orbit, delta_c, max_iterations, tau_sq);
                        data.push(ComputeData::Mandelbrot(result));

                        delta_c.0 += delta_step.0;
                    }

                    delta_c_row.1 += delta_step.1;
                }
            } else if precision <= bigfloat_threshold_bits {
                // Medium path: FloatExp arithmetic (extended range, f64 precision)
                let delta_origin = (
                    FloatExp::from_bigfloat(&delta_c_origin.0),
                    FloatExp::from_bigfloat(&delta_c_origin.1),
                );
                let delta_step = (
                    FloatExp::from_bigfloat(&delta_c_step.0),
                    FloatExp::from_bigfloat(&delta_c_step.1),
                );

                let mut delta_c_row = delta_origin;

                for _py in 0..tile.height {
                    let mut delta_c = delta_c_row;

                    for _px in 0..tile.width {
                        let result = compute_pixel_perturbation_floatexp(
                            &orbit, delta_c, max_iterations, tau_sq
                        );
                        data.push(ComputeData::Mandelbrot(result));

                        delta_c.0 = delta_c.0.add(&delta_step.0);
                        delta_c.1 = delta_c.1.add(&delta_step.1);
                    }

                    delta_c_row.1 = delta_c_row.1.add(&delta_step.1);
                }
            } else {
                // Deep zoom path: BigFloat arithmetic (full precision)
                let delta_c_row_re = delta_c_origin.0.clone();
                let mut delta_c_row_im = delta_c_origin.1.clone();

                for _py in 0..tile.height {
                    let mut delta_c_re = delta_c_row_re.clone();

                    for _px in 0..tile.width {
                        let result = compute_pixel_perturbation_bigfloat(
                            &orbit,
                            &delta_c_re,
                            &delta_c_row_im,
                            max_iterations,
                            tau_sq,
                        );
                        data.push(ComputeData::Mandelbrot(result));

                        delta_c_re = delta_c_re.add(&delta_c_step.0);
                    }

                    delta_c_row_im = delta_c_row_im.add(&delta_c_step.1);
                }
            }

            let compute_time_ms = Date::now() - start_time;

            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            });

            post_message(&WorkerToMain::RequestWork {
                render_id: Some(render_id),
            });
        }
```

**Step 3: Build check**

Run: `cargo check -p fractalwonder-compute`

Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat: worker uses FloatExp for medium-precision deep zoom"
```

---

## Task 9: Run quality checks

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings or errors

**Step 3: Full test suite**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 4: Build check**

Run: `cargo check --workspace --all-targets --all-features`

Expected: Success

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: format and lint fixes" --allow-empty
```

---

## Summary

After completing all tasks:

1. **FloatExp type** in `fractalwonder-core/src/floatexp.rs`:
   - f64 mantissa normalized to [0.5, 1.0)
   - i64 exponent for unlimited range
   - Operations: `zero`, `from_f64`, `to_f64`, `mul`, `mul_f64`, `add`, `sub`, `neg`, `norm_sq`, `from_bigfloat`

2. **New perturbation function** `compute_pixel_perturbation_floatexp`:
   - Cross-validated against both f64 and BigFloat versions
   - Same algorithm, different arithmetic type

3. **Three-tier worker dispatch**:
   - precision ≤ 64 bits: f64 (fastest)
   - 64 < precision ≤ threshold: FloatExp (fast extended range)
   - precision > threshold: BigFloat (full precision)

**Expected performance improvement:** 10-20x faster than BigFloat for most deep zoom rendering.

---

Plan complete and saved to `docs/plans/2025-11-28-floatexp-implementation.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
