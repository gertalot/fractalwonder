# GPU Glitch Detection: Investigation Findings

## Problem Summary

The GPU Mandelbrot renderer produces visible 1px noise scattered throughout images. Initial hypothesis
was that glitched pixels were correctly detected but never re-rendered. **Investigation revealed the
actual problem is different: the GPU HDRFloat implementation diverges from the CPU HDRFloat
implementation, producing incorrect iteration counts.**

## Key Finding: GPU vs CPU Divergence

A debugging test (`debug_glitched_pixels_gpu_vs_cpu` in `fractalwonder-gpu/src/tests.rs`) compared
GPU, CPU HDRFloat, and BigFloat (ground truth) results for glitched pixels:

**Test conditions:**
- Viewport: center (-1.2627, -0.4084), size 8.76e-7 x 5.65e-7
- Zoom: 10^6.66 (moderate zoom, well within HDRFloat capability)
- Max iterations: 22,890
- Reference orbit: 1965 iterations, escaped at iteration 1964
- Glitched pixels: 2727 out of 250,000 (1.1%)

**Results (100 sampled glitched pixels):**
```
GPU vs CPU HDRFloat: avg diff = 1921, max diff = 22,651 iterations
GPU vs BigFloat:     avg diff = 1537, max diff = 22,665 iterations
CPU vs BigFloat:     avg diff = 1174
```

**Critical observation:** GPU almost always **underestimates** iteration counts compared to CPU. This
means GPU pixels escape earlier than they should.

**Example divergences:**
| Pixel | GPU iter | CPU iter | BigFloat | Notes |
|-------|----------|----------|----------|-------|
| (325, 228) | 822 | 22890 | 22890 | GPU escaped, should reach max |
| (277, 197) | 225 | 11641 | 22890 | GPU escaped way too early |
| (317, 214) | 239 | 22890 | 3435 | GPU escaped early |
| (154, 127) | 22890 | 313 | 299 | GPU didn't escape, should have |

## Root Cause Hypothesis

The pattern of GPU escaping too early suggests issues in one of:

1. **Rebasing logic** - GPU might not trigger rebase when it should, causing precision loss and
   premature escape detection
2. **HDRFloat magnitude comparison** - The `z_mag_sq < dz_mag_sq` check uses f32; subtle differences
   from CPU's f64 version could cause different rebase decisions
3. **Reference orbit wraparound** - When `m >= orbit_len`, GPU might handle the short reference orbit
   differently than CPU
4. **HDRFloat normalization edge cases** - `hdr_normalize()` uses bit manipulation that might have
   edge cases not present in CPU version

## Important Context

**The CPU HDRFloat renderer produces clean images** - the glitches are GPU-specific. This rules out:
- Reference orbit quality
- Tau sensitivity
- Fundamental HDRFloat precision limits

**The reference orbit is short** (escaped at iteration 1964) - pixels needing more iterations must
rely heavily on rebasing. Any bug in rebasing logic would be amplified.

## Relevant Code Locations

**GPU shaders (where the bug likely is):**
- `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` - main progressive renderer
- `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl` - HDR tile renderer
- `fractalwonder-gpu/src/shaders/hdrfloat.wgsl` - HDRFloat library

**Key GPU shader sections to examine:**
- Rebasing check (progressive_iteration.wgsl:325-333):
  ```wgsl
  if z_mag_sq < dz_mag_sq {
      dz = z;
      m = 0u;
      continue;
  }
  ```
- HDRFloat magnitude calculation (`hdr_complex_norm_sq`)
- HDRFloat normalization (`hdr_normalize`)
- Reference orbit exhaustion handling (lines 289-291)

**CPU HDRFloat implementation (known working):**
- `fractalwonder-compute/src/perturbation.rs` - `compute_pixel_perturbation_hdr()` (lines 191-280)
- `fractalwonder-core/src/hdrfloat.rs` - HDRFloat type

## Debugging Test

The test `debug_glitched_pixels_gpu_vs_cpu` in `fractalwonder-gpu/src/tests.rs`:
1. Decodes viewport from a URL with known glitches
2. Renders with GPU progressive renderer, caches glitched pixel coordinates
3. For a sample of glitched pixels, compares GPU, CPU HDRFloat, and BigFloat results
4. Reports iteration count differences

Run with: `cargo test -p fractalwonder-gpu debug_glitched_pixels_gpu_vs_cpu -- --nocapture`

Cached glitch data: `target/glitched_pixels_cache.json` (delete to regenerate)

## Investigation Strategy

1. **Compare CPU and GPU rebasing decisions** - Add logging/output to track when each triggers rebase
2. **Check HDRFloat magnitude precision** - The `hdr_complex_norm_sq` function converts to f32 for
   comparison; verify this matches CPU behavior
3. **Test with longer reference orbit** - Find a viewport where reference doesn't escape early to
   isolate rebasing from orbit exhaustion
4. **Verify HDRFloat arithmetic** - Create unit tests comparing GPU HDRFloat operations to CPU

## Original Problem Statement (for context)

The original hypothesis was that glitch detection worked but re-rendering was missing. While
re-rendering is indeed not implemented, the investigation revealed a more fundamental issue: the GPU
produces incorrect iteration counts for many pixels, which then get flagged as glitched.

Fixing the GPU HDRFloat bug may significantly reduce glitch frequency, potentially making re-rendering
unnecessary for most use cases. However, some glitches are mathematically expected (reference orbit
too far from pixel orbit), so a re-rendering mechanism may still be needed eventually.

## Investigation: Orbit Precision (December 2025)

### What Was Fixed

The reference orbit was being uploaded to GPU with precision loss. Originally, orbit values
(f64 pairs) were converted to f32 during upload, losing 29 bits of mantissa precision.

**Solution implemented:**
- Changed orbit buffer from `[f32; 2]` (re, im) to `[f32; 6]` (re_head, re_tail, im_head, im_tail, re_exp, im_exp)
- Upload uses `HDRFloat::from_f64()` to properly split f64 values into normalized HDRFloat representation
- GPU reconstructs `HDRFloat(head, tail, exp)` from the 6 values per orbit point

**Files modified:**
- `fractalwonder-gpu/src/buffers.rs` - orbit buffer size changes
- `fractalwonder-gpu/src/progressive_renderer.rs` - HDRFloat orbit upload
- `fractalwonder-gpu/src/perturbation_hdr_renderer.rs` - same
- `fractalwonder-gpu/src/renderer.rs` - same
- `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` - orbit reading with HDRFloat reconstruction
- `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl` - same

### Verification

The orbit precision fix was verified by checking reconstruction error:
```
Orbit reconstruction check: GPU HDRFloat from orbit matches original f64 to ~1e-16 relative error
```

This confirms the HDRFloat orbit representation is now correct.

### Key Finding

**The orbit precision fix did NOT resolve the GPU/CPU divergence.** The ~43% mismatch rate persists.

This proves the remaining divergence is due to **HDRFloat arithmetic differences** between GPU and
CPU implementations, not orbit precision loss. The GPU consistently underestimates iteration counts
(escapes earlier than CPU), which points to subtle differences in:
- HDRFloat multiplication/addition/subtraction precision
- Rebase decision threshold comparisons
- Normalization edge cases in WGSL vs Rust

### WGSL Shader Validation

WGSL shaders can be validated offline using the `naga` CLI tool (part of wgpu):
```bash
~/.cargo/bin/naga src/shaders/progressive_iteration.wgsl
```

This catches syntax errors and undefined variables before runtime, avoiding opaque "Invalid
ComputePipeline" browser errors.

### Test Added

`gpu_orbit_precision_matches_cpu` in `fractalwonder-gpu/src/tests.rs`:
- Renders same viewport with GPU and CPU HDRFloat
- Compares iteration counts for all pixels
- Reports mismatch rate and examples of divergence

Run with: `cargo test -p fractalwonder-gpu gpu_orbit_precision_matches_cpu -- --nocapture`

## TODO: Known Bugs to Fix

### C1: `hdr_to_f32` Exponent Clamping Bug

**Location:** `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` (and `hdrfloat.wgsl`)

**The bug:** GPU's `hdr_to_f32()` clamps exponents to [-126, 127]:
```wgsl
fn hdr_to_f32(x: HDRFloat) -> f32 {
    let clamped_exp = clamp(x.exp, -126, 127);
    return mantissa * hdr_exp2(clamped_exp);
}
```

CPU's `to_f64()` uses the full exponent range:
```rust
pub fn to_f64(&self) -> f64 {
    libm::ldexp(mantissa, self.exp)  // full i32 range
}
```

**Impact:** At deep zoom (10^38+), HDRFloat exponents can exceed ±126. The GPU computes
incorrect magnitudes, causing wrong glitch detection and rebase decisions.

**Fix needed:** Either:
1. Keep magnitudes as HDRFloat for comparisons (don't convert to f32), or
2. Return a normalized comparison result instead of the raw magnitude

**Note:** This is probably NOT the cause of current divergence at moderate zoom (10^6.66),
since exponents stay within f32 range at that depth. But it will cause problems at deep zoom.

### C3: Exponent Overflow Wrapping vs Saturating

**Location:** `fractalwonder-gpu/src/shaders/hdrfloat.wgsl` — all functions that modify exponents

**The bug:** WGSL uses **wrapping** for i32 overflow, while Rust uses **saturating** arithmetic:

GPU (wraps):
```wgsl
return HDRFloat(new_head, new_tail, exp + exp_adjust);  // wraps on overflow
```

CPU (saturates):
```rust
exp: self.exp.saturating_add(exp_adjust),  // clamps to i32::MAX/MIN
```

If exponent overflows on GPU:
- `i32::MAX + 1` wraps to `i32::MIN` (-2147483648)
- The HDRFloat value becomes completely wrong (sign flip, massive magnitude change)

On CPU, saturating arithmetic preserves at least the direction (stays at max/min).

**Impact:** At extreme zoom with many iterations, exponent arithmetic could overflow. GPU would
produce garbage values while CPU would produce saturated but directionally-correct values.

**Fix needed:** Implement saturating add/multiply for exponents in WGSL:
```wgsl
fn saturating_add_i32(a: i32, b: i32) -> i32 {
    let sum = a + b;
    // Check for overflow: if signs of a and b match but sum sign differs
    if (a > 0 && b > 0 && sum < 0) { return 2147483647; }  // i32::MAX
    if (a < 0 && b < 0 && sum > 0) { return -2147483648; } // i32::MIN
    return sum;
}
```

Apply this to `hdr_normalize`, `hdr_mul`, `hdr_square`, and `hdr_add`.

**Note:** Probably NOT the cause of current divergence at moderate zoom, but will cause
problems at extreme zoom depths.

### C7: Un-normalized HDRFloat for Pixel Coordinates (LIKELY ROOT CAUSE)

**Location:** `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` lines 280-285

**The bug:** GPU creates un-normalized HDRFloats for pixel coordinates:

```wgsl
let x_hdr = HDRFloat(f32(col), 0.0, 0);    // head=500, tail=0, exp=0 — NOT normalized!
let y_hdr = HDRFloat(f32(actual_row), 0.0, 0);

let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
```

CPU (in test) uses proper HDRFloat conversion:

```rust
let dc_re = origin_re_hdr.add(&HDRFloat::from_f64(x as f64).mul(&step_re_hdr));
```

`HDRFloat::from_f64(500.0)` normalizes to `HDRFloat { head: 0.977, tail: ..., exp: 9 }` because
500 = 0.977 × 2^9. The GPU version has `head=500.0, exp=0` which violates the HDRFloat invariant
that head should be in [0.5, 1.0).

**Why this matters:**

HDRFloat arithmetic assumes normalized inputs. While `hdr_mul` and `hdr_add` call `hdr_normalize`
on their outputs, the intermediate calculations may lose precision or produce different results
when inputs violate the normalization invariant.

For example, in `hdr_mul`:
```wgsl
let p = a.head * b.head;              // 500.0 * 0.5 = 250.0 (large intermediate)
let err = fma(a.head, b.head, -p);    // FMA error term computed with un-normalized head
let tail = err + a.head * b.tail + a.tail * b.head;  // Cross terms with un-normalized head
```

The error tracking in double-single arithmetic relies on heads being in a normalized range.
With head=500 instead of head≈0.977, the error computation is wrong.

**Impact:** Every pixel's δc is computed incorrectly from the start. This error propagates
through ALL iterations, causing the GPU to diverge from CPU.

**This is likely the primary cause of the GPU/CPU divergence at moderate zoom.**

**Fix needed:** Add an `hdr_from_f32` function to the shader and use it:

```wgsl
// Add to hdrfloat.wgsl or progressive_iteration.wgsl
fn hdr_from_f32(val: f32) -> HDRFloat {
    if val == 0.0 { return HDR_ZERO; }

    // Extract exponent via bit manipulation (same as hdr_normalize)
    let bits = bitcast<u32>(val);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    if biased_exp == 0u {
        // Subnormal - just normalize
        return hdr_normalize(HDRFloat(val, 0.0, 0));
    }

    // Normal: adjust to [0.5, 1.0) range
    let exp = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let head = bitcast<f32>(new_mantissa_bits | sign);

    return HDRFloat(head, 0.0, exp);
}
```

Then update the δc computation:
```wgsl
let x_hdr = hdr_from_f32(f32(col));      // Properly normalized
let y_hdr = hdr_from_f32(f32(actual_row));

let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
```

**Files to modify:**
- `fractalwonder-gpu/src/shaders/hdrfloat.wgsl` — add `hdr_from_f32` function
- `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` — use `hdr_from_f32` for pixel coords
- `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl` — check if same issue exists

## Next Steps

1. ~~Find and fix the GPU orbit precision bug~~ ✓ Fixed (but didn't resolve divergence)
2. **Investigate GPU HDRFloat arithmetic divergence** - the remaining root cause
3. Compare specific HDRFloat operations (mul, add, normalize) between GPU and CPU
4. If HDRFloat precision can't be improved, implement CPU fallback for glitched pixels

## References

- [Kalles Fraktaler](https://mathr.co.uk/kf/kf.html) - Reference implementation with multi-reference
- [mathr deep zoom theory](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html) -
  Pauldelbrot criterion and rebasing
- [FractalShark](https://github.com/mattsaccount364/FractalShark) - CUDA implementation with HDRFloat
