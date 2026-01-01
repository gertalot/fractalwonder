# COMPLETE Forensic Comparison: CPU vs GPU Mandelbrot Iteration

**Files compared:**
- CPU: `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs`
- GPU: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`
- CPU HDRFloat: `fractalwonder-core/src/hdrfloat.rs`
- CPU HDRComplex: `fractalwonder-core/src/hdrcomplex.rs`

---

## ALL DIFFERENCES FOUND

| # | Category | CPU | GPU | Impact |
|---|----------|-----|-----|--------|
| 1 | **Pixel coord construction** | N/A (param) | `HDRFloat(f32(col), 0, 0)` UNNORMALIZED | **CRITICAL** |
| 2 | Orbit storage | f64 pairs | HDRFloat (head/tail/exp) | Medium |
| 3 | Orbit loading | f64 values | Pre-converted HDRFloat | Medium |
| 4 | z_mag_sq output | `to_f64()` → f64 | `hdr_to_f32()` → f32 | **HIGH** |
| 5 | z_m_mag_sq computation | `z_m_re * z_m_re` (f64×f64) | Via HDR→f32 path | **HIGH** |
| 6 | Escape check | f64 > 65536.0 | HDR > HDR | Low |
| 7 | Glitch detection values | f64 | f32 | **HIGH** |
| 8 | Rebase comparison | `sub().is_negative()` | `hdr_less_than()` | Low |
| 9 | Delta iter: orbit×delta | `mul_f64(f64)` | `hdr_mul(HDR)` | **HIGH** |
| 10 | hdr_add exp_diff | `saturating_sub` | plain `-` | Low |
| 11 | hdr_square exp | `saturating_mul(2)` | `* 2` | Low |
| 12 | hdr_mul exp | `saturating_add` | `+` | Low |
| 13 | Subnormal handling | Special path | Not handled | Low |
| 14 | complex_square ×2 | `saturating_add(1)` | `+ 1` | Low |
| 15 | BLA acceleration | Yes | **NO** | Performance only |

---

## DETAILED ANALYSIS

### DIFFERENCE #1: UNNORMALIZED PIXEL COORDINATES (CRITICAL BUG)

**GPU (progressive_iteration.wgsl:327-331):**
```wgsl
let x_hdr = HDRFloat(f32(col), 0.0, 0);        // BUG: head=500.0, exp=0
let y_hdr = HDRFloat(f32(global_row), 0.0, 0); // BUG: head=300.0, exp=0
let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
```

**What CPU does (via HDRFloat::from_f64):**
- `from_f64(500.0)` → `{ head: 0.9765625, tail: 0, exp: 9 }` (normalized!)

**Why this causes streaks:**
- For col=500: GPU head=500.0, CPU head=0.977
- For col=501: GPU head=501.0, CPU head=0.978
- The FMA error computation `fma(a.head, b.head, -p)` behaves differently for each column
- Creates column-dependent precision errors → visible vertical streaks

**Fix:** Use `hdr_from_f32_const(f32(col))` which calls `hdr_normalize()`.

---

### DIFFERENCE #2-3: ORBIT STORAGE AND LOADING

**CPU (reference_orbit.rs + pixel_hdr_bla.rs:62-63):**
```rust
// Storage: Vec<(f64, f64)>
let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];  // Load f64
```

**GPU (progressive_renderer.rs:108 + progressive_iteration.wgsl:366-385):**
```rust
// Upload: converts f64 → HDRFloat
let z_re_hdr = HDRFloat::from_f64(z_re);
// ...stored as 12 f32s per point...

// Load in shader:
let z_m_hdr_re = HDRFloat(z_m_re_head, z_m_re_tail, z_m_re_exp);
```

**Impact:** The f64→HDRFloat conversion happens at upload time (GPU) vs per-iteration (CPU). Mathematically equivalent, but different precision accumulation.

---

### DIFFERENCE #4-5: MAGNITUDE COMPUTATION (HIGH IMPACT)

**CPU (pixel_hdr_bla.rs:71-73):**
```rust
let z_mag_sq_hdr = z_re.square().add(&z_im.square());
let z_mag_sq = z_mag_sq_hdr.to_f64();              // 53 bits precision
let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im; // Direct f64 * f64
```

**GPU (progressive_iteration.wgsl:391-398):**
```wgsl
let z_mag_sq_hdr = hdr_complex_norm_sq_hdr(z);
let z_mag_sq = hdr_to_f32(z_mag_sq_hdr);           // 24 bits precision!
let z_m_mag_sq = hdr_to_f32(hdr_complex_norm_sq_hdr(...)); // Via HDR→f32
```

**Impact:**
- z_mag_sq: CPU has 53-bit precision, GPU has 24-bit
- z_m_mag_sq: CPU computes directly in f64, GPU goes through HDR path

---

### DIFFERENCE #6: ESCAPE CHECK

**CPU (pixel_hdr_bla.rs:78):**
```rust
if z_mag_sq > 65536.0  // f64 comparison
```

**GPU (progressive_iteration.wgsl:401-402):**
```wgsl
let escape_radius_sq_hdr = hdr_from_f32_const(uniforms.escape_radius_sq);
if hdr_greater_than(z_mag_sq_hdr, escape_radius_sq_hdr)  // HDR comparison
```

**Impact:** Low - threshold is coarse (65536), but slightly different timing is possible.

---

### DIFFERENCE #7: GLITCH DETECTION (HIGH IMPACT)

**CPU (pixel_hdr_bla.rs:105):**
```rust
if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq
// All values are f64
```

**GPU (progressive_iteration.wgsl:428):**
```wgsl
if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq
// All values are f32!
```

**Impact:** At deep zoom with very small magnitudes:
- f32 has only ~7 decimal digits vs f64's ~15
- Glitch detection may trigger at different points

---

### DIFFERENCE #8: REBASE COMPARISON

**CPU (pixel_hdr_bla.rs:111):**
```rust
if z_mag_sq_hdr.sub(&dz_mag_sq).is_negative()
// Computes a - b, checks if result.head < 0
```

**GPU (progressive_iteration.wgsl:437):**
```wgsl
if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr)
// Compares exponents first, then mantissas
```

**CPU is_negative (hdrfloat.rs:38-40):**
```rust
pub fn is_negative(&self) -> bool {
    self.head < 0.0
}
```

**GPU hdr_less_than (progressive_iteration.wgsl:155-169):**
```wgsl
fn hdr_less_than(a: HDRFloat, b: HDRFloat) -> bool {
    // Handle zeros
    if a_zero { return !b_zero; }
    if b_zero { return false; }
    // Compare exponents first
    if a.exp != b.exp { return a.exp < b.exp; }
    // Same exponent - compare mantissas
    return (a.head + a.tail) < (b.head + b.tail);
}
```

**Impact:** Mathematically equivalent for positive values, but edge cases (near zero, identical values) may differ.

---

### DIFFERENCE #9: DELTA ITERATION - ORBIT MULTIPLICATION (HIGH IMPACT)

This is where the main iteration happens: `δz' = 2·Z_m·δz + δz² + δc`

**CPU (pixel_hdr_bla.rs:138-147):**
```rust
let two_z_dz_re = dz.re
    .mul_f64(z_m_re)           // HDRFloat × f64 (uses mul_f64)
    .sub(&dz.im.mul_f64(z_m_im))
    .mul_f64(2.0);
```

**GPU (progressive_iteration.wgsl:453-454):**
```wgsl
let two_z_dz_re = hdr_mul_f32(
    hdr_sub(
        hdr_mul(dz.re, z_m_hdr_re),  // HDRFloat × HDRFloat (uses hdr_mul)
        hdr_mul(dz.im, z_m_hdr_im)
    ), 2.0);
```

**Key difference in multiplication:**

**CPU mul_f64 (hdrfloat.rs:348-368):**
```rust
pub fn mul_f64(&self, scalar: f64) -> Self {
    // Split f64 into head + tail (preserves ~48 bits of f64 precision)
    let s_head = scalar as f32;
    let s_tail = (scalar - s_head as f64) as f32;

    let p = self.head * s_head;
    let err = self.head.mul_add(s_head, -p);
    let tail = err + self.head * s_tail + self.tail * s_head;

    Self { head: p, tail, exp: self.exp }  // EXPONENT UNCHANGED
        .normalize()
}
```

**GPU hdr_mul (progressive_iteration.wgsl:67-73):**
```wgsl
fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;
    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));  // EXPONENTS ADD
}
```

**Critical differences:**
1. **CPU splits f64 at runtime**: `s_tail = (scalar - s_head as f64) as f32`
   - This captures more precision from the f64 value
2. **GPU uses pre-split HDRFloat**: `from_f64` was called at upload
   - The split happened once at upload, not per-iteration
3. **Exponent handling**:
   - CPU: `exp: self.exp` (unchanged, f64 scalar carries its magnitude implicitly)
   - GPU: `a.exp + b.exp` (both operands contribute to exponent)

**Why they're mathematically equivalent but numerically different:**
- For z_m = 1.5:
  - CPU: scalar=1.5 → s_head=1.5, s_tail≈0, exp unchanged
  - GPU: HDRFloat{head=0.75, tail≈0, exp=1} → exp adds to result
- The math is the same, but rounding errors accumulate differently

---

### DIFFERENCES #10-14: SATURATION VS PLAIN ARITHMETIC

**CPU uses saturating arithmetic for exponents:**
```rust
self.exp.saturating_add(other.exp)   // Won't overflow
self.exp.saturating_sub(other.exp)   // Won't underflow
self.exp.saturating_mul(2)           // Won't overflow
```

**GPU uses plain arithmetic:**
```wgsl
a.exp + b.exp    // Can wrap around on i32 overflow
a.exp - b.exp    // Can wrap around on i32 underflow
a.exp * 2        // Can overflow
```

**Impact:** Only affects extreme exponents (near i32::MAX or i32::MIN). At 10^277 zoom, exponents are around -920, well within safe range.

---

### DIFFERENCE #15: BLA ACCELERATION

**CPU:** Has full BLA (Bivariate Linear Approximation) support
**GPU:** No BLA - every iteration is computed

**Impact:** Performance only, not correctness. But more iterations = more accumulated precision errors on GPU.

---

## SUMMARY: ROOT CAUSES OF VISIBLE ARTIFACTS

### Primary Cause: Unnormalized Pixel Coordinates (Difference #1)
- Creates column-dependent precision errors
- Manifests as vertical streaks in gradient areas
- **FIX: `let x_hdr = hdr_from_f32_const(f32(col));`**

### Secondary Causes:
1. **f64 vs f32 for magnitude values** (#4, #5, #7)
   - Affects escape timing and glitch detection
   - May cause subtle color banding

2. **mul_f64 vs hdr_mul** (#9)
   - Different precision accumulation over thousands of iterations
   - May cause "rougher" detail in GPU render

3. **No BLA on GPU** (#15)
   - More iterations = more accumulated errors

---

## RECOMMENDED FIXES

### Must Fix (causes visible artifacts):
```wgsl
// Line 327-328: Replace with:
let x_hdr = hdr_from_f32_const(f32(col));
let y_hdr = hdr_from_f32_const(f32(global_row));
```

### Consider Fixing (improves quality):
1. Use HDRFloat comparison for glitch detection instead of f32
2. Add `hdr_mul_f64` equivalent that splits f64 at runtime (if WebGPU ever supports f64)
3. Implement GPU BLA for fewer iterations = fewer accumulated errors
