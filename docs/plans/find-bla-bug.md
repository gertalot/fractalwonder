# BLA Deep Zoom Bug: Diagnosis Session

**STATUS: ✅ FIXED** - See [ROOT CAUSE ANALYSIS AND FIX](#root-cause-analysis-and-fix) section.

## Context

We are building a high-performance browser-based Mandelbrot explorer in Rust/WASM targeting world-record zoom depths
(10^250 to 10^1000+). The implementation uses:

- Reference orbit: Computed with arbitrary-precision BigFloat
- Pixel iteration: Perturbation theory with HDRFloat (f32 head + f32 tail + i32 exponent, ~48-bit mantissa)
- BLA (Bivariate Linear Approximation): Hierarchical table to skip iterations

## The Bug

When we enable multi-level BLA (allowing skip counts > 1), only the center tile renders as a uniform color. All other
tiles render correctly.

NOTE: We are ONLY rendering the center 4 tiles to make debugging faster and easier.

Given this URL: <http://127.0.0.1:8080/fractalwonder/#v1:7ZvLbhvZFUX_hWMh2CcIMtA8XxEEhCyVbQJqyaBodxpG_3soiXXvWqfoadADlV9iFe_rPPbZZxf8c_fjsPz-7fl42t3-3N0vT6fluLv998_dj7vH78vudpe_pZLznzpfrz_U28-vd94evN16ffT2J1lvvX_nMuz91_vjy_jLo8u4y5fXdd4_XpZ7XyE1Zl2Hr1_BBufn9Z91A2Nnucxd686K84-nqTHP-jfmX9e8zJQ55dh_u5d1-rHoOknNm-tpxwnHynM1HDjDOGMfq2Hn8plng3PGbnj-y94LFh3TVabBa3hznqV5cXUXxhXHa_rIP5kGjJ7PZddHbe14IzPiZugMC03zjBkZdGMMYm41T8LQDCzMJabD56amaddtzU3PcJ1_zUiEpUaIzTAb0TgchxRDLGtfY7V5jJqzFzzYDL9-GfkyP4dbnNcIoBllzqyWqA7iYSv7tpCd3DLinhE3g3hk0Qzk6U3Fk2ZiwOIBgnmaf2wLGT7PMjeRbNw5jodErdpMYHOP78PmTmn6ITz6BqNHnGX-DhPcR0xfeyJWbXK1A-Xc3ghlmEsAVATIHjoTFQogMjZFJ7DMCHxrumZWiFnYEG5EyRAzkNM4x8QKOXdueeZYIcDC-AHgK6ILnqKj54ojX2PPVBgNV-opQs-gn2wqOYsiFp5rJzQ4Xaw9F-rnXBsVTBNEsDQZgDBfqTUHIlUrpBMjV42PDYkKSTELCrFLbGR3s_t2XO4PL4fnp_2nw-lld1v5-z__vBHV4mIon7QgaAvcq0hSPSiWGiMSqUpgTjEW1kGFhKhPULZIYbip6VCgNYahipYZmtgRjj49TChymmZLmbhUj63iHpMtjJEtcYhzOmRnBqKOU4AoU9_o8GA2rkDyX7lS6FNKgRsyF5WVIg6X8Bs4AAJZHeoVb1UuKKgqAH9wkogtC0YKlJeEDRi8hgopAKMLeUScwbQl4FGsgLyonJDCMUGYzjPVel2N-HxE3kX6AkwAJQr9BNwk7ovksxg5dJTHwEv0LM5E94XVaOpMFECrylJMEqdrczVzOiQxMgTFpKfshXhNEKpNGaZh0TJiUPUzmRQnG_RSp1YoySTkrHNB98GZCf-iOxFoIfVJ1Vjy5lkUKHFtLxMkNZTEHe0xNiAo7YhCdBvVGzxXMedfymmu2GYXnTK2kCu7FTNnUx8pdl9CrojzzA5CVmhhi0ZVbSqZDoG1talpZy3VSeF9V1HIsLsYgBwNGXd3Qal9VJiwjSpx0wYrxYZAaIZS4KawYRPpaIkLjTP_inH952b3--Hh9PVV8NpQr4_r4_q4_v8XGIs6vXQFZKseEgZIW6800q0LALdIldlw2N-ijKtBv1Y2B1xFPVs2uJVOwPvKaF2qrhaqhEpZrJ-JpVQjC9I4pwswxlperOym0e2gVFgJLPbNG9gm9S92GShQwU1OFStntCyKREDR5UCSRRBBFbVIyBbHpohY1jtIlCRNwktdvi3Lq-j91emoUbna-gQ6hNUrTLqNXuxA6gd4uZhrQnoq4YH8Xl1Qy2OfTjKajc9zMrApC0jCqw1XpqrgiaF-paIWbXZ2lCyakTlN0NFNohuQslBMc0vfM13sEtmA6JwxDzeWE3VzrKjvc1sV5q5VjtnoILOcDtQUShhFS0XCztWs64Zs2EAFLJPyFXdZxjdzZXZAaXFUkJnZIkXNmDK35UGlyWqMcYeDwFAqF-CIrpc-wyYaIIGOOhRtFWzlJi1ECgJIkcIjIbbZAJSVEa-ISY5oMHwVJOBOSsipwC8Wy6j57hU1KqnxB9exksxScbcHZsEE9JE7SAIei5lqKSxNuACDoGTgfvr1h18KzLuvy-HL19NHu_NxfVx_kUuYn2oMIaK5FTVFgggJdARxKo2XcUTVSAcPkVZvDSj5BFzfGnlYYsJqLV1q221RV-sSJu4SWi0C85VGJGNRB2RfwF7Q7WU0EujcpMlqnIwUuR2EdkgrZ2pHzXCTbV1DgAxDmJPPromVnlSx0QG_gVEZr67WSonU2whKqBZ3m3rvoK5qrZLf-og9t6alWtyKkldJmFYPfvW9EPworXuchyJCrFKSd7DnSSk61HEzuRFom5AwM-U6DvAyK6YgjQOYKzElZDG38RZcW2wiSa2O00GR4ovgKGVc48-hkBJBTMcmm0LItFV1TGjlTuVDGlIWFJ2uuqShoN7OpK3JA1uC32x3240n8r2lHrc3yk1q-UhFYZEiABnt2hJigdNRin2qty96_eHXE8p6n4lpGb16sZazef0h7bA2yWqSHUuJZOm9PPbKY9VlU9qdl0Ar1Ev6pqkjrRJtuusCkqlMh9XI4iUQFXpDkzdnHQ-xMDJ2aDCIA5JM1TG2rs0mN360BJvgJcAoYbwKOwXDKkorktuk_mz-tXaJEgoxMSVJRnW0FUHKANBSVDEtRZVObVSIjCLJq4oiXMd2eMIEIQS4WYXCPj8sBFS91OFH3LJpJmK9aYWqGIyVxlHAbBquM3iVNFaqoHu5eMkn80w8SKQVXNHgcoVFm7wgl8XQwCqRdwz2WDto-FZQZUgByOU2-caA3zIC2Rngvm7oV3LDWW-4f376fPiyPzyclYbf7p4elsdPx-fT64C7x-V0WvZPd7-9qhD_elzuT8fD_fnJcTl_7bh_OT8-PH15efu_CH_cPy77--fvT2fpom5231-W_Zdv33e3n-8eX5ab3X-Pd3_sl6e7T4_Lw7j5-fl4v-y_Phz3nx-f706X--c9_ViOrzvd3f7jz_8B>

the render produces the following logs:

```
[WorkerPool] Creating 8 workers
[WorkerPool] Recreating 8 workers
[WorkerPool] Creating 8 workers
[Precision] bits_from_ratio=9, iter_bits=14, safety_bits=16, total=39, result=64
[Precision] bits_from_ratio=905, iter_bits=14, safety_bits=106, total=1025, result=1025
DEBUG: Rendering only 4 center tiles for BLA debugging
Using CPU renderer (zoom=1.09e270, force_hdr=false)
[WorkerPool] Starting perturbation render #1 with 4 tiles, zoom=10^270.0, max_iter=10000000
[WorkerPool] No workers initialized yet, queueing orbit request
[Worker] Started
[Worker] Started
[WorkerPool] First worker ready, dispatching queued orbit request
[Worker] Started
[Worker] Started
[Worker] Started
[Worker] Started
[Worker] Started
[Worker] Started
[Worker] Reference orbit computed: 30302 iterations in 3844ms, escaped_at=Some(30301)
[WorkerPool] Reference orbit complete: 30302 points, escaped_at=Some(30301)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[Worker] Built BLA table: 60608 entries, 16 levels (dc_max: head=5.62e-1, exp=-895)
[WorkerPool] All 8 workers have orbit, dispatching 4 tiles
[WorkerPool] Tile (384,224): 0/1024 glitched, 99.7% BLA (30940148/31028224)
[WorkerPool] Tile (384,192): 0/1024 glitched, 0.0% BLA (0/30999103)
[WorkerPool] Tile (352,224): 0/1024 glitched, 0.0% BLA (0/30994931)
[WorkerPool] Tile (352,192): 0/1024 glitched, 0.0% BLA (0/31042749)
[WorkerPool] Render complete: 0 tiles had glitches (of 4 total)
```

The logs show that 0.0% of iterations are skipped in three of the tiles, while 99.7% of iterations are skipped in one
tile. This is PROBABLY the problematic tile that renders in a uniform color (needs proof).

## Our BLA Implementation

BLA Entry Structure (fractalwonder-compute/src/bla.rs)

```rs
pub struct BlaEntry {
    pub a: HDRComplex,     // Coefficient A (multiplies δz)
    pub b: HDRComplex,     // Coefficient B (multiplies δc)
    pub l: u32,            // Iterations to skip
    pub r_sq: HDRFloat,    // Validity radius squared
}
```

Level-1 BLA Creation (from reference orbit point Z)

```rs
pub fn from_orbit_point(z_re: f64, z_im: f64) -> Self {
    let epsilon = 2.0_f64.powi(-53);
    let z_mag = (z_re * z_re + z_im * z_im).sqrt();
    let r = epsilon * z_mag;

    Self {
        a: HDRComplex { re: HDRFloat::from_f64(2.0 * z_re), im: HDRFloat::from_f64(2.0 * z_im) },
        b: HDRComplex { re: HDRFloat::from_f64(1.0), im: HDRFloat::ZERO },
        l: 1,
        r_sq: HDRFloat::from_f64(r * r),  // Stores r², not r
    }
}
```

BLA Merging (combining two BLAs into one that skips more)

```rs
pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: &HDRFloat) -> BlaEntry {
    let a = y.a.mul(&x.a);                    // A_merged = A_y × A_x
    let b = y.a.mul(&x.b).add(&y.b);          // B_merged = A_y × B_x + B_y

    let r_x = x.r_sq.sqrt();
    let r_y = y.r_sq.sqrt();
    let b_x_mag = x.b.norm_hdr();
    let a_x_mag = x.a.norm_hdr();

    // r_merged = min(r_x, max(0, (r_y - |B_x|·dc_max) / |A_x|))
    let b_dc = b_x_mag.mul(dc_max);
    let r_adjusted_num = r_y.sub(&b_dc);
    let r_adjusted = if r_adjusted_num.is_negative() || r_adjusted_num.is_zero() {
        HDRFloat::ZERO
    } else if a_x_mag.is_zero() {
        HDRFloat::ZERO
    } else {
        r_adjusted_num.div(&a_x_mag)
    };
    let r = r_x.min(&r_adjusted);

    BlaEntry { a, b, l: x.l + y.l, r_sq: r.square() }
}
```

BLA Search (find_valid function) - **FIXED VERSION**

```rs
pub fn find_valid(&self, m: usize, dz_mag_sq: &HDRFloat, dc_max: &HDRFloat) -> Option<&BlaEntry> {
    let max_b_dc_exp = 0;  // Threshold: |B × dc_max| < 2^0 = 1

    for level in (0..=self.num_levels.saturating_sub(1)).rev() {  // Highest level first
        let level_start = self.level_offsets[level];
        let skip_size = 1usize << level;

        // BUG FIX: Only use higher-level BLA when m is aligned to skip_size.
        // Level n entry i was built from orbit points Z_{i*2^n} to Z_{i*2^n + 2^n - 1}.
        // These coefficients are only valid when applied starting at m = i * 2^n.
        if !m.is_multiple_of(skip_size) {
            continue;
        }

        let idx_in_level = m / skip_size;
        let entry_idx = level_start + idx_in_level;

        // Bounds check...
        let entry = &self.entries[entry_idx];

        // Validity check: |δz|² < r²
        let diff = dz_mag_sq.sub(&entry.r_sq);
        let validity_check = diff.is_negative();

        if validity_check {
            // B coefficient check
            let b_norm = entry.b.norm_hdr();
            let b_dc = b_norm.mul(dc_max);
            if b_dc.exp <= max_b_dc_exp {
                return Some(entry);
            }
        }
    }
    None
}
```

BLA Application (in iteration loop)

```rs
if let Some(bla) = bla_entry {
    // Apply BLA: δz_new = A·δz + B·δc
    let a_dz = bla.a.mul(&dz);
    let b_dc = bla.b.mul(&delta_c);
    dz = a_dz.add(&b_dc);
    // Skip bla.l iterations
}
```

## How Professional Renderers Differ

Based on research of Kalles Fraktaler, Fraktaler-3, rust-fractal, and Phil Thompson's article:

| Aspect                  | Professional Renderers            | Our Implementation                               |
|-------------------------|-----------------------------------|--------------------------------------------------|
| Validity comparison     | |δz| < r (linear)                 | |δz|² < r² (quadratic)                           |
| Validity radius storage | Store r directly                  | Store r²                                         |
| Precision format        | FloatExp (f64 mantissa + i32 exp) | HDRFloat (~48-bit mantissa via f32+f32)          |
| Validity formula        | r = ε × |Z| - |B| × |δc| / |A|    | r = ε × |Z| (simpler, but merge adds adjustment) |

Key Reference: Phil Thompson's BLA Article

From <https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html>:

> The validity radius formula: r = ε × |Z_n| - |B| × |δc| / |A|
> BLA is valid when: |δz| < r

## Observed Symptoms

1. **99.7% BLA usage in center tile**: High BLA usage but incorrect rendering
2. **Uniform center tile**: Only the center tile (containing the reference point) shows the bug
3. **Other tiles correct**: Non-center tiles render correctly (with 0% BLA due to rebasing)

## Hypotheses

### Hypothesis 1: Validity Check Always Fails

The check dz_mag_sq.sub(&entry.r_sq).is_negative() may always return false because:
- r_sq values are extremely small (ε² × |Z|² ≈ 10^-32 at surface)
- At deep zoom, dz_mag_sq starts near zero but may not compare correctly due to HDRFloat precision issues

### Hypothesis 2: B-Coefficient Check Too Strict

The check b_dc.exp <= 0 (meaning |B × dc_max| < 1) may reject all high-level BLAs because:
- B coefficients grow exponentially with level (B_merged = A_y × B_x + B_y)
- At deep zoom, even small dc_max values multiplied by large B may exceed threshold

### Hypothesis 3: Index Calculation Bug ✓ **CORRECT**

The index calculation `m / skip_size` was wrong when `m` was not aligned to `skip_size`, causing
lookup of BLA entries built from different orbit points than the current reference index.

### Hypothesis 4: HDRFloat Arithmetic Bug

Subtraction or comparison of HDRFloat values with vastly different exponents may have precision issues.

## Debugging Strategy

### Step 1: Instrument find_valid() with detailed logging

Add logging to trace:

```rs
log::debug!("find_valid: m={}, dz_mag_sq=(head={:.2e}, exp={})", m, dz_mag_sq.head, dz_mag_sq.exp);

for level in (0..self.num_levels).rev() {
    let entry = &self.entries[entry_idx];
    log::debug!(
        "  Level {}: skip={}, r_sq=(head={:.2e}, exp={}), validity={}",
        level, skip_size, entry.r_sq.head, entry.r_sq.exp, validity_check
    );

    if validity_check {
        let b_dc = b_norm.mul(dc_max);
        log::debug!("    B check: b_dc=(head={:.2e}, exp={}), pass={}",
            b_dc.head, b_dc.exp, b_dc.exp <= max_b_dc_exp);
    }
}
```

NOTE: USE CHROME DEVTOOLS MCP TO SEE THE RENDER AND WEB CONSOLE LOGS.

### Step 2: Add a test case for a specific pixel

Create a unit test that:
1. Uses a known reference orbit (e.g., center = -0.5 + 0i)
2. Computes BLA table
3. Calls find_valid() with known δz values
4. Verifies BLA is found and applied correctly

### Step 3: Compare single pixel with/without BLA

Trace one pixel's iteration path:
- Without BLA: record (n, δz) at each iteration
- With BLA: record (n, δz, bla_skip) at each iteration
- Compare where they diverge

### Step 4: Verify HDRFloat comparison

Test that a.sub(&b).is_negative() works correctly when:
- a and b have same exponent
- a and b have vastly different exponents (e.g., 10^-500 vs 10^-32)

## Files to Read

1. fractalwonder-compute/src/bla.rs - BLA table construction and search
2. fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs - HDR iteration with BLA
3. fractalwonder-core/src/hdrfloat.rs - HDRFloat arithmetic
4. fractalwonder-core/src/hdrcomplex.rs - HDRComplex operations
5. fractalwonder-compute/src/perturbation/tile.rs - Tile rendering dispatch

## Current Debug State

- Only 4 center tiles are rendered (filter in parallel_renderer.rs for debugging)
- **BUG FIXED**: Alignment check added to `find_valid()` in `bla.rs:196-202`

## Goal

~~Find and fix the root cause of why find_valid() returns None for all pixels, preventing any BLA acceleration.~~

**COMPLETED**: Root cause identified (alignment bug) and fixed. See ROOT CAUSE ANALYSIS AND FIX section below.

---

# ROOT CAUSE ANALYSIS AND FIX

## Previous Hypothesis (WRONG)

The previous hypothesis was that `find_valid()` didn't check orbit bounds. This was NOT the root cause -
implementing that check did not fix the bug.

## Actual Root Cause: BLA Alignment Bug

**Root Cause**: `find_valid()` used BLA entries at misaligned reference indices.

### The Bug

In `find_valid()`, higher-level BLA entries were selected using:
```rust
let idx_in_level = m / skip_size;  // e.g., m=5, skip_size=2 → idx=2
```

At level 1 (skip=2), entry at index 2 was built from orbit points Z_4 and Z_5.
This entry is ONLY valid when applied starting at m=4, not m=5!

When m=5, the code selected entry 2 and applied BLA coefficients that were computed assuming
the iteration starts at Z_4. But the actual current orbit point is Z_5. This mismatch causes:
- Wrong transformation applied to δz
- All pixels converge to similar values
- Uniform color in the affected tile

### Why Only Center Tile Was Affected

| Tile | BLA Usage | Rendering | Explanation |
|------|-----------|-----------|-------------|
| Center (384,224) | 99.7% | Uniform color (BUG) | Small δc → δz stays small → no rebase → misaligned higher-level BLAs used → wrong coefficients |
| Other tiles | 0% | Correct | Larger δc → rebase triggers → m resets to 0 → BLA invalid at m=0 (Z₀=0 means r=0) |

The center tile never rebases because δz remains small relative to z. This allows the iteration
to use higher-level BLAs at arbitrary (misaligned) m values, triggering the bug.

## The Fix

Added alignment check to `find_valid()` in `bla.rs:196-202`:

```rust
// BUG FIX: Only use higher-level BLA when m is aligned to skip_size.
// Level n entry i was built from orbit points Z_{i*2^n} to Z_{i*2^n + 2^n - 1}.
// These coefficients are only valid when applied starting at m = i * 2^n.
// If m is not aligned, the BLA would use wrong orbit points.
if !m.is_multiple_of(skip_size) {
    continue;
}
```

This ensures:
- Level 0 (skip=1): Always usable (m % 1 == 0)
- Level 1 (skip=2): Only when m is even
- Level 2 (skip=4): Only when m % 4 == 0
- Level n (skip=2^n): Only when m % 2^n == 0

## Test Results

Before fix (all BLA levels enabled):
```
Tile (384,224): 99.7% BLA → uniform color (BUG)
```

With fix (alignment check):
```
Tile (384,224): 97.7% BLA → correct fractal detail ✓
```

All 158 tests pass. The fix ensures BLA entries are only used at correctly aligned reference
indices, preventing coefficient mismatch errors while still providing ~98% BLA acceleration.