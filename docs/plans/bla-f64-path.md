# Implement BLA for f64 Path

## Problem Statement

Currently, BLA (Bivariate Linear Approximation) iteration skipping is only available in the HDR path (`render_tile_hdr`). The f64 path (`render_tile_f64`) has no BLA support.

At moderate zoom levels (up to ~10^300), f64 arithmetic works fine and is **~50x faster** than HDRFloat. However, when BLA is enabled, we currently force all tiles through the slow HDR path to get BLA benefits.

**The result**: With BLA enabled at 10^270 zoom, rendering is **slower** than without BLA because:
- f64 without BLA: 30,000 fast iterations = 30,000 work units
- HDR with BLA: 900 slow iterations × 50 = 45,000 work units

**The goal**: Implement f64+BLA for the best of both worlds:
- f64 with BLA: 900 fast iterations = **900 work units** (33x faster than current)

## Current Architecture

### File Structure
```
fractalwonder-compute/src/
├── bla.rs                      # BLA table construction and lookup
├── perturbation/
│   ├── mod.rs                  # Exports
│   ├── pixel.rs                # Generic perturbation (no BLA)
│   ├── pixel_hdr_bla.rs        # HDR perturbation WITH BLA
│   └── tile.rs                 # render_tile_f64 and render_tile_hdr
└── worker.rs                   # Dispatch logic (line 260)
```

### Current Dispatch Logic (worker.rs:254-283)
```rust
// Dispatch based on delta magnitude and BLA requirement
// BLA is only supported in HDR path, so force HDR when BLA is enabled
let delta_log2 = delta_c_origin.0.log2_approx().max(delta_c_origin.1.log2_approx());
let use_f64 = !force_hdr_float && !bla_enabled && delta_log2 > -900.0 && delta_log2 < 900.0;

let result = if use_f64 {
    render_tile_f64(&orbit, delta_origin, delta_step, &config)  // No BLA
} else {
    render_tile_hdr(&orbit, cached.bla_table.as_ref(), delta_origin, delta_step, &config)  // With BLA
};
```

### BLA Data Structures (bla.rs)

```rust
/// Single BLA entry: skips `l` iterations.
/// Applies: δz_new = A·δz + B·δc
pub struct BlaEntry {
    pub a: HDRComplex,      // Complex coefficient A (multiplies δz)
    pub b: HDRComplex,      // Complex coefficient B (multiplies δc)
    pub l: u32,             // Number of iterations to skip
    pub r_sq: HDRFloat,     // Validity radius squared
}

pub struct BlaTable {
    pub entries: Vec<BlaEntry>,
    pub level_offsets: Vec<usize>,  // Binary tree structure
    pub num_levels: usize,
    dc_max: HDRFloat,               // Maximum |delta_c| for validity
}
```

### BLA Lookup (bla.rs:180-238)
```rust
pub fn find_valid(&self, m: usize, dz_mag_sq: &HDRFloat, dc_max: &HDRFloat) -> Option<&BlaEntry>
```
- Searches from highest level (largest skips) down to level 0
- Checks alignment: `m.is_multiple_of(skip_size)`
- Checks validity: `|δz|² < r²`
- Checks B coefficient: `|B| × dc_max ≤ 2^0`

### BLA Application (pixel_hdr_bla.rs:125-133)
```rust
if let Some(bla) = bla_entry {
    // Apply BLA: δz_new = A·δz + B·δc
    let a_dz = bla.a.mul(&dz);
    let b_dc = bla.b.mul(&delta_c);
    dz = a_dz.add(&b_dc);

    bla_iters += bla.l;
    m += bla.l as usize;
    n += bla.l;
}
```

## Why BLA Coefficients Use HDRFloat

BLA coefficients are stored as HDRFloat because at **extreme** zoom levels (10^500+), the coefficients A and B can overflow f64 range (10^308). This happens because:

1. Level-0 BLA: `A = 2*Z`, `B = 1` - normal f64 values
2. Merged BLAs: `A_merged = A_y × A_x` - coefficients multiply
3. At high levels (large skips): coefficients can grow huge

**However**, at moderate zoom levels (10^270 and below), the coefficients typically stay within f64 range.

## Implementation Approach

### Option A: Convert BLA Coefficients to f64 at Lookup Time

Create `find_valid_f64()` that returns f64 coefficients:

```rust
pub struct BlaEntryF64 {
    pub a: (f64, f64),  // Complex A as (re, im)
    pub b: (f64, f64),  // Complex B as (re, im)
    pub l: u32,
    pub r_sq: f64,
}

impl BlaTable {
    pub fn find_valid_f64(&self, m: usize, dz_mag_sq: f64, dc_max: f64) -> Option<BlaEntryF64> {
        // Same search logic, but convert to f64 and check for overflow
        let entry = self.find_valid_internal(m, ...)?;

        // Try to convert - return None if overflow
        let a_re = entry.a.re.to_f64();
        let a_im = entry.a.im.to_f64();
        if !a_re.is_finite() || !a_im.is_finite() { return None; }

        // ... same for b, r_sq

        Some(BlaEntryF64 { a: (a_re, a_im), b: (b_re, b_im), l: entry.l, r_sq })
    }
}
```

### Option B: Maintain Separate f64 BLA Table

Build a parallel f64 BLA table during construction, truncating levels where coefficients overflow:

```rust
pub struct BlaTableF64 {
    pub entries: Vec<BlaEntryF64>,
    pub level_offsets: Vec<usize>,
    pub num_levels: usize,  // May be fewer than HDR table
    dc_max: f64,
}
```

**Pros**: No runtime conversion overhead
**Cons**: Double memory, more complex construction

### Recommended: Option A

Option A is simpler and the conversion overhead is minimal (only done when BLA is found). The f64 check acts as a natural fallback - if coefficients overflow, we skip BLA for that iteration.

## New Function: compute_pixel_perturbation_f64_bla

Create `pixel_f64_bla.rs` with:

```rust
pub fn compute_pixel_perturbation_f64_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> (MandelbrotData, BlaStats) {
    let mut dz = (0.0, 0.0);
    let mut m: usize = 0;
    let mut bla_iters: u32 = 0;
    let mut standard_iters: u32 = 0;
    let mut rebase_count: u32 = 0;

    while n < max_iterations {
        // ... escape check, glitch detection, rebase check (all in f64)

        // Try BLA (with f64 conversion)
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;
        if let Some(bla) = bla_table.find_valid_f64(m, dz_mag_sq, dc_max) {
            // Apply BLA: δz_new = A·δz + B·δc (f64 complex multiply)
            let a_dz = complex_mul_f64(bla.a, dz);
            let b_dc = complex_mul_f64(bla.b, delta_c);
            dz = (a_dz.0 + b_dc.0, a_dz.1 + b_dc.1);

            bla_iters += bla.l;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // Standard iteration (f64)
            // ... existing f64 perturbation logic
        }
    }
}

fn complex_mul_f64(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
```

## Changes Required

### 1. bla.rs - Add f64 lookup method
- Add `BlaEntryF64` struct
- Add `find_valid_f64()` method with overflow checking
- Keep existing HDRFloat methods unchanged

### 2. perturbation/pixel_f64_bla.rs (new file)
- Create `compute_pixel_perturbation_f64_bla()`
- Similar structure to `pixel_hdr_bla.rs` but all f64 arithmetic
- Use `find_valid_f64()` for BLA lookup

### 3. perturbation/tile.rs - Update render_tile_f64
- Pass BLA table to `render_tile_f64`
- Call `compute_pixel_perturbation_f64_bla` when BLA enabled
- Track BLA stats

### 4. worker.rs - Update dispatch logic
```rust
let use_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;
// Remove: && !bla_enabled

let result = if use_f64 {
    render_tile_f64(&orbit, cached.bla_table.as_ref(), delta_origin, delta_step, &config)
} else {
    render_tile_hdr(&orbit, cached.bla_table.as_ref(), delta_origin, delta_step, &config)
};
```

## Testing

1. **Unit tests** for `find_valid_f64()` - verify correct conversion and overflow handling
2. **Unit tests** for `compute_pixel_perturbation_f64_bla()` - compare results with HDR version
3. **Integration test** - render same tile with f64+BLA and HDR+BLA, verify matching iteration counts
4. **Performance benchmark** - measure speedup vs HDR+BLA and f64-only

## Performance Expectations

At 10^270 zoom with 30,000 max iterations and 97% BLA efficiency:

| Path | Iterations | Speed Factor | Work Units |
|------|------------|--------------|------------|
| f64 no BLA | 30,000 | 1x | 30,000 |
| HDR + BLA | 900 | 0.02x (50x slower) | 45,000 |
| **f64 + BLA** | 900 | 1x | **900** |

Expected speedup: **33x faster** than HDR+BLA, **50x faster** than f64 no BLA.

## Edge Cases

1. **BLA coefficients overflow f64**: `find_valid_f64()` returns None, falls back to standard iteration
2. **Deep zoom where f64 underflows**: Dispatch uses HDR path (existing logic)
3. **Rebase with large δz**: After rebase, δz may be too large for high-level BLA, but lower levels still work
4. **dc_max underflows in f64**: At extreme zoom (10^308+), dc_max underflows - use HDR path

## Files to Read Before Starting

1. `fractalwonder-compute/src/bla.rs` - Full file, understand BLA structure
2. `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs` - Template for f64 version
3. `fractalwonder-compute/src/perturbation/pixel.rs` - Existing f64 perturbation (no BLA)
4. `fractalwonder-compute/src/perturbation/tile.rs` - Tile rendering dispatch
5. `fractalwonder-compute/src/worker.rs` - Worker dispatch logic (lines 245-300)
6. `fractalwonder-core/src/hdr_float.rs` - HDRFloat structure for understanding conversions
