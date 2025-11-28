# BLA Acceleration Design

> Increment 4 from `docs/research/perturbation-theory.md`

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
