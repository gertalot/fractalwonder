# Perturbation Trait Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify 4 duplicate perturbation functions into a single generic function using a `ComplexDelta` trait.

**Architecture:** Create a trait in fractalwonder-core that abstracts complex number operations. Implement for F64Complex (new), HDRComplex (existing), and BigFloatComplex (new). Replace 4 functions with one generic function. Delete dead code.

**Tech Stack:** Rust, WASM, no new dependencies

---

## Task 1: Create ComplexDelta Trait and F64Complex

**Files:**
- Create: `fractalwonder-core/src/complex_delta.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-core/src/complex_delta.rs`:

```rust
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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core complex_delta`
Expected: FAIL with "can't find crate for `complex_delta`" or similar

**Step 3: Write trait and F64Complex implementation**

Create `fractalwonder-core/src/complex_delta.rs`:

```rust
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
```

**Step 4: Add module to lib.rs**

Modify `fractalwonder-core/src/lib.rs`, add after other mod declarations:

```rust
mod complex_delta;

pub use complex_delta::{ComplexDelta, F64Complex};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core complex_delta`
Expected: All 7 tests PASS

**Step 6: Commit**

```bash
git add fractalwonder-core/src/complex_delta.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add ComplexDelta trait and F64Complex implementation"
```

---

## Task 2: Implement ComplexDelta for HDRComplex

**Files:**
- Modify: `fractalwonder-core/src/hdrfloat.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Add to end of `fractalwonder-core/src/hdrfloat.rs` (inside existing `mod tests`):

```rust
    #[test]
    fn hdr_complex_delta_zero() {
        use crate::ComplexDelta;
        let c = HDRComplex::from_f64_pair(1.0, 2.0);
        let z = c.zero();
        assert!(z.re.is_zero());
        assert!(z.im.is_zero());
    }

    #[test]
    fn hdr_complex_delta_add() {
        use crate::ComplexDelta;
        let a = HDRComplex::from_f64_pair(1.0, 2.0);
        let b = HDRComplex::from_f64_pair(3.0, 4.0);
        let c = a.add(&b);
        let (re, im) = c.to_f64_pair();
        assert!((re - 4.0).abs() < 1e-10);
        assert!((im - 6.0).abs() < 1e-10);
    }

    #[test]
    fn hdr_complex_delta_mul() {
        use crate::ComplexDelta;
        // (1 + 2i) * (3 + 4i) = -5 + 10i
        let a = HDRComplex::from_f64_pair(1.0, 2.0);
        let b = HDRComplex::from_f64_pair(3.0, 4.0);
        let c = a.mul(&b);
        let (re, im) = c.to_f64_pair();
        assert!((re - (-5.0)).abs() < 1e-10);
        assert!((im - 10.0).abs() < 1e-10);
    }

    #[test]
    fn hdr_complex_delta_norm_sq() {
        use crate::ComplexDelta;
        // |3 + 4i|² = 25
        let a = HDRComplex::from_f64_pair(3.0, 4.0);
        let norm = a.norm_sq();
        assert!((norm - 25.0).abs() < 1e-10);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core hdr_complex_delta`
Expected: FAIL with "method `add` not found" or "trait `ComplexDelta` not implemented"

**Step 3: Implement ComplexDelta for HDRComplex**

Add to `fractalwonder-core/src/hdrfloat.rs`, after the existing `impl HDRComplex` block:

```rust
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core hdr_complex_delta`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/hdrfloat.rs
git commit -m "feat(core): implement ComplexDelta for HDRComplex"
```

---

## Task 3: Implement BigFloatComplex

**Files:**
- Modify: `fractalwonder-core/src/complex_delta.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-core/src/complex_delta.rs` tests module:

```rust
    #[test]
    fn bigfloat_complex_zero_preserves_precision() {
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
        // |3 + 4i|² = 25
        let a = BigFloatComplex::new(
            BigFloat::with_precision(3.0, 128),
            BigFloat::with_precision(4.0, 128),
        );
        let norm = a.norm_sq();
        assert!((norm - 25.0).abs() < 1e-10);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-core bigfloat_complex`
Expected: FAIL with "cannot find struct `BigFloatComplex`"

**Step 3: Implement BigFloatComplex**

Add to `fractalwonder-core/src/complex_delta.rs`, after F64Complex:

```rust
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
            im: self.re.mul(&self.im).mul(&BigFloat::with_precision(2.0, self.re.precision_bits())),
        }
    }

    fn norm_sq(&self) -> f64 {
        self.re.mul(&self.re).add(&self.im.mul(&self.im)).to_f64()
    }
}
```

**Step 4: Update lib.rs exports**

Modify `fractalwonder-core/src/lib.rs`:

```rust
pub use complex_delta::{BigFloatComplex, ComplexDelta, F64Complex};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core bigfloat_complex`
Expected: All 4 tests PASS

**Step 6: Commit**

```bash
git add fractalwonder-core/src/complex_delta.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add BigFloatComplex with ComplexDelta implementation"
```

---

## Task 4: Add Generic Perturbation Function (alongside existing)

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-compute/src/perturbation.rs` tests module:

```rust
    #[test]
    fn generic_f64_matches_original_escaped() {
        use fractalwonder_core::{ComplexDelta, F64Complex};

        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);
        let delta_c = (0.5, 0.0);

        let original = compute_pixel_perturbation(&orbit, delta_c, 1000, TEST_TAU_SQ);
        let generic = compute_pixel_perturbation_generic(
            &orbit,
            F64Complex::from_f64_pair(delta_c.0, delta_c.1),
            1000,
            TEST_TAU_SQ,
        );

        assert_eq!(original.iterations, generic.iterations);
        assert_eq!(original.escaped, generic.escaped);
        assert_eq!(original.glitched, generic.glitched);
    }

    #[test]
    fn generic_f64_matches_original_in_set() {
        use fractalwonder_core::{ComplexDelta, F64Complex};

        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);
        let delta_c = (0.01, 0.01);

        let original = compute_pixel_perturbation(&orbit, delta_c, 500, TEST_TAU_SQ);
        let generic = compute_pixel_perturbation_generic(
            &orbit,
            F64Complex::from_f64_pair(delta_c.0, delta_c.1),
            500,
            TEST_TAU_SQ,
        );

        assert_eq!(original.iterations, generic.iterations);
        assert_eq!(original.escaped, generic.escaped);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute generic_f64_matches`
Expected: FAIL with "cannot find function `compute_pixel_perturbation_generic`"

**Step 3: Implement generic function**

Add to `fractalwonder-compute/src/perturbation.rs`, after the existing functions:

```rust
use fractalwonder_core::ComplexDelta;

/// Generic perturbation iteration for any ComplexDelta type.
///
/// Computes the Mandelbrot iteration using perturbation theory with
/// the provided delta type. The compiler monomorphizes this into
/// type-specific code with zero runtime overhead.
pub fn compute_pixel_perturbation_generic<D: ComplexDelta>(
    orbit: &ReferenceOrbit,
    delta_c: D,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
            final_z_norm_sq: 0.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        };
    }

    // Check if reference escaped at iteration 0
    let reference_escaped = orbit.escaped_at.is_some();
    if let Some(0) = orbit.escaped_at {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 0.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        };
    }

    // Initialize deltas with matching precision
    let mut dz = delta_c.zero();
    let mut drho = delta_c.zero();

    let mut m: usize = 0;
    let mut n: u32 = 0;
    let mut glitched = false;

    while n < max_iterations {
        // Glitch if we've run past a finite orbit
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        // Get Z_m and Der_m with wrap-around
        let z_m = orbit.orbit[m % orbit_len];
        let der_m = orbit.derivative[m % orbit_len];
        let z_m_complex = D::from_f64_pair(z_m.0, z_m.1);
        let der_m_complex = D::from_f64_pair(der_m.0, der_m.1);

        // Full z = Z_m + δz
        let z = z_m_complex.add(&dz);
        let z_norm_sq = z.norm_sq();

        // Full derivative ρ = Der_m + δρ
        let rho = der_m_complex.add(&drho);

        // Escape check
        if z_norm_sq > 65536.0 {
            let (z_re, z_im) = z.to_f64_pair();
            let (rho_re, rho_im) = rho.to_f64_pair();
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
                final_z_norm_sq: z_norm_sq as f32,
                final_z_re: z_re as f32,
                final_z_im: z_im as f32,
                final_derivative_re: rho_re as f32,
                final_derivative_im: rho_im as f32,
            };
        }

        // Pauldelbrot glitch detection
        let z_m_norm_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        if z_m_norm_sq > 1e-20 && z_norm_sq < tau_sq * z_m_norm_sq {
            glitched = true;
        }

        // Rebase check
        let dz_norm_sq = dz.norm_sq();
        if z_norm_sq < dz_norm_sq {
            dz = z;
            drho = rho;
            m = 0;
            continue;
        }

        // Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        let old_dz = dz.clone();
        let two_z_dz = z_m_complex.mul(&dz).scale(2.0);
        let dz_sq = dz.square();
        dz = two_z_dz.add(&dz_sq).add(&delta_c);

        // Derivative iteration: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
        let term1 = z_m_complex.mul(&drho).scale(2.0);
        let term2 = old_dz.mul(&der_m_complex).scale(2.0);
        let term3 = old_dz.mul(&drho).scale(2.0);
        drho = term1.add(&term2).add(&term3);

        m += 1;
        n += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
        final_z_norm_sq: 0.0,
        final_z_re: 0.0,
        final_z_im: 0.0,
        final_derivative_re: 0.0,
        final_derivative_im: 0.0,
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute generic_f64_matches`
Expected: All 2 tests PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): add generic compute_pixel_perturbation_generic function"
```

---

## Task 5: Add Equivalence Tests for HDR and BigFloat

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Write HDR equivalence test**

Add to tests module:

```rust
    #[test]
    fn generic_hdr_matches_original() {
        use fractalwonder_core::{ComplexDelta, HDRComplex, HDRFloat};

        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        // Test several delta values
        let test_deltas = [
            (0.1, 0.05),
            (0.01, 0.01),
            (-0.05, 0.1),
        ];

        for (dx, dy) in test_deltas {
            let delta_hdr = HDRComplex {
                re: HDRFloat::from_f64(dx),
                im: HDRFloat::from_f64(dy),
            };

            let original = compute_pixel_perturbation_hdr(&orbit, delta_hdr, 500, TEST_TAU_SQ);
            let generic = compute_pixel_perturbation_generic(&orbit, delta_hdr, 500, TEST_TAU_SQ);

            assert_eq!(
                original.iterations, generic.iterations,
                "Iteration mismatch for delta ({}, {})",
                dx, dy
            );
            assert_eq!(
                original.escaped, generic.escaped,
                "Escaped mismatch for delta ({}, {})",
                dx, dy
            );
        }
    }

    #[test]
    fn generic_bigfloat_matches_original() {
        use fractalwonder_core::BigFloatComplex;

        let precision = 256;
        let c_ref = (BigFloat::with_precision(-0.5, precision), BigFloat::zero(precision));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        let test_deltas = [
            (0.1, 0.05),
            (0.01, 0.01),
        ];

        for (dx, dy) in test_deltas {
            let delta_re = BigFloat::with_precision(dx, precision);
            let delta_im = BigFloat::with_precision(dy, precision);

            let original = compute_pixel_perturbation_bigfloat(
                &orbit, &delta_re, &delta_im, 500, TEST_TAU_SQ
            );
            let generic = compute_pixel_perturbation_generic(
                &orbit,
                BigFloatComplex::new(delta_re, delta_im),
                500,
                TEST_TAU_SQ,
            );

            assert_eq!(
                original.iterations, generic.iterations,
                "Iteration mismatch for delta ({}, {})",
                dx, dy
            );
            assert_eq!(
                original.escaped, generic.escaped,
                "Escaped mismatch for delta ({}, {})",
                dx, dy
            );
        }
    }
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-compute generic_hdr_matches generic_bigfloat_matches`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test(compute): add equivalence tests for generic perturbation function"
```

---

## Task 6: Update Worker to Use Generic Function

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Update imports in worker.rs**

Find the import block at top of `fractalwonder-compute/src/worker.rs` and update:

```rust
use crate::{
    compute_pixel_perturbation_generic, compute_pixel_perturbation_hdr_bla, BlaTable,
    ReferenceOrbit,
};
use fractalwonder_core::{BigFloatComplex, ComplexDelta, F64Complex, HDRComplex, HDRFloat};
```

**Step 2: Update f64 path (around line 400)**

Replace:
```rust
let result =
    compute_pixel_perturbation(&orbit, delta_c, max_iterations, tau_sq);
```

With:
```rust
let result = compute_pixel_perturbation_generic(
    &orbit,
    F64Complex::from_f64_pair(delta_c.0, delta_c.1),
    max_iterations,
    tau_sq,
);
```

**Step 3: Update HDR path without BLA (around line 445)**

Replace:
```rust
compute_pixel_perturbation_hdr(&orbit, delta_c, max_iterations, tau_sq)
```

With:
```rust
compute_pixel_perturbation_generic(&orbit, delta_c, max_iterations, tau_sq)
```

**Step 4: Update BigFloat path (around line 463)**

Replace:
```rust
let result = compute_pixel_perturbation_bigfloat(
    &orbit,
    &delta_c_re,
    &delta_c_row_im,
    max_iterations,
    tau_sq,
);
```

With:
```rust
let result = compute_pixel_perturbation_generic(
    &orbit,
    BigFloatComplex::new(delta_c_re.clone(), delta_c_row_im.clone()),
    max_iterations,
    tau_sq,
);
```

**Step 5: Update lib.rs exports**

Modify `fractalwonder-compute/src/lib.rs` exports to include generic function:

```rust
pub use perturbation::{
    compute_pixel_perturbation_generic, compute_pixel_perturbation_hdr_bla, ReferenceOrbit,
};
```

**Step 6: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/worker.rs fractalwonder-compute/src/lib.rs
git commit -m "refactor(compute): switch worker to use generic perturbation function"
```

---

## Task 7: Delete Old Functions and Dead Code

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Delete old functions from perturbation.rs**

Remove these functions (keep their tests for now):
- `compute_pixel_perturbation` (f64 version, ~lines 630-778)
- `compute_pixel_perturbation_hdr` (~lines 251-419)
- `compute_pixel_perturbation_bigfloat` (~lines 94-247)
- `compute_pixel_perturbation_bla` (dead code, ~lines 782-929)

**Step 2: Rename generic function**

Rename `compute_pixel_perturbation_generic` to `compute_pixel_perturbation`:

Find and replace in perturbation.rs:
- `pub fn compute_pixel_perturbation_generic<D: ComplexDelta>` → `pub fn compute_pixel_perturbation<D: ComplexDelta>`

**Step 3: Update lib.rs exports**

```rust
pub use perturbation::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, ReferenceOrbit,
};
```

**Step 4: Update worker.rs imports**

```rust
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, BlaTable,
    ReferenceOrbit,
};
```

**Step 5: Update all test references**

Find and replace in perturbation.rs tests:
- `compute_pixel_perturbation_generic` → `compute_pixel_perturbation`

Update tests that called old functions to use new generic function with appropriate type.

**Step 6: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS

**Step 7: Run clippy and format**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all`
Expected: No errors or warnings

**Step 8: Commit**

```bash
git add -A
git commit -m "refactor(compute): delete duplicate perturbation functions, rename generic to compute_pixel_perturbation"
```

---

## Task 8: Benchmark and Verify

**Step 1: Build release version**

Run: `trunk build --release`

**Step 2: Manual visual verification**

Open the app and verify:
- Shallow zoom (f64 path) renders correctly
- Medium zoom (HDR path) renders correctly
- Deep zoom (BigFloat path) renders correctly
- BLA acceleration still works

**Step 3: Check line count reduction**

Run: `wc -l fractalwonder-compute/src/perturbation.rs`
Expected: ~1100 lines (down from ~1983)

**Step 4: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore: final cleanup after perturbation refactoring"
```

---

## Summary

After completing all tasks:
- **4 duplicate functions** replaced with **1 generic function**
- **1 dead function** deleted (`compute_pixel_perturbation_bla`)
- `compute_pixel_perturbation_hdr_bla` kept separate for BLA optimization
- ~880 lines removed (~44% reduction)
- Zero runtime overhead due to monomorphization
