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

## Next Steps

1. Find and fix the GPU HDRFloat divergence bug
2. Re-run glitch detection test to measure improvement
3. If significant glitches remain, implement CPU fallback for glitched pixels

## References

- [Kalles Fraktaler](https://mathr.co.uk/kf/kf.html) - Reference implementation with multi-reference
- [mathr deep zoom theory](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html) -
  Pauldelbrot criterion and rebasing
- [FractalShark](https://github.com/mattsaccount364/FractalShark) - CUDA implementation with HDRFloat
