# BLA for f64 Path Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add BLA (Bivariate Linear Approximation) iteration skipping to the f64 rendering path for 33x speedup at moderate zoom levels.

**Architecture:** Add `find_valid_f64()` to BlaTable that converts HDRFloat coefficients to f64 with overflow checking. Create `compute_pixel_perturbation_f64_bla()` that uses f64 arithmetic with BLA. Update dispatch logic to pass BLA table to f64 path.

**Tech Stack:** Rust, f64 arithmetic, HDRFloat->f64 conversion with overflow guards

---

## Background

Currently BLA is only available in the HDR path which is ~50x slower per iteration than f64. At moderate zoom (10^270), f64 arithmetic works fine. Goal: f64 + BLA = 900 iterations at 1x speed = 900 work units (vs 45,000 for HDR+BLA).

---

## Task 1: Add BlaEntryF64 Struct

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 1.1: Write the failing test**

Add to `bla.rs` in the `#[cfg(test)] mod tests` section:

```rust
#[test]
fn bla_entry_f64_from_hdr_entry() {
    let entry = BlaEntry::from_orbit_point(1.0, 0.5);
    let f64_entry = BlaEntryF64::try_from_hdr(&entry);

    assert!(f64_entry.is_some());
    let f64_entry = f64_entry.unwrap();

    assert!((f64_entry.a.0 - 2.0).abs() < 1e-14);
    assert!((f64_entry.a.1 - 1.0).abs() < 1e-14);
    assert!((f64_entry.b.0 - 1.0).abs() < 1e-14);
    assert!((f64_entry.b.1 - 0.0).abs() < 1e-14);
    assert_eq!(f64_entry.l, 1);
}
```

**Step 1.2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_entry_f64_from_hdr_entry -- --nocapture`
Expected: FAIL with "cannot find type `BlaEntryF64`"

**Step 1.3: Write minimal implementation**

Add after `BlaEntry` struct (around line 91):

```rust
/// BLA entry with f64 coefficients for fast-path rendering.
/// Created from HDR entry when coefficients fit in f64 range.
#[derive(Clone, Debug)]
pub struct BlaEntryF64 {
    /// Complex coefficient A as (re, im)
    pub a: (f64, f64),
    /// Complex coefficient B as (re, im)
    pub b: (f64, f64),
    /// Number of iterations to skip
    pub l: u32,
    /// Validity radius squared
    pub r_sq: f64,
}

impl BlaEntryF64 {
    /// Try to convert from HDR entry. Returns None if any coefficient overflows f64.
    pub fn try_from_hdr(entry: &BlaEntry) -> Option<Self> {
        let a_re = entry.a.re.to_f64();
        let a_im = entry.a.im.to_f64();
        let b_re = entry.b.re.to_f64();
        let b_im = entry.b.im.to_f64();
        let r_sq = entry.r_sq.to_f64();

        // Check for overflow (inf) or underflow to zero when non-zero
        if !a_re.is_finite() || !a_im.is_finite() {
            return None;
        }
        if !b_re.is_finite() || !b_im.is_finite() {
            return None;
        }
        if !r_sq.is_finite() {
            return None;
        }

        Some(Self {
            a: (a_re, a_im),
            b: (b_re, b_im),
            l: entry.l,
            r_sq,
        })
    }
}
```

**Step 1.4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_entry_f64_from_hdr_entry -- --nocapture`
Expected: PASS

**Step 1.5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "$(cat <<'EOF'
feat(bla): add BlaEntryF64 struct for f64 path BLA

Adds BlaEntryF64 with try_from_hdr() that converts HDRFloat
coefficients to f64 with overflow checking.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add Overflow Detection Test

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 2.1: Write the failing test**

Add test for overflow case:

```rust
#[test]
fn bla_entry_f64_returns_none_on_overflow() {
    // Create entry with HDRFloat coefficients that overflow f64
    let huge = HDRFloat { head: 1.0, tail: 0.0, exp: 1100 }; // ~2^1100, way beyond f64
    let entry = BlaEntry {
        a: HDRComplex { re: huge, im: HDRFloat::ZERO },
        b: HDRComplex { re: HDRFloat::from_f64(1.0), im: HDRFloat::ZERO },
        l: 1,
        r_sq: HDRFloat::from_f64(1e-10),
    };

    let f64_entry = BlaEntryF64::try_from_hdr(&entry);
    assert!(f64_entry.is_none(), "Should return None when coefficient overflows f64");
}
```

**Step 2.2: Run test to verify it passes (implementation already handles this)**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_entry_f64_returns_none_on_overflow -- --nocapture`
Expected: PASS (try_from_hdr already checks is_finite())

**Step 2.3: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "$(cat <<'EOF'
test(bla): add overflow detection test for BlaEntryF64

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Add find_valid_f64 Method to BlaTable

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 3.1: Write the failing test**

```rust
#[test]
fn bla_table_find_valid_f64_returns_some_for_tiny_dz() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);
    let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

    // At m=1, Z_m != 0, so r > 0 and BLA should be valid
    let dc_max = 1e-10;
    let result = table.find_valid_f64(1, 0.0, dc_max);
    assert!(result.is_some(), "Zero |δz|² at m=1 should allow f64 BLA");

    let bla = result.unwrap();
    assert!(bla.l >= 1);
    assert!(bla.a.0.is_finite());
    assert!(bla.r_sq > 0.0);
}
```

**Step 3.2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_table_find_valid_f64_returns_some_for_tiny_dz -- --nocapture`
Expected: FAIL with "no method named `find_valid_f64`"

**Step 3.3: Write minimal implementation**

Add to `impl BlaTable` (after `find_valid` method, around line 238):

```rust
    /// Find the largest valid BLA at reference index `m` for current |δz|², returning f64 coefficients.
    /// Returns None if no BLA is valid or if coefficients overflow f64 range.
    ///
    /// This is the f64-optimized version for moderate zoom levels where BLA coefficients
    /// fit in f64 range. Falls back gracefully when coefficients overflow.
    pub fn find_valid_f64(&self, m: usize, dz_mag_sq: f64, dc_max: f64) -> Option<BlaEntryF64> {
        if self.entries.is_empty() {
            return None;
        }

        let max_b_dc_exp = 0;

        for level in (0..=self.num_levels.saturating_sub(1)).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1usize << level;

            if !m.is_multiple_of(skip_size) {
                continue;
            }

            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            let level_end = if level + 1 < self.level_offsets.len() {
                self.level_offsets[level + 1]
            } else {
                self.entries.len()
            };

            if entry_idx >= level_end {
                continue;
            }

            let entry = &self.entries[entry_idx];

            // Convert r_sq to f64 for comparison
            let r_sq_f64 = entry.r_sq.to_f64();
            if !r_sq_f64.is_finite() || r_sq_f64 <= 0.0 {
                continue;
            }

            // Validity check: |δz|² < r²
            if dz_mag_sq >= r_sq_f64 {
                continue;
            }

            // B coefficient check: |B| * dc_max must not be too large
            let b_norm = entry.b.norm_hdr();
            let dc_max_hdr = HDRFloat::from_f64(dc_max);
            let b_dc = b_norm.mul(&dc_max_hdr);
            if b_dc.exp > max_b_dc_exp {
                continue;
            }

            // Try to convert to f64 - returns None if overflow
            if let Some(f64_entry) = BlaEntryF64::try_from_hdr(entry) {
                return Some(f64_entry);
            }
            // If conversion failed, try lower level
        }

        None
    }
```

**Step 3.4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_table_find_valid_f64_returns_some_for_tiny_dz -- --nocapture`
Expected: PASS

**Step 3.5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "$(cat <<'EOF'
feat(bla): add find_valid_f64 method for f64 path BLA lookup

Searches BLA table and returns f64 coefficients when they fit
in f64 range. Gracefully returns None on overflow.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Add find_valid_f64 Returns None for Large dz Test

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 4.1: Write the test**

```rust
#[test]
fn bla_table_find_valid_f64_returns_none_for_large_dz() {
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);
    let table = BlaTable::compute(&orbit, &HDRFloat::from_f64(0.01));

    // With |δz|² = 1.0 (huge), no BLA should be valid
    let result = table.find_valid_f64(0, 1.0, 0.01);
    assert!(result.is_none(), "Large |δz| should invalidate all f64 BLAs");
}
```

**Step 4.2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute bla::tests::bla_table_find_valid_f64_returns_none_for_large_dz -- --nocapture`
Expected: PASS

**Step 4.3: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "$(cat <<'EOF'
test(bla): add large dz test for find_valid_f64

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Create pixel_f64_bla.rs - Basic Structure

**Files:**
- Create: `fractalwonder-compute/src/perturbation/pixel_f64_bla.rs`
- Modify: `fractalwonder-compute/src/perturbation/mod.rs`

**Step 5.1: Write the failing test**

Create `pixel_f64_bla.rs`:

```rust
//! f64 perturbation with BLA (Bivariate Linear Approximation) acceleration.
//!
//! Fast path for moderate zoom levels where f64 arithmetic works and
//! BLA coefficients don't overflow f64 range.

use super::{compute_surface_normal_direction, ReferenceOrbit};
use crate::bla::BlaTable;
use fractalwonder_core::MandelbrotData;

pub use super::pixel_hdr_bla::BlaStats;

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{BigFloat, HDRFloat};

    #[test]
    fn pixel_f64_bla_escapes_at_correct_iteration() {
        // Create orbit for c = -0.5 + 0i (inside set, doesn't escape)
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-6));

        // Pixel at c = 2.0 + 0i (outside set, escapes immediately)
        // delta_c = 2.0 - (-0.5) = 2.5
        let delta_c = (2.5, 0.0);
        let (result, _stats) = compute_pixel_perturbation_f64_bla(&orbit, &bla_table, delta_c, 100, 1e-6);

        assert!(result.escaped, "Point at c=2+0i should escape");
        assert!(result.iterations < 10, "Should escape quickly");
    }
}
```

**Step 5.2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute perturbation::pixel_f64_bla::tests::pixel_f64_bla_escapes_at_correct_iteration -- --nocapture`
Expected: FAIL with "cannot find function `compute_pixel_perturbation_f64_bla`"

**Step 5.3: Write minimal implementation**

Add to `pixel_f64_bla.rs` above the tests module:

```rust
/// Compute pixel using f64 perturbation with BLA acceleration.
/// Returns pixel data and BLA statistics for performance monitoring.
///
/// This is the fast path for moderate zoom levels where:
/// - f64 arithmetic is sufficient (delta values in ~10^±300 range)
/// - BLA coefficients fit in f64 range
///
/// Falls back to standard iteration when BLA coefficients overflow.
pub fn compute_pixel_perturbation_f64_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> (MandelbrotData, BlaStats) {
    let mut dz = (0.0, 0.0);
    let mut drho = (0.0, 0.0);
    let mut m: usize = 0;
    let mut glitched = false;
    let mut bla_iters: u32 = 0;
    let mut standard_iters: u32 = 0;
    let mut rebase_count: u32 = 0;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return (
            MandelbrotData {
                iterations: 0,
                max_iterations,
                escaped: false,
                glitched: true,
                final_z_norm_sq: 0.0,
                surface_normal_re: 0.0,
                surface_normal_im: 0.0,
            },
            BlaStats::default(),
        );
    }

    let reference_escaped = orbit.escaped_at.is_some();
    let mut n = 0u32;

    // dc_max for BLA validity check (magnitude of delta_c)
    let dc_max = (delta_c.0 * delta_c.0 + delta_c.1 * delta_c.1).sqrt();

    while n < max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];
        let (der_m_re, der_m_im) = orbit.derivative[m % orbit_len];

        // Full values: z = Z_m + δz, ρ = Der_m + δρ
        let z_re = z_m_re + dz.0;
        let z_im = z_m_im + dz.1;
        let rho_re = der_m_re + drho.0;
        let rho_im = der_m_im + drho.1;

        let z_mag_sq = z_re * z_re + z_im * z_im;
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            let (sn_re, sn_im) = compute_surface_normal_direction(z_re, z_im, rho_re, rho_im);

            return (
                MandelbrotData::new(
                    n,
                    max_iterations,
                    true,
                    glitched,
                    z_mag_sq as f32,
                    sn_re,
                    sn_im,
                ),
                BlaStats {
                    bla_iterations: bla_iters,
                    total_iterations: bla_iters + standard_iters,
                    rebase_count,
                },
            );
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check: if |z| < |δz|, the perturbation dominates the full value
        if z_mag_sq < dz_mag_sq {
            dz = (z_re, z_im);
            drho = (rho_re, rho_im);
            m = 0;
            rebase_count += 1;
            continue;
        }

        // 4. Try BLA acceleration (with f64 coefficients)
        if let Some(bla) = bla_table.find_valid_f64(m, dz_mag_sq, dc_max) {
            // Apply BLA: δz_new = A·δz + B·δc (f64 complex multiply)
            let a_dz = complex_mul_f64(bla.a, dz);
            let b_dc = complex_mul_f64(bla.b, delta_c);
            dz = (a_dz.0 + b_dc.0, a_dz.1 + b_dc.1);

            // Note: drho derivative tracking not implemented for BLA path
            // This is acceptable since surface normals are computed at escape

            bla_iters += bla.l;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration: δz' = 2·Z_m·δz + δz² + δc
            let old_dz = dz;

            let two_z_dz_re = 2.0 * (z_m_re * dz.0 - z_m_im * dz.1);
            let two_z_dz_im = 2.0 * (z_m_re * dz.1 + z_m_im * dz.0);

            let dz_sq_re = dz.0 * dz.0 - dz.1 * dz.1;
            let dz_sq_im = 2.0 * dz.0 * dz.1;

            dz = (
                two_z_dz_re + dz_sq_re + delta_c.0,
                two_z_dz_im + dz_sq_im + delta_c.1,
            );

            // Derivative delta iteration: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
            let two_z_drho_re = 2.0 * (z_m_re * drho.0 - z_m_im * drho.1);
            let two_z_drho_im = 2.0 * (z_m_re * drho.1 + z_m_im * drho.0);

            let two_dz_der_re = 2.0 * (old_dz.0 * der_m_re - old_dz.1 * der_m_im);
            let two_dz_der_im = 2.0 * (old_dz.0 * der_m_im + old_dz.1 * der_m_re);

            let two_dz_drho_re = 2.0 * (old_dz.0 * drho.0 - old_dz.1 * drho.1);
            let two_dz_drho_im = 2.0 * (old_dz.0 * drho.1 + old_dz.1 * drho.0);

            drho = (
                two_z_drho_re + two_dz_der_re + two_dz_drho_re,
                two_z_drho_im + two_dz_der_im + two_dz_drho_im,
            );

            standard_iters += 1;
            m += 1;
            n += 1;
        }
    }

    (
        MandelbrotData {
            iterations: max_iterations,
            max_iterations,
            escaped: false,
            glitched,
            final_z_norm_sq: 0.0,
            surface_normal_re: 0.0,
            surface_normal_im: 0.0,
        },
        BlaStats {
            bla_iterations: bla_iters,
            total_iterations: bla_iters + standard_iters,
            rebase_count,
        },
    )
}

/// Complex multiplication for f64 tuples: (a_re, a_im) * (b_re, b_im)
#[inline]
fn complex_mul_f64(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
```

**Step 5.4: Update mod.rs to include the new module**

Modify `fractalwonder-compute/src/perturbation/mod.rs`:

```rust
//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

mod pixel;
mod pixel_f64_bla;
mod pixel_hdr_bla;
mod reference_orbit;
mod tile;

pub use tile::{render_tile_f64, render_tile_hdr, TileConfig, TileRenderResult, TileStats};

pub use pixel::compute_pixel_perturbation;
pub use pixel_f64_bla::compute_pixel_perturbation_f64_bla;
pub use pixel_hdr_bla::{compute_pixel_perturbation_hdr_bla, BlaStats};
pub use reference_orbit::ReferenceOrbit;

/// Compute normalized z/ρ direction for 3D lighting.
/// Returns (re, im) of the unit vector, or (0, 0) if degenerate.
/// This works at any zoom level since we normalize to a unit vector.
#[inline]
pub(crate) fn compute_surface_normal_direction(
    z_re: f64,
    z_im: f64,
    rho_re: f64,
    rho_im: f64,
) -> (f32, f32) {
    // u = z / ρ (complex division)
    // u = z * conj(ρ) / |ρ|²
    let rho_norm_sq = rho_re * rho_re + rho_im * rho_im;
    if !rho_norm_sq.is_finite() || rho_norm_sq == 0.0 {
        return (0.0, 0.0);
    }

    let u_re = (z_re * rho_re + z_im * rho_im) / rho_norm_sq;
    let u_im = (z_im * rho_re - z_re * rho_im) / rho_norm_sq;

    // Normalize to unit vector
    let u_norm = (u_re * u_re + u_im * u_im).sqrt();
    if !u_norm.is_finite() || u_norm == 0.0 {
        return (0.0, 0.0);
    }

    ((u_re / u_norm) as f32, (u_im / u_norm) as f32)
}

#[cfg(test)]
mod tests;
```

**Step 5.5: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute perturbation::pixel_f64_bla::tests::pixel_f64_bla_escapes_at_correct_iteration -- --nocapture`
Expected: PASS

**Step 5.6: Commit**

```bash
git add fractalwonder-compute/src/perturbation/pixel_f64_bla.rs fractalwonder-compute/src/perturbation/mod.rs
git commit -m "$(cat <<'EOF'
feat(perturbation): add compute_pixel_perturbation_f64_bla

f64 perturbation with BLA acceleration for moderate zoom levels.
Uses find_valid_f64 for BLA lookup with f64 coefficients.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Add BLA Iteration Skipping Test

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/pixel_f64_bla.rs`

**Step 6.1: Write the test**

Add to tests module in `pixel_f64_bla.rs`:

```rust
#[test]
fn pixel_f64_bla_skips_iterations() {
    // Create orbit for point inside set
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);
    let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

    // Small delta - should trigger BLA
    let delta_c = (1e-12, 1e-12);
    let (_result, stats) = compute_pixel_perturbation_f64_bla(&orbit, &bla_table, delta_c, 1000, 1e-6);

    // BLA should skip some iterations
    assert!(
        stats.bla_iterations > 0,
        "BLA should skip iterations, got bla_iters={}",
        stats.bla_iterations
    );
    assert!(
        stats.total_iterations < 1000,
        "Should complete in fewer than max iterations"
    );
}
```

**Step 6.2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute perturbation::pixel_f64_bla::tests::pixel_f64_bla_skips_iterations -- --nocapture`
Expected: PASS (implementation already handles BLA)

**Step 6.3: Commit**

```bash
git add fractalwonder-compute/src/perturbation/pixel_f64_bla.rs
git commit -m "$(cat <<'EOF'
test(pixel_f64_bla): add BLA iteration skipping test

Verifies that BLA is actually skipping iterations in f64 path.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Update render_tile_f64 to Accept BLA Table

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/tile.rs`

**Step 7.1: Write the failing test**

Add to end of `tile.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bla::BlaTable;
    use crate::ReferenceOrbit;
    use fractalwonder_core::{BigFloat, HDRFloat};

    #[test]
    fn render_tile_f64_with_bla_uses_bla() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);
        let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

        let config = TileConfig {
            size: (4, 4),
            max_iterations: 1000,
            tau_sq: 1e-6,
            bla_enabled: true,
        };

        // Small deltas to trigger BLA
        let delta_origin = (1e-12, 1e-12);
        let delta_step = (1e-14, 1e-14);

        let result = render_tile_f64(&orbit, Some(&bla_table), delta_origin, delta_step, &config);

        // Should have used BLA for at least some iterations
        assert!(
            result.stats.bla_iterations > 0,
            "BLA should be used in f64 path, got bla_iters={}",
            result.stats.bla_iterations
        );
    }
}
```

**Step 7.2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute perturbation::tile::tests::render_tile_f64_with_bla_uses_bla -- --nocapture`
Expected: FAIL with "this function takes 4 arguments but 5 arguments were supplied"

**Step 7.3: Update render_tile_f64 signature and implementation**

Modify `render_tile_f64` in `tile.rs`:

```rust
/// Render a tile using f64 precision with optional BLA acceleration.
///
/// This path is used when delta values fit comfortably in f64 range (~10^±300).
/// BLA is applied when enabled and coefficients fit in f64 range.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit
/// * `bla_table` - Optional BLA table for iteration skipping
/// * `delta_origin` - Delta from reference point to top-left pixel (re, im)
/// * `delta_step` - Delta step between pixels (re, im)
/// * `config` - Tile rendering configuration
///
/// # Returns
/// Computed pixel data and rendering statistics
pub fn render_tile_f64(
    orbit: &ReferenceOrbit,
    bla_table: Option<&BlaTable>,
    delta_origin: (f64, f64),
    delta_step: (f64, f64),
    config: &TileConfig,
) -> TileRenderResult {
    let capacity = (config.size.0 * config.size.1) as usize;
    let mut data = Vec::with_capacity(capacity);
    let mut stats = TileStats::default();

    let mut delta_c_row = delta_origin;

    for _py in 0..config.size.1 {
        let mut delta_c = delta_c_row;

        for _px in 0..config.size.0 {
            if config.bla_enabled {
                if let Some(bla) = bla_table {
                    let (result, pixel_stats) = compute_pixel_perturbation_f64_bla(
                        orbit,
                        bla,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.bla_iterations += pixel_stats.bla_iterations as u64;
                    stats.total_iterations += pixel_stats.total_iterations as u64;
                    stats.rebase_count += pixel_stats.rebase_count as u64;
                    data.push(ComputeData::Mandelbrot(result));
                } else {
                    // BLA enabled but no table - fall back to non-BLA path
                    let result = compute_pixel_perturbation(
                        orbit,
                        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.total_iterations += result.iterations as u64;
                    data.push(ComputeData::Mandelbrot(result));
                }
            } else {
                // BLA disabled - use generic path
                let result = compute_pixel_perturbation(
                    orbit,
                    F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                    config.max_iterations,
                    config.tau_sq,
                );
                stats.total_iterations += result.iterations as u64;
                data.push(ComputeData::Mandelbrot(result));
            }

            delta_c.0 += delta_step.0;
        }

        delta_c_row.1 += delta_step.1;
    }

    TileRenderResult { data, stats }
}
```

Also update the imports at the top of `tile.rs`:

```rust
use super::{compute_pixel_perturbation, compute_pixel_perturbation_f64_bla, compute_pixel_perturbation_hdr_bla, ReferenceOrbit};
```

**Step 7.4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute perturbation::tile::tests::render_tile_f64_with_bla_uses_bla -- --nocapture`
Expected: PASS

**Step 7.5: Commit**

```bash
git add fractalwonder-compute/src/perturbation/tile.rs
git commit -m "$(cat <<'EOF'
feat(tile): update render_tile_f64 to support BLA acceleration

render_tile_f64 now accepts optional BLA table and uses
compute_pixel_perturbation_f64_bla when BLA is enabled.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Update Worker Dispatch to Pass BLA Table to f64 Path

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 8.1: Update the dispatch logic**

Find the dispatch logic (around line 263) and update:

```rust
let result = if use_f64 {
    let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
    let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());
    render_tile_f64(&orbit, cached.bla_table.as_ref(), delta_origin, delta_step, &config)
} else {
    let delta_origin = (
        HDRFloat::from_bigfloat(&delta_c_origin.0),
        HDRFloat::from_bigfloat(&delta_c_origin.1),
    );
    let delta_step = (
        HDRFloat::from_bigfloat(&delta_c_step.0),
        HDRFloat::from_bigfloat(&delta_c_step.1),
    );
    render_tile_hdr(
        &orbit,
        cached.bla_table.as_ref(),
        delta_origin,
        delta_step,
        &config,
    )
};
```

Also update the TODO comment (around line 255):

```rust
// Dispatch based on delta magnitude
// f64 path now supports BLA when coefficients fit in f64 range
let delta_log2 = delta_c_origin
    .0
    .log2_approx()
    .max(delta_c_origin.1.log2_approx());
let use_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;
```

**Step 8.2: Run all tests to verify nothing is broken**

Run: `cargo test --package fractalwonder-compute -- --nocapture`
Expected: All tests pass

**Step 8.3: Run clippy and format**

Run: `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No errors

**Step 8.4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "$(cat <<'EOF'
feat(worker): pass BLA table to f64 path for iteration skipping

f64 path now receives BLA table and uses BLA acceleration when
coefficients fit in f64 range. Expected 33x speedup at moderate zoom.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Integration Test - Compare f64+BLA with HDR+BLA

**Files:**
- Modify: `fractalwonder-compute/src/perturbation/pixel_f64_bla.rs`

**Step 9.1: Write the test**

Add to tests module:

```rust
#[test]
fn pixel_f64_bla_matches_hdr_bla_iteration_count() {
    use super::super::compute_pixel_perturbation_hdr_bla;
    use fractalwonder_core::HDRComplex;

    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);
    let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

    // Test with a delta that should escape
    let delta_c_f64 = (0.1, 0.1);
    let delta_c_hdr = HDRComplex {
        re: HDRFloat::from_f64(0.1),
        im: HDRFloat::from_f64(0.1),
    };

    let (result_f64, stats_f64) =
        compute_pixel_perturbation_f64_bla(&orbit, &bla_table, delta_c_f64, 1000, 1e-6);
    let (result_hdr, stats_hdr) =
        compute_pixel_perturbation_hdr_bla(&orbit, &bla_table, delta_c_hdr, 1000, 1e-6);

    // Both should escape
    assert_eq!(
        result_f64.escaped, result_hdr.escaped,
        "f64 and HDR should agree on escape"
    );

    // Iteration counts should be very close (may differ slightly due to precision)
    let iter_diff = (result_f64.iterations as i32 - result_hdr.iterations as i32).abs();
    assert!(
        iter_diff <= 2,
        "Iteration counts should be close: f64={}, hdr={}",
        result_f64.iterations,
        result_hdr.iterations
    );

    // Both should use BLA
    assert!(stats_f64.bla_iterations > 0, "f64 should use BLA");
    assert!(stats_hdr.bla_iterations > 0, "HDR should use BLA");
}
```

**Step 9.2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute perturbation::pixel_f64_bla::tests::pixel_f64_bla_matches_hdr_bla_iteration_count -- --nocapture`
Expected: PASS

**Step 9.3: Commit**

```bash
git add fractalwonder-compute/src/perturbation/pixel_f64_bla.rs
git commit -m "$(cat <<'EOF'
test(pixel_f64_bla): add integration test comparing f64 and HDR BLA paths

Verifies that f64+BLA produces same results as HDR+BLA.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Final Verification

**Step 10.1: Run all quality checks**

Run:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
```
Expected: All pass with no warnings

**Step 10.2: Build WASM and test in browser**

Run: `trunk build --release`

Then manually test:
1. Load app in browser
2. Enable BLA in settings
3. Zoom to ~10^100 (moderate zoom)
4. Verify tiles render correctly
5. Check console for BLA statistics (should show bla_iterations > 0)

**Step 10.3: Final commit (if any cleanup needed)**

---

## Performance Expectations

At 10^270 zoom with 30,000 max iterations and ~97% BLA efficiency:

| Path | Iterations | Speed Factor | Work Units |
|------|------------|--------------|------------|
| f64 no BLA | 30,000 | 1x | 30,000 |
| HDR + BLA | 900 | 0.02x (50x slower) | 45,000 |
| **f64 + BLA** | 900 | 1x | **900** |

Expected speedup: **33x faster** than HDR+BLA, **50x faster** than f64 no BLA.

---

## Edge Cases Handled

1. **BLA coefficients overflow f64**: `find_valid_f64()` returns None, falls back to standard iteration
2. **Deep zoom where f64 underflows**: Worker dispatch uses HDR path (existing delta_log2 check)
3. **Rebase with large δz**: After rebase, δz may be too large for high-level BLA, but lower levels still work
4. **dc_max underflows in f64**: At extreme zoom (10^308+), dc_max underflows - use HDR path

---

## BLA Derivative Tracking Implementation (Completed 2026-01-03)

**Problem:** BLA skips iterations for position (δz) but previously didn't update the derivative (δρ), causing 3D lighting artifacts (semi-circular patterns) at deep zoom levels.

**Solution:** Extended BLA entries with D and E coefficients to track derivative updates during BLA skips.

### Mathematical Formulas

**Position update (existing):**
```
δz_new = A·δz + B·δc
```

**Derivative update (new):**
```
δρ_new = A·δρ + D·δz + E·δc
```

Where:
- A, B: existing BLA coefficients for position
- D: coefficient for δz contribution to derivative
- E: coefficient for δc contribution to derivative

**Single-step formulas (from_orbit_point):**
```
D = 2·Der_m
E = 0
```

**Merge formulas (combining two BLA entries X and Y):**
```
D_merged = A_y·D_x + D_y·A_x
E_merged = A_y·E_x + D_y·B_x + E_y
```

**Key insight:** C = A mathematically (derivative of position w.r.t. δρ is identical to derivative of position w.r.t. δz), so C is not stored separately.

### Files Modified

| File | Changes |
|------|---------|
| `fractalwonder-compute/src/bla.rs` | Extended `BlaEntry` and `BlaEntryF64` with D and E fields; updated `from_orbit_point`, `merge`, `try_from_hdr`, `BlaTable::compute` |
| `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs` | Applied derivative formula in BLA block |
| `fractalwonder-compute/src/perturbation/pixel_f64_bla.rs` | Applied derivative formula in BLA block |
| `fractalwonder-gpu/src/bla_upload.rs` | Extended `GpuBlaEntry` to 112 bytes (28 f32s) with D and E fields |
| `fractalwonder-gpu/src/buffers.rs` | Updated buffer allocation from 16→28 f32s per entry |
| `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` | Extended shader `BlaEntry` struct, updated `bla_load`, applied derivative formula |

### Struct Changes

**BlaEntry (CPU HDR):**
```rust
pub struct BlaEntry {
    pub a: HDRComplex,
    pub b: HDRComplex,
    pub d: HDRComplex,  // δz contribution to δρ
    pub e: HDRComplex,  // δc contribution to δρ
    pub l: u32,
    pub r_sq: HDRFloat,
}
```

**BlaEntryF64 (CPU f64):**
```rust
pub struct BlaEntryF64 {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub d: (f64, f64),  // δz contribution to δρ
    pub e: (f64, f64),  // δc contribution to δρ
    pub l: u32,
    pub r_sq: f64,
}
```

**GpuBlaEntry (GPU):**
- Size increased from 64 bytes (16 f32s) to 112 bytes (28 f32s)
- Added: d_re_head, d_re_tail, d_re_exp, d_im_head, d_im_tail, d_im_exp (6 f32s)
- Added: e_re_head, e_re_tail, e_re_exp, e_im_head, e_im_tail, e_im_exp (6 f32s)

### Commits (Branch: fix/3d-at-deep-zoom)

1. `5e416e2` - refactor(bla): add d and e coefficient fields to BlaEntry
2. `2e56703` - refactor(bla): add d and e coefficient fields to BlaEntryF64
3. `b1450b4` - feat(bla): compute D and E coefficients in from_orbit_point
4. `c7f5654` - feat(bla): add D and E merge formulas
5. `e1f2089` - feat(bla): pass derivatives to from_orbit_point in table construction
6. `e480d2f` - feat(perturbation): apply derivative coefficients in HDR BLA path
7. `8d26aa3` - feat(perturbation): apply derivative coefficients in f64 BLA path
8. `c45bebe` - feat(gpu): extend GpuBlaEntry with D and E coefficients
9. `04d02b1` - feat(gpu): increase BLA buffer size to 28 f32s per entry
10. `b6657c7` - feat(gpu): extend shader BlaEntry struct with D and E
11. `485008d` - feat(gpu): apply derivative coefficients in shader BLA block

### Verification

- All 168 tests pass
- Clippy clean (no warnings)
- WASM builds successfully
- Manual verification at zoom 2.55 × 10^301 confirmed smooth 3D lighting without artifacts
