# Perturbation Theory for Deep Mandelbrot Rendering

> **Research Document** - Comprehensive analysis of perturbation theory for world-class deep zoom rendering.

---

## 1. Mathematical Foundation

### 1.1 The Mandelbrot Iteration

The standard Mandelbrot iteration is:

```
z₀ = 0
zₙ₊₁ = zₙ² + c
```

A point `c` is in the Mandelbrot set if the sequence `{zₙ}` remains bounded (|zₙ| ≤ 2 for all n).

### 1.2 Perturbation Theory Derivation

**Key Insight**: The formula `z → z² + c` is continuous, so nearby points remain nearby under iteration.

Let:
- `C` = reference point (computed at high precision)
- `c` = pixel point = `C + δc` where `δc` is small
- `Zₙ` = reference orbit (the sequence of z values for C)
- `zₙ` = pixel orbit (the sequence of z values for c)

We can write:
```
zₙ = Zₙ + δzₙ
```

where `δzₙ` is the "delta" (perturbation) from the reference orbit.

**Derivation**:
```
zₙ₊₁ = zₙ² + c
     = (Zₙ + δzₙ)² + (C + δc)
     = Zₙ² + 2Zₙδzₙ + δzₙ² + C + δc

Since Zₙ₊₁ = Zₙ² + C:
     = Zₙ₊₁ + 2Zₙδzₙ + δzₙ² + δc

Therefore:
δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc
```

**The Delta Iteration Formula**:
```
δz₀ = 0
δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc
```

### 1.3 Why This Works

1. **Reference orbit `{Zₙ}`**: Computed once at arbitrary precision, stored as f64 (bounded by escape radius ~2)
2. **Delta values `{δzₙ}`**: Computed per-pixel using only f64 arithmetic
3. **Precision gain**: At deep zoom, `δc` is tiny (e.g., 10⁻¹⁰⁰⁰). The delta iteration keeps values small and representable in f64.

**Critical observation**: The term `2Zₙδzₙ` dominates when `δzₙ` is small. The `δzₙ²` term becomes negligible, which is the basis for BLA acceleration.

---

## 2. Glitch Taxonomy

### 2.1 Overview

A "glitch" is a pixel rendered incorrectly due to perturbation math failing. There are **two fundamental glitch types**:

| Type | Cause | Detection | Solution |
|------|-------|-----------|----------|
| Reference Exhaustion | Reference escapes before pixel | `n ≥ reference_escaped` | Use different reference |
| Precision Loss | Delta dynamics diverge from reference | Pauldelbrot criterion | Rebase or different reference |

### 2.2 Type 1: Reference Exhaustion

**Cause**: The reference orbit escapes (|Zₙ| > 2) while the pixel needs more iterations.

**Why it happens**: If the reference point is outside the set or near the boundary, it may escape before pixels that are deeper inside the set.

**Detection**:
```rust
if iteration >= reference_escaped_at {
    mark_as_glitched();
}
```

**Solution**: Use a reference that doesn't escape, or use multiple references with longer orbits.

### 2.3 Type 2: Precision Loss (Pauldelbrot Criterion)

**Cause**: When `|Zₙ + δzₙ|` becomes very small compared to `|Zₙ|`, the perturbation math loses significant digits.

**Mathematical Explanation**:

Pauldelbrot derived this by perturbing the perturbation itself. Consider `δz → δz + ε`:
```
ε → (2(Zₙ + δzₙ) + ε)ε + f
```

The ratio `ε/δz ≈ 2(Zₙ + δzₙ) / 2Zₙ = (Zₙ + δzₙ) / Zₙ`

When this ratio is small, nearby pixels "stick together" - there isn't enough precision to distinguish them.

**The Pauldelbrot Criterion**:
```
IF |Zₙ + δzₙ| < τ × |Zₙ|  THEN  pixel is glitched
```

Where `τ` (tau) is a threshold, typically:
- `τ = 10⁻³` (conservative, catches most glitches)
- `τ = 10⁻⁸` (aggressive, fewer false positives but may miss some)

**Implementation**:
```rust
let z_full = (Z_n.0 + dz.0, Z_n.1 + dz.1);  // Full pixel z
let z_mag_sq = z_full.0 * z_full.0 + z_full.1 * z_full.1;
let Z_mag_sq = Z_n.0 * Z_n.0 + Z_n.1 * Z_n.1;

// τ² comparison (using squared magnitudes for efficiency)
if z_mag_sq < TAU_SQUARED * Z_mag_sq {
    mark_as_glitched();
}
```

**Note**: The value `|Zₙ + δzₙ|²` must be computed anyway for escape testing, so this check is essentially free.

### 2.4 Why Precision Loss Causes Visual Artifacts

When precision is lost:
1. Multiple nearby pixels compute identical δzₙ values (they "stick together")
2. These pixels get the same iteration count
3. Result: Flat "blobs" of solid color where there should be detail
4. These blobs are the visual signature of precision glitches

### 2.5 Choosing the Threshold τ

| τ Value | Behavior |
|---------|----------|
| 10⁻² | Very conservative, many false positives, slow rendering |
| 10⁻³ | Standard, good balance (used by Kalles Fraktaler default) |
| 10⁻⁶ | Moderate, good for most renders |
| 10⁻⁸ | Aggressive, fast but may miss edge cases |

**Kalles Fraktaler** has a "glitch low tolerance" checkbox:
- Disabled (default): Uses higher τ, faster but may miss some glitches
- Enabled: Uses lower τ (10⁻³), catches more glitches but 16x slower in worst case

---

## 3. Rebasing (Zhuoran's Breakthrough, 2021)

### 3.1 The Insight

Instead of using multiple reference orbits to fix glitches, **reset to the beginning of the SAME reference orbit** when precision loss is detected.

**Key condition for rebasing**:
```
IF |Zₘ + δzₙ| < |δzₙ|  THEN  rebase
```

Equivalently: `IF |Z + δz| < |δz|`

This means the full pixel value `z = Z + δz` has smaller magnitude than the delta alone - the delta has "overtaken" the reference.

### 3.2 Rebasing Operation

When rebasing is triggered:
```
δz_new = Z_m + δz_n    // Absorb reference value into delta
m = 0                   // Reset reference iteration counter to 0
// Continue iterating with δz_new and reference orbit from start
```

### 3.3 Why Rebasing Works

1. The pixel orbit and reference orbit are both orbits of the same dynamical system
2. When they diverge, eventually they will "sync up" again (both pass near similar points)
3. By resetting to iteration 0, we find where the pixel orbit aligns with early reference orbit values
4. This avoids precision loss without needing a new reference

### 3.4 Implementation Pseudocode

```rust
fn iterate_with_rebasing(
    reference: &ReferenceOrbit,
    delta_c: Complex,
    max_iter: u32
) -> IterationResult {
    let mut dz = Complex::zero();
    let mut m = 0;  // Reference orbit index

    for n in 0..max_iter {
        let Z_m = reference.orbit[m];
        let z = Z_m + dz;

        // Escape check
        if z.norm_sq() > 4.0 {
            return Escaped(n);
        }

        // Rebase check: |z| < |dz|
        if z.norm_sq() < dz.norm_sq() {
            dz = z;      // Absorb Z into delta
            m = 0;       // Reset reference index
        }

        // Delta iteration
        dz = 2.0 * Z_m * dz + dz * dz + delta_c;
        m += 1;

        // Handle reference exhaustion
        if m >= reference.len() {
            m = 0;  // Wrap around (for non-escaping reference)
        }
    }

    InSet(max_iter)
}
```

### 3.5 Advantages of Rebasing

1. **Single reference orbit**: Only need one reference per critical point (one for Mandelbrot)
2. **Prevents glitches**: Avoids precision loss rather than detecting and correcting afterward
3. **Simpler implementation**: No need for multi-reference management
4. **Parallel-friendly**: Each pixel is independent, no shared state modification

---

## 4. Extended Precision for Delta Values

### 4.1 The Range Problem

Standard f64:
- Mantissa: 53 bits (~16 decimal digits)
- Exponent range: ~10⁻³⁰⁸ to 10³⁰⁸

At deep zoom (e.g., 10⁻¹⁰⁰⁰), delta values underflow to zero in standard f64.

### 4.2 Solutions

There are three approaches to handling deep zoom delta values:

#### Option A: BigFloat (Arbitrary Precision)

Use existing arbitrary precision library for delta arithmetic. **Fractal Wonder already has BigFloat** (using Dashu's FBig).

**Pros**:
- Already implemented in codebase
- Unlimited precision and range
- Simplest to integrate

**Cons**:
- Slower than f64 or FloatExp
- May negate some perturbation performance gains

```rust
// Delta iteration using BigFloat
let dz_new = two.mul(&Z_n).mul(&dz)
    .add(&dz.mul(&dz))
    .add(&delta_c);
```

#### Option B: Rescaling (Keep f64 normalized)

Track a separate scale factor `S` and keep delta values normalized near 1.0:
- Store `δc = S × d` where `|d| ≈ 1`
- Periodically renormalize when values drift

**Pros**:
- Uses fast f64 arithmetic
- No new types needed

**Cons**:
- Complex bookkeeping
- Must handle scale factor in all operations

```rust
// Rescaled iteration
// w = dz/S, d = dc/S
// dz' = 2*Z*dz + dz² + dc
// w' = 2*Z*w + S*w² + d  (note: S factor on squared term)
```

#### Option C: FloatExp (Extended Exponent)

**FloatExp** = Float with extended exponent (used by Kalles Fraktaler):
- Mantissa: f64 (52 bits precision), normalized near 1.0
- Exponent: separate integer (supports huge ranges)

**Pros**:
- Faster than BigFloat
- Simpler than rescaling
- Standard approach in fractal renderers

**Cons**:
- Requires implementing new type
- Still limited precision (52 bits)

```rust
struct FloatExp {
    mantissa: f64,  // Normalized: 0.5 ≤ |mantissa| < 1.0 or mantissa == 0
    exp: i64,       // Exponent (base 2)
}

// Value = mantissa × 2^exp
```

### 4.3 Recommendation for Fractal Wonder

**Start with BigFloat for deltas** since it's already available. Profile performance at various zoom depths:

| Zoom Depth | Recommended Approach |
|------------|---------------------|
| < 10³⁰⁰ | f64 (fast, sufficient range) |
| 10³⁰⁰ - 10²⁰⁰⁰ | BigFloat or FloatExp |
| > 10²⁰⁰⁰ | BigFloat (guaranteed correctness) |

If BigFloat proves too slow for interactive use, implement FloatExp as an optimization.

---

## 5. Series Approximation and BLA

### 5.1 Series Approximation (Traditional)

The delta iteration generates a polynomial series in δc:
```
δzₙ = Aₙδc + Bₙδc² + Cₙδc³ + O(δc⁴)
```

Coefficients `Aₙ, Bₙ, Cₙ, ...` can be computed once and reused for all pixels.

**Limitation**: Accuracy degrades after many iterations; requires "probe points" to validate.

### 5.2 BLA: Bivariate Linear Approximation (Zhuoran, 2021)

**Key insight**: When `δzₙ²` is negligible compared to `2Zₙδzₙ`, the iteration becomes linear:
```
δzₙ₊₁ ≈ 2Zₙδzₙ + δc
```

This linear form allows "skipping" multiple iterations at once.

### 5.3 BLA Mathematics

**Single-iteration BLA**:
```
δzₙ₊₁ = Aδzₙ + Bδc    where A = 2Zₙ, B = 1
```

**Multi-iteration BLA** (skipping `l` iterations):
```
δzₘ₊ₗ = Aₗδzₘ + Bₗδc
```

**Validity condition**: The approximation is valid when:
```
|δzₙ²| < ε|2Zₙδzₙ|
```

Where `ε` is hardware precision (~2×10⁻⁵³ for f64).

**Validity radius**:
```
r = ε|Zₙ| - |B||δc|/|A|
```

BLA can be applied when `|δzₘ| < r`.

### 5.4 BLA Merging (Binary Tree)

Adjacent BLAs can be merged:
```
(A_y∘x, B_y∘x) = (A_y × A_x, A_y × B_x + B_y)
r_y∘x = min(r_x, max(0, (r_y - |B_x|×|δc|) / |A_x|))
```

**Algorithm**:
1. Create M single-iteration BLAs
2. Merge neighbors: M → M/2 → M/4 → ... → 1
3. Result: Binary tree of BLAs at different skip lengths

### 5.5 BLA vs Series Approximation

| Aspect | Series Approximation | BLA |
|--------|---------------------|-----|
| Conceptual complexity | Higher (polynomial coefficients) | Simpler (linear) |
| Implementation | Harder | Easier |
| Parallelization | Difficult | Easy |
| Error bounds | Ad-hoc probe validation | Mathematical validity radius |
| Generality | Mandelbrot-specific | Works for Burning Ship, hybrids |
| Performance | 1.7x slower than BLA | Fastest |

**BLA is the preferred modern approach**.

---

## 6. Multi-Reference Strategies

### 6.1 When Multi-Reference is Needed

Even with rebasing, some scenarios benefit from multiple references:
- Reference escapes too early (all pixels would need constant rebasing)
- Hybrid formulas with multiple critical points
- Extremely heterogeneous iteration counts across the image

### 6.2 Reference Selection Strategies

**Strategy 1: Iterative Refinement**
1. Compute image with center as reference
2. Detect glitched pixels (via Pauldelbrot criterion)
3. Select new reference in glitched region
4. Re-render only glitched pixels
5. Repeat until glitch-free

**Strategy 2: Periodic Points**
- Find minibrots (periodic nuclei) via Newton's method
- Use these as references (they never escape)
- Higher-period minibrots cause fewer glitches

**Strategy 3: Spatial Partitioning**
- Divide image into regions
- Assign references per region based on dynamics
- Kalles Fraktaler uses up to 10,000 references

### 6.3 Kalles Fraktaler's Approach

1. Start with center reference
2. Auto-detect glitches during render
3. For each glitch set (pixels glitched at same iteration):
   - Select one pixel as new reference
   - Re-render only that glitch set
4. Iterate until no glitches remain
5. "Glitch low tolerance" flag for aggressive detection

---

## 7. Algorithm Specification

### 7.1 Complete Perturbation Algorithm with Rebasing

```rust
struct PerturbationRenderer {
    reference: ReferenceOrbit,
    tau_sq: f64,  // Glitch detection threshold squared
}

impl PerturbationRenderer {
    fn compute_pixel(&self, delta_c: Complex) -> PixelResult {
        let mut dz = Complex::zero();
        let mut m: usize = 0;  // Reference orbit index
        let mut glitched = false;

        for n in 0..self.max_iter {
            // Get reference value (with wraparound for non-escaping reference)
            let Z_m = if m < self.reference.orbit.len() {
                self.reference.orbit[m]
            } else {
                // Reference exhausted without escaping = wrap around
                m = 0;
                self.reference.orbit[0]
            };

            // Full pixel value
            let z = Z_m + dz;
            let z_mag_sq = z.norm_sq();
            let Z_mag_sq = Z_m.norm_sq();
            let dz_mag_sq = dz.norm_sq();

            // 1. Escape check
            if z_mag_sq > 4.0 {
                return PixelResult::Escaped {
                    iterations: n,
                    glitched
                };
            }

            // 2. Glitch detection (Pauldelbrot criterion)
            // |z| < τ|Z| indicates precision loss
            if Z_mag_sq > 1e-20 && z_mag_sq < self.tau_sq * Z_mag_sq {
                glitched = true;
            }

            // 3. Rebase check: |z| < |dz|
            if z_mag_sq < dz_mag_sq {
                dz = z;   // Absorb Z into delta
                m = 0;    // Reset to beginning of reference
                continue; // Skip this iteration (already have new dz)
            }

            // 4. Delta iteration: dz' = 2*Z*dz + dz² + dc
            dz = 2.0 * Z_m * dz + dz * dz + delta_c;
            m += 1;
        }

        PixelResult::InSet {
            iterations: self.max_iter,
            glitched
        }
    }
}
```

### 7.2 Reference Orbit Computation

```rust
fn compute_reference_orbit(
    c: &BigComplex,
    max_iter: u32,
    precision: u32
) -> ReferenceOrbit {
    let mut z = BigComplex::zero(precision);
    let mut orbit = Vec::with_capacity(max_iter as usize);
    let escape_radius_sq = BigFloat::from(4.0);

    for n in 0..max_iter {
        // Store as f64 (orbit values bounded by ~2)
        orbit.push(z.to_f64_complex());

        // Escape check
        if z.norm_sq() > escape_radius_sq {
            return ReferenceOrbit {
                orbit,
                escaped_at: Some(n),
                c_ref: c.to_f64_complex(),
            };
        }

        // Iterate: z = z² + c
        z = z * z + c;
    }

    ReferenceOrbit {
        orbit,
        escaped_at: None,
        c_ref: c.to_f64_complex(),
    }
}
```

---

## 8. Test Cases

### 8.1 Mathematical Test Cases

**Test 1: Delta iteration matches direct computation (shallow zoom)**
```rust
#[test]
fn perturbation_matches_direct_for_shallow_zoom() {
    let c_ref = BigComplex::new(-0.5, 0.0, 128);
    let orbit = compute_reference_orbit(&c_ref, 1000, 128);

    // Test multiple delta values
    for delta in [
        (0.01, 0.01),
        (-0.005, 0.002),
        (0.0, 0.001),
    ] {
        let perturb_result = compute_pixel_perturbation(&orbit, delta, 1000);
        let c_pixel = c_ref + BigComplex::from(delta);
        let direct_result = compute_direct(&c_pixel, 1000);

        assert_eq!(perturb_result.escaped, direct_result.escaped);
        assert!((perturb_result.iterations - direct_result.iterations).abs() <= 1);
    }
}
```

**Test 2: Pauldelbrot criterion detects known glitch locations**
```rust
#[test]
fn pauldelbrot_detects_glitch_at_minibrot_boundary() {
    // Reference at main cardioid center
    let c_ref = BigComplex::new(-0.5, 0.0, 128);
    let orbit = compute_reference_orbit(&c_ref, 10000, 128);

    // Pixel at period-3 minibrot (very different dynamics)
    let delta = (-0.622 - (-0.5), 0.0);  // Move to ~(-1.122, 0)

    let result = compute_pixel_perturbation(&orbit, delta, 10000);

    // Should be detected as glitched (dynamics differ from reference)
    assert!(result.glitched,
        "Pixel at minibrot should be detected as glitched");
}
```

**Test 3: Rebasing prevents glitch for reuniting orbits**
```rust
#[test]
fn rebasing_prevents_precision_loss() {
    let c_ref = BigComplex::new(-0.75, 0.1, 256);
    let orbit = compute_reference_orbit(&c_ref, 5000, 256);

    // Delta that causes orbits to diverge then reunite
    let delta = (0.001, -0.001);

    // With rebasing: should not be glitched
    let result = compute_with_rebasing(&orbit, delta, 5000);

    // Verify against high-precision direct computation
    let c_pixel = c_ref + BigComplex::from(delta);
    let direct = compute_direct(&c_pixel, 5000);

    assert_eq!(result.escaped, direct.escaped);
    assert!(!result.glitched, "Rebasing should prevent glitch");
}
```

**Test 4: |z| < |dz| triggers rebase**
```rust
#[test]
fn rebase_triggered_when_delta_exceeds_reference() {
    let c_ref = BigComplex::new(-0.5, 0.0, 128);
    let orbit = compute_reference_orbit(&c_ref, 100, 128);

    // Manually simulate iteration with tracking
    let delta_c = (0.1, 0.1);
    let (result, rebase_count) = compute_with_rebase_counting(&orbit, delta_c, 100);

    // For this delta, rebasing should occur
    assert!(rebase_count > 0,
        "Rebasing should trigger for divergent delta");
}
```

**Test 5: No glitch for pixels escaping before reference**
```rust
#[test]
fn no_glitch_when_pixel_escapes_before_reference() {
    // Reference deep in set (never escapes)
    let c_ref = BigComplex::new(-0.5, 0.0, 128);
    let orbit = compute_reference_orbit(&c_ref, 10000, 128);
    assert!(orbit.escaped_at.is_none());

    // Pixel that escapes quickly
    let delta = (2.5, 0.0);  // Pixel at (2.0, 0.0)
    let result = compute_pixel_perturbation(&orbit, delta, 10000);

    assert!(result.escaped);
    assert!(result.iterations < 10);
    assert!(!result.glitched,
        "Pixel escaping with valid reference data should not be glitched");
}
```

### 8.2 Visual Validation Test Cases

| Zoom Depth | Location | Expected Behavior |
|------------|----------|-------------------|
| 10¹⁴ | (-0.75, 0.1) | All glitches detected, cyan overlay matches artifacts |
| 10⁵⁰ | Elephant valley | Multi-reference resolves all glitches |
| 10¹⁰⁰ | Deep minibrot | Extended precision (BigFloat) required, no visual artifacts |
| 10⁵⁰⁰ | Near Feigenbaum point | BLA acceleration functional |

---

## 9. Analysis of Current Implementation

### 9.1 What's Correct

1. ✅ Basic perturbation iteration formula: `dz' = 2*Z*dz + dz² + dc`
2. ✅ Reference orbit computation with BigFloat
3. ✅ Reference exhaustion detection
4. ✅ Basic rebasing trigger

### 9.2 What's Missing/Incorrect

1. ❌ **Pauldelbrot criterion not implemented**: Only detecting reference exhaustion, missing precision loss detection
   - Current: `if n >= reference_escaped`
   - Needed: `if |Z + dz|² < τ² × |Z|²`

2. ❌ **Incorrect rebasing**: Current implementation switches to "on-the-fly" f64 computation
   - Current: Computes `z = z² + c` in f64 after rebase
   - Needed: Reset to iteration 0 of same reference orbit with `dz_new = Z + dz`

3. ❌ **No extended precision for deep zoom deltas**: Deltas underflow to 0 beyond 10³⁰⁰
   - Needed: Use BigFloat (already available) or implement FloatExp for delta values

4. ❌ **No BLA acceleration**: Every iteration computed individually
   - Needed: BLA table for skipping iterations

5. ❌ **Threshold not configurable**: No way to tune glitch detection sensitivity

### 9.3 Why Glitches Go Undetected

At 10¹⁶ zoom with 26/342 tiles marked glitched:
- Reference exhaustion catches some glitches
- Precision loss glitches are **invisible** to current detection
- Visual artifacts appear where Pauldelbrot criterion would trigger but isn't checked

---

## 10. References

### Primary Sources

1. **K.I. Martin** - "SuperFractalThing" and sft_maths.pdf (2013)
   - Original perturbation theory popularization
   - https://superfractalthing.co.nf/sft_maths.pdf

2. **Claude Heiland-Allen (mathr)** - Deep Zoom Theory and Practice
   - https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html
   - https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html

3. **Pauldelbrot** - Glitch Detection Criterion (2014)
   - Fractal Forums post establishing |z| < τ|Z| criterion
   - Referenced in Kalles Fraktaler manual

4. **Zhuoran** - Rebasing and BLA (2021)
   - Fractal Forums contributions
   - Implemented in Imagina renderer

### Software References

5. **Kalles Fraktaler 2+**
   - https://mathr.co.uk/kf/kf.html
   - Manual: https://mathr.co.uk/kf/manual.html
   - Source: `git clone https://code.mathr.co.uk/kalles-fraktaler-2.git`

6. **rust-fractal-core**
   - Rust implementation with perturbation/SA
   - https://github.com/rust-fractal/rust-fractal-core

7. **DeepDrill**
   - Modern C++ implementation
   - https://dirkwhoffmann.github.io/DeepDrill/

### Additional Resources

8. **Phil Thompson's Blog**
   - Perturbation: https://philthompson.me/2022/Perturbation-Theory-and-the-Mandelbrot-set.html
   - BLA: https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html

9. **Wikipedia** - Plotting algorithms for the Mandelbrot set
   - General reference for perturbation section

10. **Mathematics Stack Exchange**
    - https://math.stackexchange.com/questions/3083263/mandelbrot-set-perturbation-theory-when-do-i-use-it
    - https://math.stackexchange.com/questions/2552605/selecting-reference-orbit-for-fractal-rendering-with-perturbation-theory

---

## 11. Implementation Roadmap

### Phase 1: Fix Glitch Detection
1. Add Pauldelbrot criterion (`|z|² < τ²|Z|²`)
2. Make threshold configurable
3. Verify cyan overlay matches visible artifacts at 10¹⁴-10¹⁶

### Phase 2: Fix Rebasing
1. Replace on-the-fly computation with true rebasing
2. Rebase to iteration 0 with `dz_new = Z + dz`
3. Test at boundary regions where rebasing matters

### Phase 3: Extended Precision
1. Use BigFloat for delta values at deep zoom (already available in codebase)
2. Validate at 10⁵⁰-10¹⁰⁰
3. If performance is insufficient, consider implementing FloatExp as optimization

### Phase 4: BLA Acceleration
1. Implement single-iteration BLA
2. Build BLA table with binary merging
3. Integrate with iteration loop
4. Benchmark performance gains

### Phase 5: Multi-Reference (if needed)
1. Track glitch locations
2. Select references in glitched regions
3. Re-render only affected pixels
4. Iterate until glitch-free

---

*Document created: November 2024*
*Based on research from Fractal Forums, mathr.co.uk, and academic sources*
