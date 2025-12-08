# GPU HDRFloat Precision Fix Design

## Problem

GPU Mandelbrot renderer produces ~1% glitched pixels with incorrect iteration counts. Investigation revealed the GPU HDRFloat implementation diverges from CPU, causing premature escape detection.

**Evidence (from `debug_glitched_pixels_gpu_vs_cpu` test):**
- GPU vs CPU HDRFloat: avg diff = 1921 iterations, max diff = 22,651
- GPU almost always underestimates iteration counts (escapes too early)
- CPU HDRFloat produces clean images - glitches are GPU-specific

## Root Cause

Three magnitude comparisons in the shader use f32 instead of extended precision:

| Check | Purpose | CPU | GPU (current) |
|-------|---------|-----|---------------|
| Escape | Stop when \|z\| > escape_radius | f64 | f32 |
| Rebase | Reset when \|z\| < \|δz\| | f64 | f32 |
| Glitch | Detect when \|z\| < τ\|Z\| | f64 | f32 |

The critical issue is the **rebase check**. When `hdr_complex_norm_sq()` converts to f32, it loses precision for small values. This causes incorrect rebase decisions, leading to numerically unstable iteration and premature escape.

## Solution

Add HDRFloat comparison functions and use them for all magnitude checks.

### New Functions

```wgsl
// Return norm_sq as HDRFloat instead of f32
fn hdr_complex_norm_sq_hdr(a: HDRComplex) -> HDRFloat {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    return hdr_add(re_sq, im_sq);
}

// Compare two HDRFloat values: a < b
fn hdr_less_than(a: HDRFloat, b: HDRFloat) -> bool {
    // Handle zeros
    if a.head == 0.0 { return b.head > 0.0; }
    if b.head == 0.0 { return a.head < 0.0; }

    // Different signs
    if a.head < 0.0 && b.head > 0.0 { return true; }
    if a.head > 0.0 && b.head < 0.0 { return false; }

    // Same sign - compare using exponents first
    let a_positive = a.head > 0.0;
    if a.exp != b.exp {
        if a_positive {
            return a.exp < b.exp;
        } else {
            return a.exp > b.exp;
        }
    }

    // Same exponent - compare mantissas
    let a_val = a.head + a.tail;
    let b_val = b.head + b.tail;
    return a_val < b_val;
}

// Compare: a > b
fn hdr_greater_than(a: HDRFloat, b: HDRFloat) -> bool {
    return hdr_less_than(b, a);
}

// Create HDRFloat from f32 constant (for escape_radius_sq, tau_sq)
fn hdr_from_f32(val: f32) -> HDRFloat {
    if val == 0.0 { return HDR_ZERO; }
    // Normalize to [0.5, 1.0) range
    return hdr_normalize(HDRFloat(val, 0.0, 0));
}
```

### Updated Iteration Loop

```wgsl
// Precompute constants as HDRFloat
let escape_radius_sq_hdr = hdr_from_f32(uniforms.escape_radius_sq);

// In the loop:
let z_mag_sq_hdr = hdr_complex_norm_sq_hdr(z);
let dz_mag_sq_hdr = hdr_complex_norm_sq_hdr(dz);

// 1. Escape check
if hdr_greater_than(z_mag_sq_hdr, escape_radius_sq_hdr) {
    escaped_buf[linear_idx] = 1u;
    results[linear_idx] = n;
    // ... (use hdr_to_f32 for final z_norm_sq output)
    return;
}

// 2. Glitch detection
let z_m_mag_sq_hdr = hdr_from_f32(z_m_re * z_m_re + z_m_im * z_m_im);
let tau_z_m_sq = hdr_mul_f32(z_m_mag_sq_hdr, uniforms.tau_sq);
if hdr_greater_than(z_m_mag_sq_hdr, hdr_from_f32(1e-20)) &&
   hdr_less_than(z_mag_sq_hdr, tau_z_m_sq) {
    glitched = true;
}

// 3. Rebase check (the critical fix)
if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr) {
    dz = z;
    m = 0u;
    continue;
}
```

## Files to Modify

1. `fractalwonder-gpu/src/shaders/hdrfloat.wgsl`
   - Add `hdr_complex_norm_sq_hdr()`
   - Add `hdr_less_than()`, `hdr_greater_than()`
   - Add `hdr_from_f32()`

2. `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`
   - Update escape check to use HDRFloat comparison
   - Update glitch detection to use HDRFloat comparison
   - Update rebase check to use HDRFloat comparison

3. `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl` (if exists and uses same pattern)
   - Apply same changes for consistency

## Verification

1. Run existing test: `cargo test -p fractalwonder-gpu debug_glitched_pixels_gpu_vs_cpu -- --nocapture`
   - Before: avg diff ~1921 iterations
   - After: avg diff should be < 10 iterations (ideally ±1)

2. Visual comparison at test coordinates:
   - Center: (-1.2627, -0.4084), zoom 10^6.66
   - Should produce clean image matching CPU render

3. Performance check:
   - HDRFloat comparison adds ~6 operations per check
   - 3 checks per iteration = ~18 extra ops
   - Expected impact: < 5% slowdown (acceptable)

## Risks

1. **Edge cases in comparison**: Subnormal values, extreme exponents
   - Mitigation: Test with known edge cases from CPU test suite

2. **Numerical stability**: Comparison logic must handle all sign/exponent combinations
   - Mitigation: Port logic from CPU implementation where possible

3. **Performance regression**: More operations per iteration
   - Mitigation: Profile before/after, optimize if needed
