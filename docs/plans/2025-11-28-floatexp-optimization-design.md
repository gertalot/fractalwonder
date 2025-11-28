# FloatExp Optimization Design

> **Implementation Plan:** After approval, use `superpowers:writing-plans` to create detailed implementation tasks.

**Goal:** Replace BigFloat delta arithmetic with FloatExp for 10-20x speedup in perturbation rendering.

**Context:** BigFloat perturbation (Increment 2) is correct but too slow for interactive use. FloatExp provides unlimited range with fixed 53-bit precision using fast f64 hardware operations.

---

## 1. Core Type

```rust
/// Extended-range floating point: f64 mantissa + i64 exponent.
/// Provides unlimited range with 53-bit precision.
/// Value = mantissa × 2^exp (or 0 if mantissa == 0)
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatExp {
    mantissa: f64,  // Normalized: 0.5 ≤ |mantissa| < 1.0, or 0.0
    exp: i64,
}
```

**Design decisions:**
- `Copy` - Cheap 16-byte value type, no heap allocation
- Private fields - Enforce normalization invariant via constructors
- Standard [0.5, 1.0) mantissa convention - Matches frexp/ldexp semantics

---

## 2. Constructors

```rust
impl FloatExp {
    /// Zero value
    pub fn zero() -> Self {
        Self { mantissa: 0.0, exp: 0 }
    }

    /// Create from f64 (normalizes automatically)
    pub fn from_f64(val: f64) -> Self {
        if val == 0.0 {
            return Self::zero();
        }
        let (mantissa, exp) = frexp(val);  // Returns mantissa in [0.5, 1.0)
        Self { mantissa, exp: exp as i64 }
    }

    /// Create from raw parts (normalizes to maintain invariant)
    pub fn from_parts(mantissa: f64, exp: i64) -> Self {
        Self { mantissa, exp }.normalize()
    }

    /// Convert from BigFloat
    pub fn from_bigfloat(bf: &BigFloat) -> Self {
        // Extract f64 mantissa and adjust exponent
        // Implementation depends on BigFloat internals
        ...
    }

    /// Normalize mantissa to [0.5, 1.0) invariant
    fn normalize(self) -> Self {
        if self.mantissa == 0.0 {
            return Self::zero();
        }
        let (m, e) = frexp(self.mantissa);
        Self {
            mantissa: m,
            exp: self.exp + e as i64,
        }
    }
}
```

---

## 3. Arithmetic Operations

### Multiplication (fast)

```rust
pub fn mul(&self, other: &Self) -> Self {
    if self.mantissa == 0.0 || other.mantissa == 0.0 {
        return Self::zero();
    }
    Self {
        mantissa: self.mantissa * other.mantissa,  // 1 f64 multiply
        exp: self.exp + other.exp,                  // 1 i64 add
    }.normalize()
}
```

### Addition (exponent alignment)

```rust
pub fn add(&self, other: &Self) -> Self {
    if self.mantissa == 0.0 { return *other; }
    if other.mantissa == 0.0 { return *self; }

    let exp_diff = self.exp - other.exp;

    // If difference > 53 bits, smaller value is negligible
    if exp_diff > 53 { return *self; }
    if exp_diff < -53 { return *other; }

    // Align to larger exponent, add mantissas
    let (mantissa, exp) = if exp_diff >= 0 {
        let scaled_other = other.mantissa * exp2(-exp_diff as f64);
        (self.mantissa + scaled_other, self.exp)
    } else {
        let scaled_self = self.mantissa * exp2(exp_diff as f64);
        (scaled_self + other.mantissa, other.exp)
    };

    Self { mantissa, exp }.normalize()
}
```

### Subtraction

```rust
pub fn sub(&self, other: &Self) -> Self {
    self.add(&other.neg())
}
```

### Negation

```rust
pub fn neg(&self) -> Self {
    Self {
        mantissa: -self.mantissa,
        exp: self.exp,
    }
}
```

### Multiply by f64 (for 2·Z·δz where Z is f64)

```rust
pub fn mul_f64(&self, scalar: f64) -> Self {
    if self.mantissa == 0.0 || scalar == 0.0 {
        return Self::zero();
    }
    Self {
        mantissa: self.mantissa * scalar,
        exp: self.exp,
    }.normalize()
}
```

---

## 4. Conversion & Utilities

```rust
impl FloatExp {
    /// Convert to f64 (may overflow/underflow for extreme exponents)
    pub fn to_f64(&self) -> f64 {
        if self.mantissa == 0.0 {
            return 0.0;
        }
        // ldexp: mantissa × 2^exp
        // Handle extreme exponents gracefully
        if self.exp > 1023 {
            return if self.mantissa > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
        }
        if self.exp < -1074 {
            return 0.0;
        }
        ldexp(self.mantissa, self.exp as i32)
    }

    /// Squared magnitude of complex number (re, im)
    /// Returns f64 since result is bounded for escape testing
    pub fn norm_sq(re: &FloatExp, im: &FloatExp) -> f64 {
        let re_sq = re.mul(re);
        let im_sq = im.mul(im);
        re_sq.add(&im_sq).to_f64()
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.mantissa == 0.0
    }
}
```

---

## 5. Perturbation Integration

### New function

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

---

## 6. File Structure

```
fractalwonder-core/src/
├── bigfloat.rs          # Unchanged
├── floatexp.rs          # NEW - FloatExp type + arithmetic
├── lib.rs               # Add: pub mod floatexp; pub use floatexp::FloatExp;

fractalwonder-compute/src/
├── perturbation.rs      # Add: compute_pixel_perturbation_floatexp()
├── lib.rs               # Export new function
```

---

## 7. Testing Strategy

### Unit tests (floatexp.rs)

1. **Normalization invariant**: After any operation, mantissa is in [0.5, 1.0) or zero
2. **Arithmetic correctness**: Results match f64 for values in f64 range
3. **Extreme exponents**: Handles 10^±1000 without panic or overflow
4. **Edge cases**: Zero handling, sign preservation, exponent alignment

### Cross-validation test (perturbation.rs)

```rust
#[test]
fn floatexp_matches_bigfloat_at_deep_zoom() {
    // At 10^500 zoom, both should produce identical iteration counts
    let c_ref = ...;
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);
    let delta = ...; // 10^-500 scale

    let bigfloat_result = compute_pixel_perturbation_bigfloat(&orbit, &delta_bf, 1000, TAU_SQ);
    let floatexp_result = compute_pixel_perturbation_floatexp(&orbit, delta_fe, 1000, TAU_SQ);

    assert_eq!(bigfloat_result.iterations, floatexp_result.iterations);
    assert_eq!(bigfloat_result.escaped, floatexp_result.escaped);
}
```

### Performance benchmark

Compare render time at 10^500 zoom:
- BigFloat version (baseline)
- FloatExp version (target: 10x+ faster)

---

## 8. Message Protocol

Update `RenderTilePerturbation` to use FloatExp:

```rust
RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    delta_c_origin: (FloatExp, FloatExp),  // Direct serialization
    delta_c_step: (FloatExp, FloatExp),
    max_iterations: u32,
    tau_sq: f64,
}
```

FloatExp derives `Serialize`/`Deserialize`, so no JSON string workaround needed.

---

## 9. Acceptance Criteria

1. All existing tests pass
2. FloatExp matches BigFloat iteration counts exactly (cross-validation)
3. Measurable speedup vs BigFloat (target: 10x+)
4. No new clippy warnings
5. Code formatted with rustfmt

---

*Design complete. Ready for implementation planning.*
