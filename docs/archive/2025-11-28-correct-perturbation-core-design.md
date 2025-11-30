# Correct Perturbation Core - Design Document

> **Increment 1** from `docs/research/perturbation-theory.md` Section 13.1

## Overview

Fix the perturbation algorithm to be mathematically correct up to ~10^300 zoom. This addresses two bugs in the current implementation:

1. **Wrong rebasing**: Currently switches to "on-the-fly" f64 computation instead of resetting to iteration 0
2. **Incomplete glitch detection**: Only checks reference exhaustion, missing Pauldelbrot criterion

## Algorithm

The corrected algorithm (from research doc Section 8.1):

```
1. δz = 0, m = 0
2. For n in 0..max_iter:
   a. Z_m = orbit[m % orbit.len()]     // wrap-around
   b. z = Z_m + δz                      // full pixel value
   c. Escape: |z|² > 4 → return escaped
   d. Glitch: |z|² < τ²|Z|² → mark glitched (Pauldelbrot criterion)
   e. Rebase: |z|² < |δz|² → δz = z, m = 0, continue
   f. Delta iteration: δz' = 2·Z·δz + δz² + δc
   g. m += 1
3. Return in-set
```

Key changes from current code:
- No "on-the-fly" f64 mode - rebasing resets to iteration 0 of same reference
- Pauldelbrot criterion (`|z|² < τ²|Z|²`) replaces reference exhaustion check
- Wrap-around when `m >= orbit.len()` for non-escaping references

## Configuration

**Threshold τ (tau):**
- Stored as `tau_sq` (τ²) for efficient comparison
- Default: `1e-6` (τ = 10⁻³, the standard value from Kalles Fraktaler)
- Added to `FractalConfig` for now, runtime-configurable UI later

## File Changes

### fractalwonder-ui/src/config.rs

Add `tau_sq` to `FractalConfig`:

```rust
pub struct FractalConfig {
    // ... existing fields ...
    /// Glitch detection threshold squared (τ²).
    /// Default 1e-6 corresponds to τ = 10⁻³ (standard).
    pub tau_sq: f64,
}

// In FRACTAL_CONFIGS mandelbrot entry:
tau_sq: 1e-6,
```

### fractalwonder-core/src/messages.rs

Add `tau_sq` to `RenderTilePerturbation`:

```rust
MainToWorker::RenderTilePerturbation {
    render_id: u32,
    tile: PixelRect,
    orbit_id: u32,
    delta_c_origin: (f64, f64),
    delta_c_step: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,  // NEW
}
```

### fractalwonder-compute/src/perturbation.rs

**Signature change:**
```rust
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,  // NEW
) -> MandelbrotData
```

**Core loop rewrite:** Replace lines 72-172 with the algorithm above. Remove all "on-the-fly" computation logic.

### fractalwonder-compute/src/worker.rs

Pass `tau_sq` from message to `compute_pixel_perturbation`.

### fractalwonder-ui/src/workers/worker_pool.rs

Include `tau_sq` (from `FractalConfig`) in `RenderTilePerturbation` messages.

## Test Strategy

**Keep existing tests:**
- `reference_orbit_*` - orbit computation unchanged
- `perturbation_matches_direct_for_nearby_point` - critical correctness check

**Modify glitch tests:**
- Update to test Pauldelbrot criterion instead of reference exhaustion

**Add new tests:**
1. Rebase triggers when `|z|² < |δz|²`
2. Pauldelbrot detects precision loss at known coordinates
3. No false glitches for normally-escaping pixels
4. Wrap-around works when m exceeds orbit length

**Out of scope:**
- Deep zoom (10^300+) - Increment 2
- BLA acceleration - Increment 4
- Multi-reference - Increment 5

## Success Criteria

- All tests pass
- Glitch overlay (cyan) accurately marks precision loss regions
- No visual artifacts at zoom up to ~10^300
- Net code reduction (removing on-the-fly mode)

## References

- Research doc: `docs/research/perturbation-theory.md` Sections 2.3, 3, 8.1, 13.1
- Pauldelbrot criterion: Section 2.3
- Rebasing: Section 3
- Complete algorithm: Section 8.1
