# BLA Derivative Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend BLA to track derivatives during iteration skips, fixing 3D lighting artifacts.

**Architecture:** Add D and E coefficients to BLA entries. D captures how δz affects δρ, E captures accumulated δc contribution. Apply formula δρ_new = A·δρ + D·δz + E·δc during BLA skips. C = A mathematically, so not stored.

**Tech Stack:** Rust, WGSL shaders, HDRFloat/HDRComplex for extended precision

**Reference:** See `docs/plans/2026-01-03-bla-derivative-tracking-design.md` for mathematical derivation.

---

## Task 1: Extend BlaEntry with D and E coefficients

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:14-24`

**Step 1: Update BlaEntry struct**

Add d and e fields after b:

```rust
/// Single BLA entry: skips `l` iterations.
///
/// Position formula: δz_new = A·δz + B·δc
/// Derivative formula: δρ_new = C·δρ + D·δz + E·δc
///
/// Note: C = A mathematically (both derive from 2·Z_m with identical merge formulas),
/// so C is not stored separately.
#[derive(Clone, Debug)]
pub struct BlaEntry {
    /// Complex coefficient A (multiplies δz for position, δρ for derivative)
    pub a: HDRComplex,
    /// Complex coefficient B (multiplies δc for position)
    pub b: HDRComplex,
    /// Complex coefficient D (δz contribution to δρ)
    pub d: HDRComplex,
    /// Complex coefficient E (δc contribution to δρ)
    pub e: HDRComplex,
    /// Number of iterations to skip
    pub l: u32,
    /// Validity radius squared
    pub r_sq: HDRFloat,
}
```

**Step 2: Run cargo check to see compilation errors**

Run: `cargo check --workspace 2>&1 | head -50`
Expected: Errors about missing fields in struct initialization

**Step 3: Commit struct change**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "refactor(bla): add d and e coefficient fields to BlaEntry"
```

---

## Task 2: Extend BlaEntryF64 with D and E

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:27-38`

**Step 1: Update BlaEntryF64 struct**

```rust
/// BLA entry with f64 coefficients for fast-path rendering.
/// Created from HDR entry when coefficients fit in f64 range.
///
/// Note: C = A mathematically, so not stored separately.
#[derive(Clone, Debug)]
pub struct BlaEntryF64 {
    /// Complex coefficient A as (re, im)
    pub a: (f64, f64),
    /// Complex coefficient B as (re, im)
    pub b: (f64, f64),
    /// Complex coefficient D as (re, im) - δz contribution to δρ
    pub d: (f64, f64),
    /// Complex coefficient E as (re, im) - δc contribution to δρ
    pub e: (f64, f64),
    /// Number of iterations to skip
    pub l: u32,
    /// Validity radius squared
    pub r_sq: f64,
}
```

**Step 2: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "refactor(bla): add d and e coefficient fields to BlaEntryF64"
```

---

## Task 3: Update from_orbit_point to compute D and E

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:69-88`

**Step 1: Update function signature and implementation**

```rust
/// Create a single-iteration BLA from a reference orbit point.
///
/// Single-step coefficients:
///   A = 2·Z_m (multiplies δz for position, δρ for derivative)
///   B = 1 (multiplies δc for position)
///   D = 2·Der_m (δz contribution to δρ)
///   E = 0 (δc contribution to δρ, zero at single step)
pub fn from_orbit_point(z_re: f64, z_im: f64, der_re: f64, der_im: f64) -> Self {
    let epsilon = 2.0_f64.powi(-53);
    let z_mag = (z_re * z_re + z_im * z_im).sqrt();
    let r = epsilon * z_mag;

    Self {
        a: HDRComplex {
            re: HDRFloat::from_f64(2.0 * z_re),
            im: HDRFloat::from_f64(2.0 * z_im),
        },
        b: HDRComplex {
            re: HDRFloat::from_f64(1.0),
            im: HDRFloat::ZERO,
        },
        d: HDRComplex {
            re: HDRFloat::from_f64(2.0 * der_re),
            im: HDRFloat::from_f64(2.0 * der_im),
        },
        e: HDRComplex::ZERO,
        l: 1,
        r_sq: HDRFloat::from_f64(r * r),
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check --workspace 2>&1 | head -50`
Expected: Errors about wrong number of arguments to from_orbit_point

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): compute D and E coefficients in from_orbit_point"
```

---

## Task 4: Update merge to compute D and E

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:90-133`

**Step 1: Add D and E merge logic**

In the `merge` function, after computing `b`, add:

```rust
// Derivative coefficients
// D_merged = C_y·D_x + D_y·A_x (note: C_y = A_y)
let d = y.a.mul(&x.d).add(&y.d.mul(&x.a));

// E_merged = C_y·E_x + D_y·B_x + E_y (note: C_y = A_y)
let e = y.a.mul(&x.e).add(&y.d.mul(&x.b)).add(&y.e);
```

And update the returned BlaEntry to include `d` and `e`:

```rust
BlaEntry {
    a,
    b,
    d,
    e,
    l: x.l + y.l,
    r_sq: r.square(),
}
```

**Step 2: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): add D and E merge formulas"
```

---

## Task 5: Update try_from_hdr to convert D and E

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:40-67`

**Step 1: Add D and E conversion**

```rust
impl BlaEntryF64 {
    /// Try to convert from HDR entry. Returns None if any coefficient overflows f64.
    pub fn try_from_hdr(entry: &BlaEntry) -> Option<Self> {
        let a_re = entry.a.re.to_f64();
        let a_im = entry.a.im.to_f64();
        let b_re = entry.b.re.to_f64();
        let b_im = entry.b.im.to_f64();
        let d_re = entry.d.re.to_f64();
        let d_im = entry.d.im.to_f64();
        let e_re = entry.e.re.to_f64();
        let e_im = entry.e.im.to_f64();
        let r_sq = entry.r_sq.to_f64();

        // Check for overflow (inf) or underflow to zero when non-zero
        if !a_re.is_finite() || !a_im.is_finite() {
            return None;
        }
        if !b_re.is_finite() || !b_im.is_finite() {
            return None;
        }
        if !d_re.is_finite() || !d_im.is_finite() {
            return None;
        }
        if !e_re.is_finite() || !e_im.is_finite() {
            return None;
        }
        if !r_sq.is_finite() {
            return None;
        }

        Some(Self {
            a: (a_re, a_im),
            b: (b_re, b_im),
            d: (d_re, d_im),
            e: (e_re, e_im),
            l: entry.l,
            r_sq,
        })
    }
}
```

**Step 2: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): convert D and E in try_from_hdr"
```

---

## Task 6: Update BlaTable::compute to pass derivatives

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs:167-171`

**Step 1: Update level 0 BLA creation loop**

Change the loop that creates single-step BLAs:

```rust
// Level 0: single-iteration BLAs
level_offsets.push(0);
for i in 0..m {
    let (z_re, z_im) = orbit.orbit[i];
    let (der_re, der_im) = orbit.derivative[i];
    entries.push(BlaEntry::from_orbit_point(z_re, z_im, der_re, der_im));
}
```

**Step 2: Run cargo check**

Run: `cargo check --workspace 2>&1 | head -30`
Expected: Should compile (or show test errors)

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): pass derivatives to from_orbit_point in table construction"
```

---

## Task 7: Fix bla.rs unit tests

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs` (tests module at bottom)

**Step 1: Update bla_entry_from_orbit_point test**

```rust
#[test]
fn bla_entry_from_orbit_point() {
    // Z = (1.0, 0.5), Der = (0.5, 0.25), ε = 2^-53
    // A = 2Z = (2.0, 1.0)
    // B = 1
    // D = 2·Der = (1.0, 0.5)
    // E = 0
    let entry = BlaEntry::from_orbit_point(1.0, 0.5, 0.5, 0.25);

    assert!((entry.a.re.to_f64() - 2.0).abs() < 1e-14);
    assert!((entry.a.im.to_f64() - 1.0).abs() < 1e-14);
    assert!((entry.b.re.to_f64() - 1.0).abs() < 1e-14);
    assert!((entry.b.im.to_f64() - 0.0).abs() < 1e-14);
    assert!((entry.d.re.to_f64() - 1.0).abs() < 1e-14);
    assert!((entry.d.im.to_f64() - 0.5).abs() < 1e-14);
    assert!(entry.e.re.is_zero());
    assert!(entry.e.im.is_zero());
    assert_eq!(entry.l, 1);

    let z_mag = (1.0_f64 * 1.0 + 0.5 * 0.5).sqrt();
    let epsilon = 2.0_f64.powi(-53);
    let expected_r_sq = (epsilon * z_mag).powi(2);
    assert!((entry.r_sq.to_f64() - expected_r_sq).abs() < 1e-40);
}
```

**Step 2: Update bla_entry_merge_two_single_iterations test**

```rust
#[test]
fn bla_entry_merge_two_single_iterations() {
    // Two single-iteration BLAs with derivatives
    // Step 0: Z=1, Der=0 -> A=(2,0), D=(0,0)
    // Step 1: Z=0.5, Der=1 -> A=(1,0), D=(2,0)
    let x = BlaEntry::from_orbit_point(1.0, 0.0, 0.0, 0.0);
    let y = BlaEntry::from_orbit_point(0.5, 0.0, 1.0, 0.0);

    let dc_max = HDRFloat::from_f64(0.001);
    let merged = BlaEntry::merge(&x, &y, &dc_max);

    assert_eq!(merged.l, 2);

    // A_merged = A_y * A_x = (1,0) * (2,0) = (2,0)
    assert!((merged.a.re.to_f64() - 2.0).abs() < 1e-14);
    assert!((merged.a.im.to_f64() - 0.0).abs() < 1e-14);

    // B_merged = A_y * B_x + B_y = (1,0)*(1,0) + (1,0) = (2,0)
    assert!((merged.b.re.to_f64() - 2.0).abs() < 1e-14);
    assert!((merged.b.im.to_f64() - 0.0).abs() < 1e-14);

    // D_merged = A_y * D_x + D_y * A_x = (1,0)*(0,0) + (2,0)*(2,0) = (4,0)
    assert!((merged.d.re.to_f64() - 4.0).abs() < 1e-14);
    assert!((merged.d.im.to_f64() - 0.0).abs() < 1e-14);

    // E_merged = A_y * E_x + D_y * B_x + E_y = 0 + (2,0)*(1,0) + 0 = (2,0)
    assert!((merged.e.re.to_f64() - 2.0).abs() < 1e-14);
    assert!((merged.e.im.to_f64() - 0.0).abs() < 1e-14);
}
```

**Step 3: Update bla_entry_f64_from_hdr_entry test**

```rust
#[test]
fn bla_entry_f64_from_hdr_entry() {
    let entry = BlaEntry::from_orbit_point(1.0, 0.5, 0.5, 0.25);
    let f64_entry = BlaEntryF64::try_from_hdr(&entry);

    assert!(f64_entry.is_some());
    let f64_entry = f64_entry.unwrap();

    assert!((f64_entry.a.0 - 2.0).abs() < 1e-14);
    assert!((f64_entry.a.1 - 1.0).abs() < 1e-14);
    assert!((f64_entry.b.0 - 1.0).abs() < 1e-14);
    assert!((f64_entry.b.1 - 0.0).abs() < 1e-14);
    assert!((f64_entry.d.0 - 1.0).abs() < 1e-14);
    assert!((f64_entry.d.1 - 0.5).abs() < 1e-14);
    assert!((f64_entry.e.0 - 0.0).abs() < 1e-14);
    assert!((f64_entry.e.1 - 0.0).abs() < 1e-14);
    assert_eq!(f64_entry.l, 1);
}
```

**Step 4: Run tests**

Run: `cargo test --package fractalwonder-compute bla:: -- --nocapture`
Expected: All bla tests pass

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "test(bla): update tests for D and E coefficients"
```

---

## Task 8: Apply derivatives in HDR BLA path

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs:120-128`

**Step 1: Update BLA application block**

Replace the existing BLA block:

```rust
if let Some(bla) = bla_entry {
    // Apply BLA to position: δz_new = A·δz + B·δc
    let a_dz = bla.a.mul(&dz);
    let b_dc = bla.b.mul(&delta_c);
    let new_dz = a_dz.add(&b_dc);

    // Apply BLA to derivative: δρ_new = A·δρ + D·δz + E·δc
    // (C = A mathematically, so we use bla.a for the δρ coefficient)
    let a_drho = bla.a.mul(&drho);
    let d_dz = bla.d.mul(&dz);
    let e_dc = bla.e.mul(&delta_c);
    let new_drho = a_drho.add(&d_dz).add(&e_dc);

    dz = new_dz;
    drho = new_drho;

    bla_iters += bla.l;
    m += bla.l as usize;
    n += bla.l;
}
```

**Step 2: Run tests**

Run: `cargo test --package fractalwonder-compute perturbation:: -- --nocapture`
Expected: Tests pass

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs
git commit -m "feat(perturbation): apply derivative coefficients in HDR BLA path"
```

---

## Task 9: Apply derivatives in f64 BLA path

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/pixel_f64_bla.rs:117-128`

**Step 1: Update BLA application block and remove incorrect comment**

Replace the existing BLA block:

```rust
// 4. Try BLA acceleration (with f64 coefficients)
if let Some(bla) = bla_table.find_valid_f64(m, dz_mag_sq, dc_max) {
    // Apply BLA to position: dz_new = A*dz + B*dc
    let a_dz = complex_mul_f64(bla.a, dz);
    let b_dc = complex_mul_f64(bla.b, delta_c);
    let new_dz = (a_dz.0 + b_dc.0, a_dz.1 + b_dc.1);

    // Apply BLA to derivative: drho_new = A*drho + D*dz + E*dc
    // (C = A mathematically, so we use bla.a for the drho coefficient)
    let a_drho = complex_mul_f64(bla.a, drho);
    let d_dz = complex_mul_f64(bla.d, dz);
    let e_dc = complex_mul_f64(bla.e, delta_c);
    let new_drho = (
        a_drho.0 + d_dz.0 + e_dc.0,
        a_drho.1 + d_dz.1 + e_dc.1,
    );

    dz = new_dz;
    drho = new_drho;

    bla_iters += bla.l;
    m += bla.l as usize;
    n += bla.l;
}
```

**Step 2: Run tests**

Run: `cargo test --package fractalwonder-compute -- --nocapture`
Expected: All tests pass

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation/pixel_f64_bla.rs
git commit -m "feat(perturbation): apply derivative coefficients in f64 BLA path"
```

---

## Task 10: Extend GpuBlaEntry with D and E

**Files:**
- Modify: `fractalwonder-gpu/src/bla_upload.rs`

**Step 1: Update GpuBlaEntry struct**

```rust
/// GPU-serializable BLA entry (112 bytes, 28 f32-equivalent values).
/// Layout: A (6), B (6), D (6), E (6), r_sq (3), l (1) = 28 values
///
/// Note: C = A mathematically for derivative computation, so not stored.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuBlaEntry {
    // Coefficient A (HDRComplex) - multiplies δz and δρ (C = A)
    pub a_re_head: f32,
    pub a_re_tail: f32,
    pub a_re_exp: i32,
    pub a_im_head: f32,
    pub a_im_tail: f32,
    pub a_im_exp: i32,

    // Coefficient B (HDRComplex) - multiplies δc for position
    pub b_re_head: f32,
    pub b_re_tail: f32,
    pub b_re_exp: i32,
    pub b_im_head: f32,
    pub b_im_tail: f32,
    pub b_im_exp: i32,

    // Coefficient D (HDRComplex) - δz contribution to δρ
    pub d_re_head: f32,
    pub d_re_tail: f32,
    pub d_re_exp: i32,
    pub d_im_head: f32,
    pub d_im_tail: f32,
    pub d_im_exp: i32,

    // Coefficient E (HDRComplex) - δc contribution to δρ
    pub e_re_head: f32,
    pub e_re_tail: f32,
    pub e_re_exp: i32,
    pub e_im_head: f32,
    pub e_im_tail: f32,
    pub e_im_exp: i32,

    // Validity radius squared (HDRFloat)
    pub r_sq_head: f32,
    pub r_sq_tail: f32,
    pub r_sq_exp: i32,

    // Iterations to skip
    pub l: u32,
}
```

**Step 2: Update from_bla_entry**

```rust
impl GpuBlaEntry {
    /// Convert from CPU BlaEntry to GPU format.
    pub fn from_bla_entry(entry: &BlaEntry) -> Self {
        Self {
            a_re_head: entry.a.re.head,
            a_re_tail: entry.a.re.tail,
            a_re_exp: entry.a.re.exp,
            a_im_head: entry.a.im.head,
            a_im_tail: entry.a.im.tail,
            a_im_exp: entry.a.im.exp,
            b_re_head: entry.b.re.head,
            b_re_tail: entry.b.re.tail,
            b_re_exp: entry.b.re.exp,
            b_im_head: entry.b.im.head,
            b_im_tail: entry.b.im.tail,
            b_im_exp: entry.b.im.exp,
            d_re_head: entry.d.re.head,
            d_re_tail: entry.d.re.tail,
            d_re_exp: entry.d.re.exp,
            d_im_head: entry.d.im.head,
            d_im_tail: entry.d.im.tail,
            d_im_exp: entry.d.im.exp,
            e_re_head: entry.e.re.head,
            e_re_tail: entry.e.re.tail,
            e_re_exp: entry.e.re.exp,
            e_im_head: entry.e.im.head,
            e_im_tail: entry.e.im.tail,
            e_im_exp: entry.e.im.exp,
            r_sq_head: entry.r_sq.head,
            r_sq_tail: entry.r_sq.tail,
            r_sq_exp: entry.r_sq.exp,
            l: entry.l,
        }
    }
}
```

**Step 3: Update tests**

```rust
#[test]
fn gpu_bla_entry_size_is_112_bytes() {
    assert_eq!(std::mem::size_of::<GpuBlaEntry>(), 112);
}

#[test]
fn gpu_bla_entry_from_bla_entry_preserves_values() {
    let entry = BlaEntry::from_orbit_point(1.5, 0.5, 0.75, 0.25);
    let gpu_entry = GpuBlaEntry::from_bla_entry(&entry);

    // Coefficient A
    assert_eq!(gpu_entry.a_re_head, entry.a.re.head);
    assert_eq!(gpu_entry.a_re_tail, entry.a.re.tail);
    assert_eq!(gpu_entry.a_re_exp, entry.a.re.exp);
    assert_eq!(gpu_entry.a_im_head, entry.a.im.head);
    assert_eq!(gpu_entry.a_im_tail, entry.a.im.tail);
    assert_eq!(gpu_entry.a_im_exp, entry.a.im.exp);

    // Coefficient B
    assert_eq!(gpu_entry.b_re_head, entry.b.re.head);
    assert_eq!(gpu_entry.b_re_tail, entry.b.re.tail);
    assert_eq!(gpu_entry.b_re_exp, entry.b.re.exp);
    assert_eq!(gpu_entry.b_im_head, entry.b.im.head);
    assert_eq!(gpu_entry.b_im_tail, entry.b.im.tail);
    assert_eq!(gpu_entry.b_im_exp, entry.b.im.exp);

    // Coefficient D
    assert_eq!(gpu_entry.d_re_head, entry.d.re.head);
    assert_eq!(gpu_entry.d_re_tail, entry.d.re.tail);
    assert_eq!(gpu_entry.d_re_exp, entry.d.re.exp);
    assert_eq!(gpu_entry.d_im_head, entry.d.im.head);
    assert_eq!(gpu_entry.d_im_tail, entry.d.im.tail);
    assert_eq!(gpu_entry.d_im_exp, entry.d.im.exp);

    // Coefficient E
    assert_eq!(gpu_entry.e_re_head, entry.e.re.head);
    assert_eq!(gpu_entry.e_re_tail, entry.e.re.tail);
    assert_eq!(gpu_entry.e_re_exp, entry.e.re.exp);
    assert_eq!(gpu_entry.e_im_head, entry.e.im.head);
    assert_eq!(gpu_entry.e_im_tail, entry.e.im.tail);
    assert_eq!(gpu_entry.e_im_exp, entry.e.im.exp);

    // Validity radius squared
    assert_eq!(gpu_entry.r_sq_head, entry.r_sq.head);
    assert_eq!(gpu_entry.r_sq_tail, entry.r_sq.tail);
    assert_eq!(gpu_entry.r_sq_exp, entry.r_sq.exp);

    // Iterations to skip
    assert_eq!(gpu_entry.l, entry.l);
}
```

**Step 4: Run tests**

Run: `cargo test --package fractalwonder-gpu bla_upload -- --nocapture`
Expected: Tests pass

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/bla_upload.rs
git commit -m "feat(gpu): extend GpuBlaEntry with D and E coefficients"
```

---

## Task 11: Update GPU buffer size

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:302-305`

**Step 1: Update buffer size comment and calculation**

```rust
// BLA data: 28 f32s per entry (112 bytes)
let bla_data = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("progressive_bla_data"),
    size: (bla_entry_count as usize * 28 * std::mem::size_of::<f32>()) as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): increase BLA buffer size to 28 f32s per entry"
```

---

## Task 12: Update GPU shader BlaEntry struct

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:340-350`

**Step 1: Update struct definition**

```wgsl
// BLA entry: 28 f32s (112 bytes)
// Note: C = A mathematically for derivative computation, so not stored.
struct BlaEntry {
    a: HDRComplex,      // Multiplies δz and δρ (C = A)
    b: HDRComplex,      // Multiplies δc for position
    d: HDRComplex,      // δz contribution to δρ
    e: HDRComplex,      // δc contribution to δρ
    r_sq: HDRFloat,     // Validity radius squared
    l: u32,             // Iterations to skip
}
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): extend shader BlaEntry struct with D and E"
```

---

## Task 13: Update GPU shader bla_load function

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:352-370`

**Step 1: Update bla_load to read D and E**

Find the `bla_load` function and update it to read all 28 values:

```wgsl
fn bla_load(idx: u32) -> BlaEntry {
    let base = idx * 28u;
    return BlaEntry(
        // A
        HDRComplex(
            HDRFloat(bla_data[base], bla_data[base + 1u], i32(bitcast<u32>(bla_data[base + 2u]))),
            HDRFloat(bla_data[base + 3u], bla_data[base + 4u], i32(bitcast<u32>(bla_data[base + 5u])))
        ),
        // B
        HDRComplex(
            HDRFloat(bla_data[base + 6u], bla_data[base + 7u], i32(bitcast<u32>(bla_data[base + 8u]))),
            HDRFloat(bla_data[base + 9u], bla_data[base + 10u], i32(bitcast<u32>(bla_data[base + 11u])))
        ),
        // D
        HDRComplex(
            HDRFloat(bla_data[base + 12u], bla_data[base + 13u], i32(bitcast<u32>(bla_data[base + 14u]))),
            HDRFloat(bla_data[base + 15u], bla_data[base + 16u], i32(bitcast<u32>(bla_data[base + 17u])))
        ),
        // E
        HDRComplex(
            HDRFloat(bla_data[base + 18u], bla_data[base + 19u], i32(bitcast<u32>(bla_data[base + 20u]))),
            HDRFloat(bla_data[base + 21u], bla_data[base + 22u], i32(bitcast<u32>(bla_data[base + 23u])))
        ),
        // r_sq
        HDRFloat(bla_data[base + 24u], bla_data[base + 25u], i32(bitcast<u32>(bla_data[base + 26u]))),
        // l
        bitcast<u32>(bla_data[base + 27u])
    );
}
```

**Step 2: Update the empty_entry in bla_find_valid**

Find where `empty_entry` is defined and update it:

```wgsl
let empty_entry = BlaEntry(HDR_COMPLEX_ZERO, HDR_COMPLEX_ZERO, HDR_COMPLEX_ZERO, HDR_COMPLEX_ZERO, HDR_ZERO, 0u);
```

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): update bla_load to read D and E coefficients"
```

---

## Task 14: Apply derivatives in GPU shader BLA block

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:625-635`

**Step 1: Update BLA application block**

```wgsl
if bla.valid {
    // Apply BLA to position: δz_new = A·δz + B·δc
    let a_dz = hdr_complex_mul(bla.entry.a, dz);
    let b_dc = hdr_complex_mul(bla.entry.b, dc);
    let new_dz = hdr_complex_add(a_dz, b_dc);

    // Apply BLA to derivative: δρ_new = A·δρ + D·δz + E·δc
    // (C = A mathematically, so we use bla.entry.a for the δρ coefficient)
    let a_drho = hdr_complex_mul(bla.entry.a, drho);
    let d_dz = hdr_complex_mul(bla.entry.d, dz);
    let e_dc = hdr_complex_mul(bla.entry.e, dc);
    let new_drho = hdr_complex_add(hdr_complex_add(a_drho, d_dz), e_dc);

    dz = new_dz;
    drho = new_drho;

    // Skip iterations
    m = m + bla.entry.l;
    n = n + bla.entry.l;
    continue;
}
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): apply derivative coefficients in shader BLA block"
```

---

## Task 15: Run full test suite and verify

**Step 1: Run all Rust tests**

Run: `cargo test --workspace -- --nocapture`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run format check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues (or run `cargo fmt --all` to fix)

**Step 4: Build WASM**

Run: `cargo check --target wasm32-unknown-unknown --package fractalwonder-ui`
Expected: Compiles successfully

**Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: fix any remaining issues from review"
```

---

## Task 16: Manual verification in browser

**Step 1: Start dev server**

Ensure `trunk serve` is running on localhost:8080

**Step 2: Test 3D lighting at deep zoom**

1. Navigate to a deep zoom location (10^50 or deeper)
2. Enable 3D lighting
3. Verify no semi-circular artifacts around center
4. Verify smooth lighting transitions across the image

**Step 3: Create final commit if all looks good**

```bash
git add -A
git commit -m "feat: complete BLA derivative tracking implementation

Fixes 3D lighting artifacts at deep zoom by correctly tracking derivatives
during BLA iteration skips.

- Add D and E coefficients to BlaEntry and BlaEntryF64
- Compute D = 2·Der_m, E = 0 at single step
- Implement merge formulas for D and E
- Apply derivative formula in CPU HDR, CPU f64, and GPU paths
- Update GPU buffer layout and shader

Mathematical basis: C = A (not stored separately)
δρ_new = A·δρ + D·δz + E·δc"
```
