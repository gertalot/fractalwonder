# BLA Acceleration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add BLA (Bivariate Linear Approximation) to skip iterations and achieve 10-100x speedup at deep zoom.

**Architecture:** Build a BlaTable from the reference orbit (once), then use it during pixel iteration to skip multiple iterations when the linear approximation is valid. Falls back to standard iteration near critical points.

**Tech Stack:** Rust, FloatExp (existing), ReferenceOrbit (existing)

---

## Overview

Bivariate Linear Approximation (BLA) skips multiple iterations when the nonlinear term `δz²` is negligible compared to `2Zδz`. This transforms O(n) per-pixel iteration into O(log n) in favorable cases, providing 10-100x speedup at deep zoom with high iteration counts.

## Mathematical Foundation

### The Approximation

Standard perturbation iteration:
```
δz' = 2·Z·δz + δz² + δc
```

When `|δz²| << |2·Z·δz|`, we can drop the squared term:
```
δz' ≈ 2·Z·δz + δc
```

This linear form allows combining multiple iterations into a single operation.

### Single-Iteration BLA

At reference iteration m, a BLA entry has:
```
A_m = 2·Z_m       (coefficient for δz)
B_m = 1           (coefficient for δc)
l_m = 1           (iterations skipped)
r_m = ε·|Z_m|     (validity radius, ε ≈ 2⁻⁵³)
```

Applying: `δz_new = A·δz + B·δc`

Valid when: `|δz| < r`

### Merging BLAs

Two adjacent BLAs (x at iteration m, y at iteration m+l_x) merge into one that skips `l_x + l_y` iterations:

```
A_merged = A_y · A_x
B_merged = A_y · B_x + B_y
l_merged = l_x + l_y
r_merged = min(r_x, max(0, (r_y - |B_x|·|δc_max|) / |A_x|))
```

This builds a binary tree: M single-iteration BLAs → M/2 two-iteration → M/4 four-iteration → ... → 1 skip-all BLA.

## Data Structures

```rust
/// Single BLA entry: skips `l` iterations starting at reference index `start_m`
#[derive(Clone, Debug)]
pub struct BlaEntry {
    pub a_re: f64,      // Real part of coefficient A
    pub a_im: f64,      // Imaginary part of coefficient A
    pub b_re: f64,      // Real part of coefficient B
    pub b_im: f64,      // Imaginary part of coefficient B
    pub l: u32,         // Number of iterations this BLA skips
    pub r_sq: f64,      // Validity radius squared (compare with |δz|²)
}

/// BLA table for a reference orbit, organized as a binary tree
pub struct BlaTable {
    /// All BLA entries, organized by level:
    /// - Level 0 (indices 0..M): skip 1 iteration, start at m=0,1,2,...
    /// - Level 1 (indices M..M+M/2): skip 2 iterations, start at m=0,2,4,...
    /// - Level 2 (indices M+M/2..M+M/2+M/4): skip 4, start at m=0,4,8,...
    /// - ...
    entries: Vec<BlaEntry>,

    /// Start index in `entries` for each level
    level_offsets: Vec<usize>,

    /// Number of levels (log2(M) + 1)
    num_levels: usize,

    /// Maximum |δc| for this render (needed for validity computation)
    dc_max: f64,
}
```

## Construction Algorithm

```rust
impl BlaTable {
    pub fn compute(orbit: &ReferenceOrbit, dc_max: f64) -> Self {
        let m = orbit.orbit.len();
        let num_levels = (m as f64).log2().ceil() as usize + 1;

        // Allocate: M + M/2 + M/4 + ... ≈ 2M entries
        let total_entries = 2 * m;
        let mut entries = Vec::with_capacity(total_entries);
        let mut level_offsets = Vec::with_capacity(num_levels);

        // Level 0: single-iteration BLAs from reference orbit
        level_offsets.push(0);
        let epsilon = 2.0_f64.powi(-53);  // f64 precision

        for (z_re, z_im) in &orbit.orbit {
            let z_mag = (z_re * z_re + z_im * z_im).sqrt();
            let r = epsilon * z_mag;

            entries.push(BlaEntry {
                a_re: 2.0 * z_re,
                a_im: 2.0 * z_im,
                b_re: 1.0,
                b_im: 0.0,
                l: 1,
                r_sq: r * r,
            });
        }

        // Build higher levels by merging pairs
        let mut level_size = m;
        for level in 1..num_levels {
            level_offsets.push(entries.len());
            let prev_offset = level_offsets[level - 1];
            level_size = (level_size + 1) / 2;

            for i in 0..level_size {
                let x_idx = prev_offset + 2 * i;
                let y_idx = prev_offset + 2 * i + 1;

                // If no pair, copy single entry
                if y_idx >= level_offsets[level] {
                    entries.push(entries[x_idx].clone());
                    continue;
                }

                let x = &entries[x_idx];
                let y = &entries[y_idx];

                // Merge: A = Ay * Ax, B = Ay * Bx + By
                let merged = BlaEntry::merge(x, y, dc_max);
                entries.push(merged);
            }
        }

        Self { entries, level_offsets, num_levels, dc_max }
    }
}

impl BlaEntry {
    fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: f64) -> BlaEntry {
        // A_merged = A_y * A_x (complex multiplication)
        let a_re = y.a_re * x.a_re - y.a_im * x.a_im;
        let a_im = y.a_re * x.a_im + y.a_im * x.a_re;

        // B_merged = A_y * B_x + B_y
        let b_re = (y.a_re * x.b_re - y.a_im * x.b_im) + y.b_re;
        let b_im = (y.a_re * x.b_im + y.a_im * x.b_re) + y.b_im;

        // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
        let r_x = x.r_sq.sqrt();
        let r_y = y.r_sq.sqrt();
        let b_x_mag = (x.b_re * x.b_re + x.b_im * x.b_im).sqrt();
        let a_x_mag = (x.a_re * x.a_re + x.a_im * x.a_im).sqrt();

        let r_adjusted = (r_y - b_x_mag * dc_max).max(0.0) / a_x_mag.max(1e-300);
        let r = r_x.min(r_adjusted);

        BlaEntry {
            a_re, a_im, b_re, b_im,
            l: x.l + y.l,
            r_sq: r * r,
        }
    }
}
```

## Lookup Algorithm

```rust
impl BlaTable {
    /// Find the largest valid BLA at reference index `m` for current |δz|²
    pub fn find_valid(&self, m: usize, dz_mag_sq: f64) -> Option<&BlaEntry> {
        // Search from highest level (largest skips) down to level 0
        for level in (0..self.num_levels).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1 << level;  // 2^level iterations per entry

            // Index within this level
            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            if entry_idx >= self.entries.len() {
                continue;
            }

            let entry = &self.entries[entry_idx];

            // Check validity: |δz|² < r²
            if dz_mag_sq < entry.r_sq {
                return Some(entry);
            }
        }

        None  // No valid BLA found, use standard iteration
    }
}
```

## Modified Pixel Loop

```rust
pub fn compute_pixel_perturbation_floatexp_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
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
    let mut n = 0u32;

    while n < max_iterations {
        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];

        // z = Z_m + δz
        let z_re = FloatExp::from_f64(z_m_re).add(&dz_re);
        let z_im = FloatExp::from_f64(z_m_im).add(&dz_im);

        let z_mag_sq = FloatExp::norm_sq(&z_re, &z_im);
        let dz_mag_sq = FloatExp::norm_sq(&dz_re, &dz_im);

        // 1. Escape check
        if z_mag_sq > 4.0 {
            return MandelbrotData { iterations: n, max_iterations, escaped: true, glitched };
        }

        // 2. Pauldelbrot glitch detection
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz_re = z_re;
            dz_im = z_im;
            m = 0;
            n += 1;
            continue;
        }

        // 4. Try BLA acceleration
        if let Some(bla) = bla_table.find_valid(m, dz_mag_sq) {
            // Apply BLA: δz_new = A·δz + B·δc
            let new_dz_re = dz_re.mul_f64(bla.a_re).sub(&dz_im.mul_f64(bla.a_im))
                .add(&dc_re.mul_f64(bla.b_re)).sub(&dc_im.mul_f64(bla.b_im));
            let new_dz_im = dz_re.mul_f64(bla.a_im).add(&dz_im.mul_f64(bla.a_re))
                .add(&dc_re.mul_f64(bla.b_im)).add(&dc_im.mul_f64(bla.b_re));

            dz_re = new_dz_re;
            dz_im = new_dz_im;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration (no valid BLA)
            let two_z_dz_re = dz_re.mul_f64(z_m_re).sub(&dz_im.mul_f64(z_m_im)).mul_f64(2.0);
            let two_z_dz_im = dz_re.mul_f64(z_m_im).add(&dz_im.mul_f64(z_m_re)).mul_f64(2.0);

            let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
            let dz_sq_im = dz_re.mul(&dz_im).mul_f64(2.0);

            dz_re = two_z_dz_re.add(&dz_sq_re).add(&dc_re);
            dz_im = two_z_dz_im.add(&dz_sq_im).add(&dc_im);
            m += 1;
            n += 1;
        }
    }

    MandelbrotData { iterations: max_iterations, max_iterations, escaped: false, glitched }
}
```

## Memory Usage

| Orbit Length | BLA Table Size |
|--------------|----------------|
| 10,000 | ~800 KB |
| 100,000 | ~8 MB |
| 1,000,000 | ~80 MB |

Each BlaEntry is 48 bytes (4×f64 + u32 + f64, with padding).

## Future Optimizations

1. **Merge-and-cull**: Discard BLAs with r < threshold (Phil Thompson keeps ~500)
2. **Periodic reference optimization**: If reference is periodic, BLA table can be period-length only
3. **Adaptive epsilon**: Auto-tune ε based on zoom depth (smaller at deeper zoom)

## Testing Strategy

1. **Correctness**: BLA version must produce identical iteration counts to non-BLA version
2. **Validity**: Verify BLA is never applied when |δz| >= r (would produce wrong results)
3. **Performance**: Benchmark speedup at various zoom depths and iteration counts

## Sources

- [Phil Thompson: Faster Mandelbrot Set Rendering with BLA](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html)
- [mathr: Deep zoom theory and practice (again)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
- [Zhuoran's original BLA work](https://www.deviantart.com/microfractal/journal/New-deep-zoom-algorithms-for-fractals-933730336)

---

# Implementation Tasks

## Task 1: BlaEntry struct and basic tests

**Files:**
- Create: `fractalwonder-compute/src/bla.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Write the failing test for BlaEntry creation**

Add to `fractalwonder-compute/src/bla.rs`:

```rust
//! Bivariate Linear Approximation for iteration skipping.

/// Single BLA entry: skips `l` iterations.
/// Applies: δz_new = A·δz + B·δc
#[derive(Clone, Debug, PartialEq)]
pub struct BlaEntry {
    pub a_re: f64,
    pub a_im: f64,
    pub b_re: f64,
    pub b_im: f64,
    pub l: u32,
    pub r_sq: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bla_entry_from_orbit_point() {
        // Z = (1.0, 0.5), ε = 2^-53
        // A = 2Z = (2.0, 1.0)
        // B = 1
        // r = ε·|Z| = ε·√(1 + 0.25) ≈ ε·1.118
        let entry = BlaEntry::from_orbit_point(1.0, 0.5);

        assert!((entry.a_re - 2.0).abs() < 1e-14);
        assert!((entry.a_im - 1.0).abs() < 1e-14);
        assert!((entry.b_re - 1.0).abs() < 1e-14);
        assert!((entry.b_im - 0.0).abs() < 1e-14);
        assert_eq!(entry.l, 1);

        let z_mag = (1.0_f64 * 1.0 + 0.5 * 0.5).sqrt();
        let epsilon = 2.0_f64.powi(-53);
        let expected_r_sq = (epsilon * z_mag).powi(2);
        assert!((entry.r_sq - expected_r_sq).abs() < 1e-40);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_entry_from_orbit_point`
Expected: FAIL with "cannot find function `from_orbit_point`"

**Step 3: Write minimal implementation**

Add to `BlaEntry` impl in `fractalwonder-compute/src/bla.rs`:

```rust
impl BlaEntry {
    /// Create a single-iteration BLA from a reference orbit point Z = (z_re, z_im).
    pub fn from_orbit_point(z_re: f64, z_im: f64) -> Self {
        let epsilon = 2.0_f64.powi(-53);
        let z_mag = (z_re * z_re + z_im * z_im).sqrt();
        let r = epsilon * z_mag;

        Self {
            a_re: 2.0 * z_re,
            a_im: 2.0 * z_im,
            b_re: 1.0,
            b_im: 0.0,
            l: 1,
            r_sq: r * r,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-compute bla_entry_from_orbit_point`
Expected: PASS

**Step 5: Register the module**

Add to `fractalwonder-compute/src/lib.rs` after `mod perturbation;`:

```rust
mod bla;
pub use bla::{BlaEntry, BlaTable};
```

**Step 6: Run all tests**

Run: `cargo test -p fractalwonder-compute`
Expected: All tests pass (BlaTable not found yet is OK - we'll add it next)

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/bla.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(bla): add BlaEntry struct with from_orbit_point constructor"
```

---

## Task 2: BlaEntry merge operation

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 1: Write the failing test for merge**

Add to tests in `fractalwonder-compute/src/bla.rs`:

```rust
    #[test]
    fn bla_entry_merge_two_single_iterations() {
        // Two single-iteration BLAs should merge into one that skips 2
        let x = BlaEntry::from_orbit_point(1.0, 0.0);  // Z = 1
        let y = BlaEntry::from_orbit_point(0.5, 0.0);  // Z = 0.5

        let dc_max = 0.001;  // Small delta_c
        let merged = BlaEntry::merge(&x, &y, dc_max);

        // l should be 2
        assert_eq!(merged.l, 2);

        // A_merged = A_y * A_x = (1.0, 0) * (2.0, 0) = (2.0, 0)
        // (complex multiply: (1)(2) - (0)(0) = 2, (1)(0) + (0)(2) = 0)
        assert!((merged.a_re - 2.0).abs() < 1e-14);
        assert!((merged.a_im - 0.0).abs() < 1e-14);

        // B_merged = A_y * B_x + B_y = (1,0)*(1,0) + (1,0) = (2,0)
        assert!((merged.b_re - 2.0).abs() < 1e-14);
        assert!((merged.b_im - 0.0).abs() < 1e-14);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_entry_merge`
Expected: FAIL with "cannot find function `merge`"

**Step 3: Write minimal implementation**

Add to `BlaEntry` impl:

```rust
    /// Merge two BLAs: x (first) then y (second).
    /// Result skips l_x + l_y iterations.
    pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: f64) -> BlaEntry {
        // A_merged = A_y * A_x (complex multiplication)
        let a_re = y.a_re * x.a_re - y.a_im * x.a_im;
        let a_im = y.a_re * x.a_im + y.a_im * x.a_re;

        // B_merged = A_y * B_x + B_y
        let b_re = (y.a_re * x.b_re - y.a_im * x.b_im) + y.b_re;
        let b_im = (y.a_re * x.b_im + y.a_im * x.b_re) + y.b_im;

        // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
        let r_x = x.r_sq.sqrt();
        let r_y = y.r_sq.sqrt();
        let b_x_mag = (x.b_re * x.b_re + x.b_im * x.b_im).sqrt();
        let a_x_mag = (x.a_re * x.a_re + x.a_im * x.a_im).sqrt();

        let r_adjusted = (r_y - b_x_mag * dc_max).max(0.0) / a_x_mag.max(1e-300);
        let r = r_x.min(r_adjusted);

        BlaEntry {
            a_re,
            a_im,
            b_re,
            b_im,
            l: x.l + y.l,
            r_sq: r * r,
        }
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-compute bla_entry_merge`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): add BlaEntry::merge for combining adjacent BLAs"
```

---

## Task 3: BlaTable construction (Level 0 only)

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 1: Write the failing test**

Add to tests:

```rust
    use crate::ReferenceOrbit;
    use fractalwonder_core::BigFloat;

    #[test]
    fn bla_table_level_0_has_one_entry_per_orbit_point() {
        // Create a simple reference orbit
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        let dc_max = 0.01;
        let table = BlaTable::compute(&orbit, dc_max);

        // Level 0 should have orbit.len() entries
        assert!(table.entries.len() >= orbit.orbit.len());

        // First entry should match first orbit point
        let z0 = orbit.orbit[0];
        let expected = BlaEntry::from_orbit_point(z0.0, z0.1);
        assert_eq!(table.entries[0].l, expected.l);
        assert!((table.entries[0].a_re - expected.a_re).abs() < 1e-14);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_table_level_0`
Expected: FAIL with "cannot find struct `BlaTable`"

**Step 3: Write minimal implementation**

Add to `fractalwonder-compute/src/bla.rs`:

```rust
use crate::ReferenceOrbit;

/// BLA table for a reference orbit, organized as a binary tree.
pub struct BlaTable {
    pub entries: Vec<BlaEntry>,
    pub level_offsets: Vec<usize>,
    pub num_levels: usize,
    dc_max: f64,
}

impl BlaTable {
    /// Compute BLA table from a reference orbit.
    pub fn compute(orbit: &ReferenceOrbit, dc_max: f64) -> Self {
        let m = orbit.orbit.len();
        if m == 0 {
            return Self {
                entries: vec![],
                level_offsets: vec![0],
                num_levels: 0,
                dc_max,
            };
        }

        let num_levels = ((m as f64).log2().ceil() as usize).max(1);
        let mut entries = Vec::with_capacity(2 * m);
        let mut level_offsets = Vec::with_capacity(num_levels);

        // Level 0: single-iteration BLAs
        level_offsets.push(0);
        for &(z_re, z_im) in &orbit.orbit {
            entries.push(BlaEntry::from_orbit_point(z_re, z_im));
        }

        // Higher levels will be added in next task

        Self {
            entries,
            level_offsets,
            num_levels,
            dc_max,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p fractalwonder-compute bla_table_level_0`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): add BlaTable with Level 0 construction"
```

---

## Task 4: BlaTable binary tree construction (all levels)

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 1: Write the failing test**

Add to tests:

```rust
    #[test]
    fn bla_table_has_multiple_levels() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 16);

        let table = BlaTable::compute(&orbit, 0.01);

        // 16 entries -> should have ~log2(16)+1 = 5 levels
        // Level 0: 16 entries (skip 1)
        // Level 1: 8 entries (skip 2)
        // Level 2: 4 entries (skip 4)
        // Level 3: 2 entries (skip 8)
        // Level 4: 1 entry (skip 16)
        assert!(table.num_levels >= 4, "Expected at least 4 levels, got {}", table.num_levels);
        assert!(table.level_offsets.len() >= 4);

        // Total entries should be ~2M
        assert!(table.entries.len() >= 16 + 8 + 4 + 2);
    }

    #[test]
    fn bla_table_higher_level_entries_skip_more() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 16);

        let table = BlaTable::compute(&orbit, 0.01);

        // Level 0 entries skip 1
        assert_eq!(table.entries[0].l, 1);

        // Level 1 entries skip 2
        let level1_start = table.level_offsets[1];
        assert_eq!(table.entries[level1_start].l, 2);

        // Level 2 entries skip 4
        if table.level_offsets.len() > 2 {
            let level2_start = table.level_offsets[2];
            assert_eq!(table.entries[level2_start].l, 4);
        }
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_table_has_multiple`
Expected: FAIL (only 1 level currently)

**Step 3: Update implementation**

Replace the `BlaTable::compute` function:

```rust
    pub fn compute(orbit: &ReferenceOrbit, dc_max: f64) -> Self {
        let m = orbit.orbit.len();
        if m == 0 {
            return Self {
                entries: vec![],
                level_offsets: vec![0],
                num_levels: 0,
                dc_max,
            };
        }

        let num_levels = ((m as f64).log2().ceil() as usize).max(1) + 1;
        let mut entries = Vec::with_capacity(2 * m);
        let mut level_offsets = Vec::with_capacity(num_levels);

        // Level 0: single-iteration BLAs
        level_offsets.push(0);
        for &(z_re, z_im) in &orbit.orbit {
            entries.push(BlaEntry::from_orbit_point(z_re, z_im));
        }

        // Build higher levels by merging pairs
        let mut prev_level_size = m;
        for _level in 1..num_levels {
            let prev_offset = *level_offsets.last().unwrap();
            level_offsets.push(entries.len());

            let this_level_size = (prev_level_size + 1) / 2;

            for i in 0..this_level_size {
                let x_idx = prev_offset + 2 * i;
                let y_idx = prev_offset + 2 * i + 1;

                if y_idx >= entries.len() {
                    // Odd number: copy last entry unchanged
                    entries.push(entries[x_idx].clone());
                } else {
                    let merged = BlaEntry::merge(&entries[x_idx], &entries[y_idx], dc_max);
                    entries.push(merged);
                }
            }

            prev_level_size = this_level_size;
            if this_level_size <= 1 {
                break;
            }
        }

        Self {
            entries,
            level_offsets,
            num_levels: level_offsets.len(),
            dc_max,
        }
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute bla_table`
Expected: All BLA table tests pass

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): complete binary tree construction for all levels"
```

---

## Task 5: BlaTable::find_valid lookup

**Files:**
- Modify: `fractalwonder-compute/src/bla.rs`

**Step 1: Write the failing test**

Add to tests:

```rust
    #[test]
    fn bla_table_find_valid_returns_none_for_large_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, 0.01);

        // With |δz|² = 1.0 (huge), no BLA should be valid
        let result = table.find_valid(0, 1.0);
        assert!(result.is_none(), "Large |δz| should invalidate all BLAs");
    }

    #[test]
    fn bla_table_find_valid_returns_some_for_tiny_dz() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);
        let table = BlaTable::compute(&orbit, 0.01);

        // With |δz|² = 0 (at start), some BLA should be valid
        let result = table.find_valid(0, 0.0);
        assert!(result.is_some(), "Zero |δz| should allow BLA");

        // Should skip multiple iterations
        let bla = result.unwrap();
        assert!(bla.l >= 1);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_table_find_valid`
Expected: FAIL with "no method named `find_valid`"

**Step 3: Write implementation**

Add to `BlaTable` impl:

```rust
    /// Find the largest valid BLA at reference index `m` for current |δz|².
    /// Returns None if no BLA is valid (fallback to standard iteration).
    pub fn find_valid(&self, m: usize, dz_mag_sq: f64) -> Option<&BlaEntry> {
        if self.entries.is_empty() {
            return None;
        }

        // Search from highest level (largest skips) down to level 0
        for level in (0..self.num_levels).rev() {
            let level_start = self.level_offsets[level];
            let skip_size = 1usize << level;  // 2^level iterations per entry at this level

            // Index within this level for reference index m
            let idx_in_level = m / skip_size;
            let entry_idx = level_start + idx_in_level;

            // Check bounds
            let level_end = if level + 1 < self.level_offsets.len() {
                self.level_offsets[level + 1]
            } else {
                self.entries.len()
            };

            if entry_idx >= level_end {
                continue;
            }

            let entry = &self.entries[entry_idx];

            // Check validity: |δz|² < r²
            if dz_mag_sq < entry.r_sq {
                return Some(entry);
            }
        }

        None
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute bla_table_find_valid`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/bla.rs
git commit -m "feat(bla): add find_valid lookup for largest valid skip"
```

---

## Task 6: compute_pixel_perturbation_floatexp_bla function

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-compute/src/perturbation.rs` tests:

```rust
    use crate::bla::BlaTable;

    #[test]
    fn bla_version_matches_non_bla_for_escaping_point() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        let delta_c = (FloatExp::from_f64(2.5), FloatExp::from_f64(0.0));
        let dc_max = 2.5;
        let bla_table = BlaTable::compute(&orbit, dc_max);

        // Non-BLA version
        let result_no_bla = compute_pixel_perturbation_floatexp(&orbit, delta_c, 500, TEST_TAU_SQ);

        // BLA version
        let result_bla = compute_pixel_perturbation_floatexp_bla(
            &orbit, &bla_table, delta_c, 500, TEST_TAU_SQ
        );

        assert_eq!(result_no_bla.escaped, result_bla.escaped);
        assert_eq!(result_no_bla.iterations, result_bla.iterations);
    }

    #[test]
    fn bla_version_matches_non_bla_for_in_set_point() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        let delta_c = (FloatExp::from_f64(0.01), FloatExp::from_f64(0.01));
        let dc_max = 0.02;
        let bla_table = BlaTable::compute(&orbit, dc_max);

        let result_no_bla = compute_pixel_perturbation_floatexp(&orbit, delta_c, 500, TEST_TAU_SQ);
        let result_bla = compute_pixel_perturbation_floatexp_bla(
            &orbit, &bla_table, delta_c, 500, TEST_TAU_SQ
        );

        assert_eq!(result_no_bla.escaped, result_bla.escaped);
        assert_eq!(result_no_bla.iterations, result_bla.iterations);
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p fractalwonder-compute bla_version_matches`
Expected: FAIL with "cannot find function `compute_pixel_perturbation_floatexp_bla`"

**Step 3: Write implementation**

Add to `fractalwonder-compute/src/perturbation.rs`:

```rust
use crate::bla::BlaTable;

/// Compute pixel using perturbation with FloatExp deltas and BLA acceleration.
pub fn compute_pixel_perturbation_floatexp_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
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

    let mut n = 0u32;

    while n < max_iterations {
        let (z_m_re, z_m_im) = orbit.orbit[m % orbit_len];

        // z = Z_m + δz
        let z_re = FloatExp::from_f64(z_m_re).add(&dz_re);
        let z_im = FloatExp::from_f64(z_m_im).add(&dz_im);

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
            n += 1;
            continue;
        }

        // 4. Try BLA acceleration
        if let Some(bla) = bla_table.find_valid(m, dz_mag_sq) {
            // Apply BLA: δz_new = A·δz + B·δc
            let new_dz_re = dz_re
                .mul_f64(bla.a_re)
                .sub(&dz_im.mul_f64(bla.a_im))
                .add(&dc_re.mul_f64(bla.b_re))
                .sub(&dc_im.mul_f64(bla.b_im));
            let new_dz_im = dz_re
                .mul_f64(bla.a_im)
                .add(&dz_im.mul_f64(bla.a_re))
                .add(&dc_re.mul_f64(bla.b_im))
                .add(&dc_im.mul_f64(bla.b_re));

            dz_re = new_dz_re;
            dz_im = new_dz_im;
            m += bla.l as usize;
            n += bla.l;
        } else {
            // 5. Standard delta iteration (no valid BLA)
            let two_z_dz_re = dz_re
                .mul_f64(z_m_re)
                .sub(&dz_im.mul_f64(z_m_im))
                .mul_f64(2.0);
            let two_z_dz_im = dz_re
                .mul_f64(z_m_im)
                .add(&dz_im.mul_f64(z_m_re))
                .mul_f64(2.0);

            let dz_sq_re = dz_re.mul(&dz_re).sub(&dz_im.mul(&dz_im));
            let dz_sq_im = dz_re.mul(&dz_im).mul_f64(2.0);

            dz_re = two_z_dz_re.add(&dz_sq_re).add(&dc_re);
            dz_im = two_z_dz_im.add(&dz_sq_im).add(&dc_im);
            m += 1;
            n += 1;
        }
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
    }
}
```

**Step 4: Export the function**

Update `fractalwonder-compute/src/lib.rs`:

```rust
pub use perturbation::{
    compute_pixel_perturbation, compute_pixel_perturbation_bigfloat,
    compute_pixel_perturbation_floatexp, compute_pixel_perturbation_floatexp_bla,
    ReferenceOrbit,
};
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute bla_version_matches`
Expected: PASS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(bla): add compute_pixel_perturbation_floatexp_bla with BLA acceleration"
```

---

## Task 7: Comprehensive correctness tests

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs`

**Step 1: Write comprehensive tests**

Add to tests:

```rust
    #[test]
    fn bla_matches_non_bla_for_many_deltas() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);

        let test_deltas = [
            (0.01, 0.01),
            (-0.005, 0.002),
            (0.1, -0.05),
            (0.0, 0.001),
            (0.5, 0.5),
            (-0.1, 0.1),
        ];

        for (dx, dy) in test_deltas {
            let delta_c = (FloatExp::from_f64(dx), FloatExp::from_f64(dy));
            let dc_max = (dx.abs() + dy.abs()).max(0.001);
            let bla_table = BlaTable::compute(&orbit, dc_max);

            let result_no_bla = compute_pixel_perturbation_floatexp(&orbit, delta_c, 1000, TEST_TAU_SQ);
            let result_bla = compute_pixel_perturbation_floatexp_bla(
                &orbit, &bla_table, delta_c, 1000, TEST_TAU_SQ
            );

            assert_eq!(
                result_no_bla.escaped, result_bla.escaped,
                "Escape mismatch for delta ({}, {})", dx, dy
            );
            assert_eq!(
                result_no_bla.iterations, result_bla.iterations,
                "Iteration mismatch for delta ({}, {}): no_bla={}, bla={}",
                dx, dy, result_no_bla.iterations, result_bla.iterations
            );
        }
    }

    #[test]
    fn bla_handles_rebasing() {
        let c_ref = (
            BigFloat::with_precision(-0.75, 128),
            BigFloat::with_precision(0.1, 128),
        );
        let orbit = ReferenceOrbit::compute(&c_ref, 500);

        let delta_c = (FloatExp::from_f64(0.1), FloatExp::from_f64(0.05));
        let bla_table = BlaTable::compute(&orbit, 0.15);

        let result_no_bla = compute_pixel_perturbation_floatexp(&orbit, delta_c, 500, TEST_TAU_SQ);
        let result_bla = compute_pixel_perturbation_floatexp_bla(
            &orbit, &bla_table, delta_c, 500, TEST_TAU_SQ
        );

        assert_eq!(result_no_bla.escaped, result_bla.escaped);
        assert_eq!(result_no_bla.iterations, result_bla.iterations);
    }
```

**Step 2: Run all tests**

Run: `cargo test -p fractalwonder-compute`
Expected: All tests pass

**Step 3: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "test(bla): add comprehensive correctness tests for BLA acceleration"
```

---

## Task 8: Final cleanup and full quality checks

**Files:**
- All modified files

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features`
Expected: All tests pass

**Step 4: Final commit if any changes**

```bash
git add -A
git commit -m "chore: format and lint fixes for BLA implementation"
```

---

## Summary

After completing all tasks, you will have:

1. `BlaEntry` struct with `from_orbit_point` and `merge` methods
2. `BlaTable` struct with `compute` (builds binary tree) and `find_valid` (lookup)
3. `compute_pixel_perturbation_floatexp_bla` function that uses BLA acceleration
4. Comprehensive tests proving BLA produces identical results to non-BLA
5. All code formatted and passing Clippy checks
