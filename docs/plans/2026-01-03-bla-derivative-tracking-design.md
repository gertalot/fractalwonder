# BLA Derivative Tracking Design

Extend BLA (Bivariate Linear Approximation) to correctly track derivatives during iteration skips,
fixing 3D lighting artifacts at deep zoom.

## Problem

BLA accelerates rendering by skipping iterations using linear approximation:
```
δz_{m+L} = A·δz_m + B·δc
```

The derivative delta `δρ` is not updated during BLA skips, causing incorrect surface normals
and visible artifacts in 3D-shaded renders.

## Solution

Extend BLA with derivative coefficients D and E:
```
δρ_{m+L} = C·δρ_m + D·δz_m + E·δc
```

Key insight: **C = A** mathematically (both equal 2·Z_m at single step, same merge formula).
We reuse A instead of storing C.

## Coefficients

### Single-step (from orbit point Z_m, Der_m)

| Coefficient | Formula | Purpose |
|-------------|---------|---------|
| A | 2·Z_m | Multiplies δz (position) and δρ (derivative) |
| B | 1 | Multiplies δc for position |
| D | 2·Der_m | Cross-term: δz contribution to δρ |
| E | 0 | δc contribution to δρ (zero at single step) |

### Merge formulas (BLA_x then BLA_y)

| Merged | Formula |
|--------|---------|
| A | A_y · A_x |
| B | A_y · B_x + B_y |
| D | A_y · D_x + D_y · A_x |
| E | A_y · E_x + D_y · B_x + E_y |

## File Changes

### fractalwonder-compute/src/bla.rs

**BlaEntry struct:**
```rust
pub struct BlaEntry {
    pub a: HDRComplex,      // Multiplies δz and δρ (C = A, not stored)
    pub b: HDRComplex,      // Multiplies δc for position
    pub d: HDRComplex,      // δz contribution to δρ
    pub e: HDRComplex,      // δc contribution to δρ
    pub l: u32,
    pub r_sq: HDRFloat,
}
```

**BlaEntryF64 struct:**
```rust
pub struct BlaEntryF64 {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub d: (f64, f64),
    pub e: (f64, f64),
    pub l: u32,
    pub r_sq: f64,
}
```

**from_orbit_point:** Add `der_re`, `der_im` parameters; compute D = 2·Der, E = 0.

**merge:** Add D and E merge logic using formulas above.

**try_from_hdr:** Convert d and e to f64, check for overflow.

### fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs

**BLA application block:**
```rust
if let Some(bla) = bla_entry {
    // Position: δz_new = A·δz + B·δc
    let new_dz = bla.a.mul(&dz).add(&bla.b.mul(&delta_c));

    // Derivative: δρ_new = A·δρ + D·δz + E·δc (C = A)
    let new_drho = bla.a.mul(&drho)
        .add(&bla.d.mul(&dz))
        .add(&bla.e.mul(&delta_c));

    dz = new_dz;
    drho = new_drho;
    // ...
}
```

### fractalwonder-compute/src/perturbation/pixel_f64_bla.rs

Same pattern as HDR path with f64 arithmetic. Remove incorrect comment about derivatives.

### fractalwonder-gpu/src/bla_upload.rs

**GpuBlaEntry:** Add d and e fields (12 more f32s). Size increases from 64 to 112 bytes.

### fractalwonder-gpu/src/buffers.rs

Update buffer size from 16 to 28 f32s per entry.

### fractalwonder-gpu/src/shaders/progressive_iteration.wgsl

**BlaEntry struct:** Add d and e fields.

**bla_load:** Read additional 12 f32s for d and e.

**BLA application:**
```wgsl
if bla.valid {
    // Position
    let new_dz = hdr_complex_add(
        hdr_complex_mul(bla.entry.a, dz),
        hdr_complex_mul(bla.entry.b, dc)
    );

    // Derivative (C = A)
    let new_drho = hdr_complex_add(
        hdr_complex_add(
            hdr_complex_mul(bla.entry.a, drho),
            hdr_complex_mul(bla.entry.d, dz)
        ),
        hdr_complex_mul(bla.entry.e, dc)
    );

    dz = new_dz;
    drho = new_drho;
    // ...
}
```

## Testing

Update existing tests for new `from_orbit_point` signature.

Add tests verifying:
- D = 2·Der_m at single step
- E = 0 at single step
- Correct D, E values after merge
- GpuBlaEntry size is 112 bytes

## Memory Impact

- BlaEntry: +2 HDRComplex (~50% increase)
- GpuBlaEntry: 64 → 112 bytes (75% increase)
- BLA table with 1M entries: ~64MB → ~112MB GPU memory

Acceptable tradeoff for correct 3D lighting at full BLA speed.
