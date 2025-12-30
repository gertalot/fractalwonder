# Research Task: Deep Zoom Mandelbrot Rendering Algorithms (10^250 - 10^1000)

  ## Context

  We are building a high-performance browser-based Mandelbrot explorer targeting world-record zoom depths (10^250 to 10^1000+). The implementation uses Rust compiled to WebAssembly with Leptos.

  Our current architecture:
  - **Reference orbit computation**: Uses arbitrary-precision BigFloat
  - **Pixel iteration**: Uses perturbation theory with either f64 or HDRFloat (high dynamic range float with extended exponent)
  - **BLA (Bilinear Approximation)**: Implemented but only works with HDRFloat path, which is too slow

  ## The Problem

  At 10^270 zoom:
  - Pixel deltas (δc) are ~10^-270, within f64 range
  - But δz² values reach ~10^-540, far below f64 minimum (~10^-308)
  - BLA validity checks require comparing |δz|² against r², which underflows in f64
  - Our f64 path is fast but has no BLA (0% iteration skipping)
  - Our HDRFloat path has BLA but is unacceptably slow
  - Professional renderers (Kalles Fraktaler, Fraktaler-3, Imagina) handle this efficiently

  ## Research Objectives

  ### 1. Number Representation at Extreme Depths

  Research how professional renderers represent numbers at 10^250-10^1000 zoom:
  - What precision formats do they use? (double-double, quad-double, custom floats)
  - How do they handle exponent ranges beyond f64?
  - What's the performance cost of each representation?
  - How do they balance precision vs speed?

  ### 2. BLA Implementation for Deep Zoom

  Research BLA (Bilinear/Bivariate Linear Approximation) at extreme depths:
  - How do professional renderers store BLA coefficients?
  - How do they perform the validity check (|δz|² < r²) when values underflow f64?
  - Do they use different precision for BLA table vs iteration?
  - What is the "scaled" or "normalized" approach some renderers use?

  ### 3. Iteration Strategies

  Research the actual iteration loop at deep zoom:
  - Do professional renderers use f64 for iterations and only HDR for specific checks?
  - Is there a "rescaling" approach that keeps values in f64 range?
  - How do they handle the transition between zoom levels?
  - What's the actual performance profile (iterations/second) they achieve?

  ### 4. Specific Implementations to Study

  Research these specific implementations in detail:

  **Kalles Fraktaler 2+ / KF** (Claude Heiland-Allen)
  - Source: https://mathr.co.uk/kf/kf.html
  - How does it handle deep zoom iteration?
  - What precision formats does it use?

  **Fraktaler-3** (Claude Heiland-Allen)
  - Source: https://code.mathr.co.uk/fraktaler-3
  - What improvements over KF2?
  - BLA implementation details?

  **Imagina** (Zhuoran)
  - Source: https://github.com/ImaginaFractal/Imagina
  - Known for efficient deep zoom
  - MipLA (Mipmap Linear Approximation) implementation

  **FractalShark**
  - Source: https://github.com/mattsaccount364/FractalShark
  - GPU-accelerated deep zoom
  - How does it handle precision on GPU?

  **Very Plotter** (Phil Thompson)
  - Source: https://github.com/philthompson/visualize-primes (plots.js)
  - JavaScript implementation with BLA
  - How does it handle precision in JS?

  ### 5. Key Technical Questions

  Answer these specific questions with citations:

  1. At 10^500 zoom, what number format do professional renderers use for pixel iteration?

  2. How is the BLA validity check performed when |δz|² underflows standard floats?

  3. Is there a "scaled iteration" technique that keeps δz in a manageable range?

  4. What's the typical BLA skip percentage achieved at 10^270+ zoom?

  5. How do renderers handle the derivative (for distance estimation/lighting) at extreme zoom?

  6. What is the memory layout of BLA tables in production implementations?

  7. Are there alternative iteration-skipping techniques besides BLA (series approximation, etc)?

  ### 6. Performance Benchmarks

  Find any published benchmarks or performance data:
  - Iterations per second at various zoom depths
  - BLA effectiveness (% iterations skipped) vs zoom depth
  - Memory usage for BLA tables
  - Comparison between different precision formats

  ## Deliverables

  Provide:

  1. **Summary table** comparing how each major renderer handles:
     - Number representation
     - BLA storage format
     - Validity checking
     - Performance characteristics

  2. **Recommended architecture** for our implementation based on research findings

  3. **Code patterns** - actual code snippets from reference implementations showing:
     - BLA validity check implementation
     - Iteration loop with BLA at deep zoom
     - Number representation used

  4. **Citations** for all claims - link to source code files, papers, or forum posts

  ## Important Notes

  - DO NOT GUESS. If you cannot find definitive information, say so.
  - Cite specific source files and line numbers where possible
  - Focus on implementations that work at 10^250+ zoom, not shallow zoom optimizations
  - We need production-ready approaches, not theoretical algorithms
  - Performance matters - an approach that's 10x slower is not acceptable

  ## Resources to Search

  - mathr.co.uk (Claude Heiland-Allen's site)
  - fractalforums.org (deep zoom discussions)
  - GitHub repositories of listed projects
  - Academic papers on Mandelbrot computation
  - Phil Thompson's BLA article: https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html

---

# Research Findings

## 1. Summary Table: How Professional Renderers Handle Deep Zoom

| Renderer | Number Representation | BLA Storage | Validity Check | Max Depth | Performance |
|----------|----------------------|-------------|----------------|-----------|-------------|
| **Kalles Fraktaler 2+** | Scaled double (e600), Scaled long double (e4900-e9800), FloatExp for deeper | BLA table with 2M entries for M iterations | Uses ε×\|Z\| formula | Arbitrary | ~4.2x slower than double for long double |
| **Fraktaler-3** | FloatExp (float + int32 exponent), rescaled perturbation | Hierarchical BLA tables (2^n levels) | \|z\| < \|r\| with floatexp | 5e-433 in ~2 min | GPU floatexp 2.3x faster than CPU long double |
| **Imagina** | FloatExp, reference compression | Synchronizes with minibrot periods | Standard BLA validity | Very deep | Best CPU-only performance |
| **FractalShark** | HDRFloat (2×32 + exponent), CUDA kernels | LAv2 tables | GPU-accelerated checks | 10e30 (float), deeper with HDR | RTX 4090 optimized |
| **Very Plotter** | FloatExp (JS float + int exponent) | Merge-and-cull strategy (~500 BLAs for 1000 iterations) | Auto-epsilon binary search | 1e1744+ | 36x speedup in some locations |
| **rust-fractal** | Mantissa-exponent extended precision | Series approximation + perturbation | Standard | E50000+ verified | Good via Rayon parallelism |

## 2. Answers to Key Technical Questions

### Q1: At 10^500 zoom, what number format do professional renderers use for pixel iteration?

**Answer:** They use **FloatExp** (or "scaled doubles") - a custom format storing:
- A double-precision mantissa normalized near 1.0
- A separate integer exponent

From [mathr.co.uk](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html):
> "Store the mantissa as a double precision value, but normalized to be near 1 in magnitude, with a separate integer to store the exponent."

**Precision breakdown by depth:**
- Double (f64): up to ~e300
- Scaled double: up to ~e600 (SIMD supported in KF)
- Scaled long double: e4900 to e9800
- FloatExp (float + int32): unlimited depth

### Q2: How is the BLA validity check performed when |δz|² underflows standard floats?

**Answer:** The validity check is reformulated to avoid squaring δz directly. From [Phil Thompson's BLA article](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html):

The validity radius formula is:
```
r = ε × |Z_n| - |B| × |δc| / |A|
```

Where:
- ε = hardware precision (2^-53 for f64, 2^-24 for f32)
- Z_n = reference orbit value at iteration n
- A, B = BLA coefficients
- δc = pixel delta from reference

**The key insight:** The check becomes `|δz| < r` (linear), NOT `|δz|² < r²` (quadratic). This avoids the underflow problem entirely since you're comparing magnitudes, not squared magnitudes.

From [mathr.co.uk](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html):
> "Pick the BLA that skips the most iterations, among those starting at the current reference iteration that satisfy |z| < |r|."

**When using FloatExp:** All BLA coefficients and validity radii are stored in FloatExp format, so comparisons work at any depth.

### Q3: Is there a "scaled iteration" technique that keeps δz in a manageable range?

**Answer:** Yes! This is called **rescaled perturbation** in Kalles Fraktaler.

From [mathr.co.uk](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html):
> "Use substitution z = Sw and c = Sd to keep double-precision values near magnitude 1, avoiding underflow at 1e-308. This rescaling typically occurs every few hundred iterations."

The rescaled formula becomes:
```
w → 2Zw + Sw² + d
```

Where S is chosen so |w| ≈ 1. Key optimization:
> "If S underflowed to 0 in double precision, you don't need to calculate the + Sw² term at all when Z is not small."

From [Kalles Fraktaler Manual](https://mathr.co.uk/kf/manual.html):
> "Rescaled perturbation calculations for arbitrarily deep zooms (usually faster than old long double and floatexp implementations; with or without derivatives)"

### Q4: What's the typical BLA skip percentage achieved at 10^270+ zoom?

**Answer:** From Phil Thompson's benchmarks:
- **"Evolution of Trees" location:** 989,000 iterations skipped per pixel with BLA
- **"Cerebral Spin" location:** 36.2× faster with BLA than without
- Skip percentages can exceed 99% at deep zoom

The effectiveness depends heavily on the location. Near minibrot centers, BLA is extremely effective. At "deep needle" locations, Series Approximation may be better.

### Q5: How do renderers handle the derivative (for distance estimation/lighting) at extreme zoom?

**Answer:** From [Kalles Fraktaler Manual](https://mathr.co.uk/kf/manual.html):
> "SIMD is not yet implemented for scaled double with derivatives."
> "Rescaled perturbation calculations... with or without derivatives"

Derivatives are computed alongside the perturbation iteration, using the same precision format. The derivative follows the chain rule:
```
dz/dc → 2Z × (dz/dc) + 2z × (dz/dc) + 1
```

At extreme depths, derivatives are stored in FloatExp format.

### Q6: What is the memory layout of BLA tables in production implementations?

**Answer:** From [Phil Thompson's BLA article](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html):

BLA tables use a **hierarchical structure**:
- **Level 1:** M entries, each skipping 1 iteration (A₁ = 2Z, B₁ = 1)
- **Level 2:** M/2 entries, each skipping 2 iterations (merged from Level 1)
- **Level n:** M/2^(n-1) entries, each skipping 2^(n-1) iterations

**Merge formulas:**
```
A(y∘x) = Ay × Ax
B(y∘x) = Ay × Bx + By
l(y∘x) = lx + ly  (iterations skipped)
```

**Memory optimization:** Using "merge-and-cull" strategy:
> "A 1,000-iteration reference orbit requires only ~500 stored BLAs instead of millions."

Each BLA entry stores:
- A coefficient (complex, FloatExp)
- B coefficient (complex, FloatExp)
- Validity radius r (real, FloatExp)
- Skip count l (integer)

### Q7: Are there alternative iteration-skipping techniques besides BLA?

**Answer:** Yes, several:

1. **Series Approximation (SA):** Older method, computes polynomial coefficients
   - More complex to implement
   - Better for some "deep needle" locations
   - From KF Manual: "Series approximation uses Horner's rule to evaluate polynomials"

2. **NanoMB1/NanoMB2:** Experimental "bivariate super-series-approximation"
   - From [smurfix/kf2](https://github.com/smurfix/kf2): "knighty's experimental NanoMB1 algorithm"

3. **Rebasing:** Technique to avoid glitches (proposed by Zhuoran)
   - Resets reference iteration when pixel orbit gets near critical point
   - From [mathr.co.uk](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html): "Rebasing to avoid glitches: when |Z_m + z_n| < |z_n|, replace z_n with Z_m + z_n and reset m to 0"

4. **Reference Compression:** Memory optimization for deep zooms
   - From Imagina: Decompresses reference orbit on-the-fly
   - "Period-600,000,000 spot can see memory reduction of multiple gigabytes"

## 3. Recommended Architecture for FractalWonder

Based on the research, here's the recommended approach for your WASM/Rust implementation:

### 3.1 Number Representation Strategy

**Implement a tiered precision system:**

```
Zoom Depth        | Format           | BLA Validity Check
------------------|------------------|--------------------
< 10^300          | f64              | f64 (no underflow)
10^300 - 10^600   | Scaled f64       | FloatExp for r
10^600 - 10^1000+ | FloatExp         | FloatExp
```

**FloatExp structure (recommended):**
```rust
struct FloatExp {
    mantissa: f64,    // Normalized to [1.0, 10.0) or (-10.0, -1.0]
    exponent: i32,    // Base-10 exponent (can use i64 for extreme depths)
}
```

### 3.2 BLA Implementation Strategy

**Critical insight for your problem:**

Your current issue is that BLA only works with HDRFloat because validity checks underflow in f64. The solution:

1. **Store BLA coefficients and validity radii in FloatExp format always**
2. **Perform BLA validity check in FloatExp** (cheap - just compare mantissas if exponents match)
3. **Perform actual iteration in f64** when values are in range, FloatExp otherwise

**The key optimization:** The BLA validity check `|δz| < r` is much cheaper in FloatExp than full iteration in FloatExp, because:
- It's a single comparison per BLA level tested
- vs. complex multiplication and addition for each iteration

**Pseudo-code for iteration loop:**
```rust
fn iterate_with_bla(delta_z: FloatExp, delta_c: FloatExp, bla_table: &BlaTable) {
    let mut n = 0;  // Reference iteration
    let mut m = 0;  // Pixel iteration

    while m < max_iterations {
        // Try to find valid BLA (highest level that skips most iterations)
        if let Some(bla) = find_valid_bla(bla_table, n, delta_z.magnitude()) {
            // Apply BLA: δz_new = A × δz + B × δc
            delta_z = bla.A * delta_z + bla.B * delta_c;
            n += bla.skip_count;
            m += bla.skip_count;
        } else {
            // Regular perturbation iteration
            // δz_new = 2 × Z_n × δz + δz² + δc
            delta_z = 2.0 * Z[n] * delta_z + delta_z * delta_z + delta_c;
            n += 1;
            m += 1;

            // Rebasing check
            if should_rebase(Z[n], delta_z) {
                delta_z = Z[n] + delta_z;
                n = 0;
            }
        }

        // Escape check
        if (Z[n] + delta_z).magnitude() > 4.0 {
            return m;
        }
    }
}
```

### 3.3 Hybrid f64/FloatExp Approach

**The key to performance:**

From [mathr.co.uk](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html):
> "Using floatexp from the get go avoids many branches and rescaling in the inner loop, so it's significantly faster" [for deep needle locations]

However, for most locations:
1. Use f64 for the actual iteration (fast)
2. Use FloatExp only for:
   - BLA coefficient storage
   - Validity radius comparisons
   - When |δz| gets too small for f64

**The rescaling trick:**
When δz approaches f64 underflow, rescale:
```rust
if delta_z.abs() < 1e-300 {
    // Convert to FloatExp, continue in that mode
    delta_z_floatexp = FloatExp::from_f64(delta_z);
}
```

### 3.4 BLA Table Construction

From [Phil Thompson's article](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html):

**Level 1 BLA coefficients:**
```
A_n = 2 × Z_n
B_n = 1
r_n = ε × |Z_n|  (approximate)
```

**Merging (Level 2+):**
```
A_merged = A_y × A_x
B_merged = A_y × B_x + B_y
r_merged = min(r_x, r_y / |A_x|)  (approximate)
```

### 3.5 Performance Expectations

Based on research:
- FloatExp iterations are ~4-10x slower than f64
- BLA can skip 90-99%+ of iterations at deep zoom
- Net result: BLA with FloatExp validity checks should be **much faster** than non-BLA f64

From [Fractalshades documentation](https://gbillotey.github.io/Fractalshades-doc/API/arbitrary_models.html):
- BLA_eps parameter default: 1e-6
- Can be disabled by setting to None
- Chained bilinear approximations across perturbation classes

## 4. Code Patterns from Reference Implementations

### 4.1 FloatExp Implementation (from Very Plotter's floatexp.js)

```javascript
// Normalized mantissa between 1 and 10
function floatExpAlign(a) {
    if (a.v === 0) return { v: 0, e: 0 };
    const exp = Math.floor(Math.log10(Math.abs(a.v)));
    return { v: a.v / Math.pow(10, exp), e: a.e + exp };
}

// Multiplication
function floatExpMul(a, b) {
    return floatExpAlign({ v: a.v * b.v, e: a.e + b.e });
}

// Addition (with underflow handling)
function floatExpAdd(a, b) {
    const eDiff = a.e - b.e;
    if (eDiff > 307) return a;  // b is negligible
    if (eDiff < -307) return b; // a is negligible
    if (eDiff >= 0) {
        return floatExpAlign({ v: a.v + b.v / Math.pow(10, eDiff), e: a.e });
    } else {
        return floatExpAlign({ v: a.v / Math.pow(10, -eDiff) + b.v, e: b.e });
    }
}
```

### 4.2 BLA Validity Check Pattern

From [mathr.co.uk](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html):

```
// For Mandelbrot set:
r_n = max(0, ε × (|Z_n| - max_image{|δc|}) / (|2Z_n| + 1))

// Simplified (with safety margin):
r_n = ε × |Z_n| / 2

// BLA is valid when:
|δz_n| < r_n
```

### 4.3 Rescaled Perturbation (from mathr.co.uk)

The substitution z = Sw keeps values near 1:
```
w_{n+1} = 2Z_n × w_n + S × w_n² + d
```

Where:
- S = current scale factor (updated periodically)
- w = scaled perturbation (|w| ≈ 1)
- d = δc / S

## 5. Specific Recommendations for Your Implementation

### Problem: BLA validity check underflows in f64

**Solution:**

1. **Keep your current HDRFloat (FloatExp) for BLA table storage** - this is correct
2. **Compute BLA validity radii in HDRFloat** - store as HDRFloat in BLA table
3. **During iteration:**
   - Convert current δz magnitude to HDRFloat for validity comparison (cheap)
   - If BLA is valid: apply BLA coefficients (can be done in f64 if result is in range)
   - If BLA is not valid: do regular f64 perturbation iteration (fast)

**Key insight:** The validity check itself doesn't need the actual δz value - it just needs its magnitude compared to r. You can track the magnitude separately:

```rust
struct IterationState {
    delta_z: Complex<f64>,      // The actual value (when in f64 range)
    delta_z_exp: i32,           // Implicit exponent (for underflow tracking)
}

fn magnitude_floatexp(&self) -> FloatExp {
    FloatExp {
        mantissa: self.delta_z.norm(),
        exponent: self.delta_z_exp,
    }
}
```

This way, you can:
- Do fast f64 arithmetic on `delta_z`
- Track the true magnitude via `delta_z_exp`
- Compare against BLA validity radii in FloatExp
- Only switch to full FloatExp when absolutely necessary

### Performance Estimate

If your current f64 path does 0% BLA skipping and HDRFloat path is "unacceptably slow":

With proper BLA at 10^270 zoom:
- Expected BLA skip rate: 95%+ of iterations
- Only 5% of iterations need actual computation
- Of those, most can use f64 (only edge cases need HDRFloat)
- **Expected speedup: 10-50x over non-BLA f64**

## 6. Citations and Sources

All findings are sourced from:

1. [Phil Thompson's BLA Article](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html) - Primary BLA implementation guide
2. [mathr.co.uk - Deep zoom theory and practice (2021)](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html) - FloatExp, rescaling
3. [mathr.co.uk - Deep zoom theory and practice again (2022)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html) - BLA, rebasing
4. [Kalles Fraktaler 2+ Manual](https://mathr.co.uk/kf/manual.html) - Scaled double, rescaled perturbation
5. [FractalShark GitHub](https://github.com/mattsaccount364/FractalShark) - CUDA HDRFloat implementation
6. [Imagina GitHub](https://github.com/5E-324/Imagina) - Reference compression, MipLA
7. [rust-fractal GitHub](https://github.com/rust-fractal/rust-fractal-core) - Rust implementation reference
8. [Very Plotter GitHub](https://github.com/philthompson/visualize-primes) - JavaScript FloatExp implementation
9. [Fractalshades Documentation](https://gbillotey.github.io/Fractalshades-doc/API/arbitrary_models.html) - Python BLA implementation
10. [DeviantArt - New deep zoom algorithms](https://www.deviantart.com/microfractal/journal/New-deep-zoom-algorithms-for-fractals-933730336) - Algorithm overview
11. [WebGL Mandelbrot Deep Zoom](https://ambrosecavalier.com/projects/gpu-deep-zoom/about/) - GPU perturbation implementation

## 7. Gaps in Research (What I Could NOT Confirm)

1. **Exact BLA coefficient memory layout** - Found hierarchical structure but not byte-level layout
2. **Shared exponent optimization** - Mentioned in some sources but no implementation details found
3. **Specific iteration/second benchmarks** - Performance is location-dependent; no standardized benchmarks found
4. **MipLA vs BLA comparison** - Imagina's MipLA mentioned but implementation details not accessible (code being rewritten)
5. **WASM-specific optimizations** - No production WASM deep zoom implementations found for reference

---

# Deep Algorithm Analysis

## 8. How Professional Renderers Work (Precise Documentation)

### 8.1 Kalles Fraktaler 2+ (KF)

**Source:** [smurfix/kf2 on GitHub](https://github.com/smurfix/kf2), [mathr.co.uk/kf](https://mathr.co.uk/kf/kf.html)

#### Number Types and Switching Thresholds

KF uses **dynamic precision selection** based on zoom depth via `GetReferenceType(m_nZoom)`:

| Zoom Depth | Number Type | Description |
|------------|-------------|-------------|
| < ~1e300 | `double` (f64) | Native IEEE 754 double precision |
| ~1e300 - ~1e600 | `scaled double` | f64 with periodic rescaling to prevent underflow |
| ~1e600 - ~1e4900 | `long double` (x87) | 80-bit extended precision (CPU only) |
| > ~1e4900 | `floatexp` | f64 mantissa + i32 exponent |

**Precision selection code pattern** (from `fraktal_sft.cpp`):
```cpp
Reference_Float, Reference_ScaledFloat → tfloatexp<float, int32_t>
Reference_Double, Reference_ScaledDouble → double
Reference_FloatExpFloat → tfloatexp<float, int32_t>
Reference_FloatExpDouble → floatexp (double mantissa)
```

**Switching logic:** The zoom level determines the reference type. OpenCL kernels use float32 or float64 depending on device capability. For zooms between ~1e300 and ~1e4900, CPU is faster than GPU because OpenCL cannot use x87 long double.

#### Floatexp Structure (from `floatexp.h`)

```cpp
template<typename mantissa_t, typename exponent_t>
struct tfloatexp {
    mantissa_t val;  // Normalized to [0.5, 1.0)
    exponent_t exp;  // Base-2 exponent

    void align() {
        // Extract native exponent from mantissa's IEEE representation
        // Transfer to explicit exponent field
        // Handle zero, denormalized, overflow/underflow
    }
};
```

**Arithmetic operations:**
- Multiplication: multiply mantissas, add exponents, then normalize
- Addition: align exponents by shifting smaller mantissa, add, then normalize
- Values differing by > MAX_PREC bits use "sticky" semantics (smaller ignored)

#### Iteration Acceleration: Series Approximation (SA)

KF uses **Series Approximation** (not BLA) to skip iterations:

**Taylor series coefficients stored in:**
- `m_APr[]`, `m_APi[]` - coefficient arrays for real/imaginary parts
- `m_APs` - `SeriesR2<double, int64_t>` structure
- `m_nMaxApproximation` - iteration count where SA applies
- `m_nTerms` - number of polynomial terms (configurable)

**Algorithm:**
1. Compute Taylor series: `z_n = Σ(c_k × δ^k)` where δ is perturbation from center
2. Evaluate polynomial to skip directly to iteration n
3. Continue with standard perturbation from there

**SA vs BLA:** SA computes a polynomial approximation at render start. BLA applies linear approximations during iteration. KF manual states BLA is "unlikely to be implemented in KF any time soon" due to the 100+ formula support requirement.

#### Rescaled Perturbation

For arbitrarily deep zooms, KF uses **rescaled perturbation**:

**Concept:** Substitute `z = S·w` where S is a scale factor, keeping `|w| ≈ 1` to prevent underflow.

**Formula transformation:**
```
Standard: δz' = 2·Z·δz + δz² + δc
Rescaled: w' = 2·Z·w + S·w² + d   (where d = δc/S)
```

**Key optimization:** When S underflows to 0 in f64, skip the `S·w²` term entirely (it's negligible).

**From KF manual:**
> "Rescaled perturbation calculations for arbitrarily deep zooms (usually faster than old long double and floatexp implementations; with or without derivatives)"

---

### 8.2 Fraktaler-3

**Source:** [code.mathr.co.uk/fraktaler-3](https://code.mathr.co.uk/fraktaler-3), [mathr.co.uk blog](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)

#### Number Types

| Type | Precision | Use Case |
|------|-----------|----------|
| `float` | 24-bit mantissa | Low zoom, can overflow with high powers |
| `double` | 53-bit mantissa | Standard precision |
| `floatexp` | float + int32 exp | Extended range |
| `doubleexp` | double + int32 exp | Extended range + precision |
| `float128` | 113-bit mantissa | Extreme precision (where supported) |

Wisdom system automatically selects optimal type per hardware.

#### BLA Implementation

Fraktaler-3 implements **Bivariate Linear Approximation (BLA)**.

**BLA validity radius formula** (from mathr.co.uk):

For Mandelbrot set:
```
R = max{ 0, ε·|Z| - |B|·|δc| / |A| }
```

For Burning Ship (handles absolute value folds):
```
R = max{ 0, min{ ε·inf|A| - (sup|B|/inf|A|)·|c|, |X|, |Y| } }
```

**Fudge factor:** Divides |X| and |Y| by 2 for safety margin.

**BLA coefficient merging:**
```
A_merged = A_y × A_x
B_merged = A_y × B_x + B_y
R_merged = min(R_x, R_y / |A_x|)
l_merged = l_x + l_y
```

**Search algorithm:**
1. Start at highest BLA level (largest skip)
2. Check if `|δz| < R` for that BLA
3. If valid → apply BLA, advance by l iterations
4. If invalid → try next lower level
5. If no BLA valid → do 1 standard iteration

**Key detail:** One BLA table per reference orbit. For hybrid formulas with multiple phases, need multiple reference orbits (one per phase).

---

### 8.3 rust-fractal

**Source:** [github.com/rust-fractal/rust-fractal-core](https://github.com/rust-fractal/rust-fractal-core)

#### FloatExtended Structure (from `float_extended.rs`)

```rust
struct FloatExtended {
    mantissa: f64,  // Normalized via frexp()
    exponent: i32,  // Base-2 exponent
}
```

**Operations:**
- `reduce()`: calls `frexp()` on mantissa to extract binary exponent, updates overall exponent
- Addition/subtraction: align exponents by shifting smaller mantissa
- Multiplication: multiply mantissas, add exponents, normalize

#### Precision Switching

**Threshold** (from `perturbation.rs`):
```rust
// Escape check threshold: exponent > -500
if pixel.delta_current.exponent > -500 {
    // Use fixed-point (f64) path with escape checks
} else {
    // Switch to extended precision
}
```

**Batch processing:** Iterations processed in batches of `400 / FRACTAL_POWER` before checking precision.

#### Series Approximation (from `series_approximation.rs`)

**Coefficient storage:**
```rust
Vec<Vec<ComplexExtended>>
// Stored every 100 iterations (data_storage_interval)
// Index: coefficients[(iteration - 1) / data_storage_interval]
```

**Validation:**
- Compare `relative_error / derivative` against `delta_pixel_square`
- If exceeded, go back to nearest valid stored value
- Probe testing: 4 corners first (10x larger tolerance), then remaining probes

**Iteration skipping:**
```rust
probe = probe * (2*reference[i] + probe) + delta_probe
// When accuracy validated at checkpoints:
probe_iteration_level += data_storage_interval  // Skip ahead
```

---

### 8.4 FractalShark

**Source:** [github.com/mattsaccount364/FractalShark](https://github.com/mattsaccount364/FractalShark)

#### Precision Types

**Template-based polymorphic storage:**
- `float`, `double` - native types
- `HDRFloat<float>`, `HDRFloat<double>` - extended range
- `CudaDblflt<dblflt>`, `CudaDblflt<MattDblflt>` - CUDA-optimized
- `FloatComplex<SubType>`, `HDRFloatComplex<SubType>` - complex variants

**HDRFloat structure:** "2×32 + exponent" - two 32-bit floats for mantissa precision plus integer exponent.

#### CUDA Kernels

**Main kernels in `render_gpu.cu`:**
- `mandel_1x_float_perturb` - straight perturbation up to ~1e30 (no LA)
- `mandel_1xHDR_float_perturb_bla` - HDR with BLA
- `mandel_1xHDR_float_perturb_lav2` - HDR with Linear Approximation v2

**From README:**
> "For better performance at low zoom levels, `mandel_1x_float_perturb` leaves out linear approximation and just does straight perturbation up to ~10e30, which corresponds with the 32-bit float exponent range."

#### LAv2 (Linear Approximation v2)

**Data structures** (from `LAReference.h`):
- `GrowableVector<LAInfoDeep<...>> m_LAs` - LA entries (default capacity 10,000)
- `GrowableVector<LAStageInfo<IterType>> m_LAStages` - hierarchical stages (max 1,024)

**Algorithm phases:**
1. **Orbit computation:** `CreateLAFromOrbit()` or `CreateLAFromOrbitMT()` (multi-threaded)
2. **Stage management:** `CreateNewLAStage()` organizes into hierarchical levels
3. **AT generation:** `CreateATFromLA(Float radius, bool UseSmallExponents)` creates Approximation Theory data

**Radius computation:** Encapsulated in `CreateATFromLA()` - specific formula not exposed in headers.

---

## 9. How Our Implementation Works (Precise Documentation)

### 9.1 Number Types

**File:** `fractalwonder-core/src/hdrfloat.rs`

#### HDRFloat Structure

```rust
pub struct HDRFloat {
    pub head: f32,   // Primary mantissa, normalized to [0.5, 1.0)
    pub tail: f32,   // Error term, |tail| ≤ 0.5 × ulp(head)
    pub exp: i32,    // Binary exponent (base 2)
}
// Value = (head + tail) × 2^exp
// Provides ~48-bit mantissa precision using two f32 values
```

**Normalization** (`normalize()` method):
- Extracts exponent via bit manipulation from head
- Adjusts to [0.5, 1.0) range
- Scales tail by same factor

**Arithmetic operations:**
- Addition: align exponents, two-sum for error-free head addition, combine tails
- Multiplication: primary product + FMA error term + cross terms
- Uses Knuth's two-sum algorithm for error tracking

#### F64Complex Structure

**File:** `fractalwonder-core/src/complex_delta.rs`

```rust
pub struct F64Complex {
    pub re: f64,
    pub im: f64,
}
```

Standard f64 complex arithmetic with no extended range.

#### HDRComplex Structure

**File:** `fractalwonder-core/src/hdrcomplex.rs`

```rust
pub struct HDRComplex {
    pub re: HDRFloat,
    pub im: HDRFloat,
}
```

All operations use HDRFloat arithmetic.

### 9.2 Precision Switching

**File:** `fractalwonder-compute/src/worker.rs`, lines 386-481

#### Threshold

```rust
// Line 389-393
let delta_log2 = delta_c_origin.0.log2_approx()
    .max(delta_c_origin.1.log2_approx());
let deltas_fit_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;
```

**Decision:** If `log2(delta)` is between -900 and 900, use f64. Otherwise use HDRFloat.

#### f64 Path (lines 407-432)

```rust
if deltas_fit_f64 {
    // Convert BigFloat deltas to f64
    let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
    let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());

    for _py in 0..tile.height {
        for _px in 0..tile.width {
            let result = compute_pixel_perturbation(
                &orbit,
                F64Complex::from_f64_pair(delta_c.0, delta_c.1),  // f64
                max_iterations,
                tau_sq,
            );
            // ...
        }
    }
}
```

**Note:** f64 path uses `compute_pixel_perturbation<F64Complex>()` - NO BLA.

#### HDRFloat Path (lines 433-481)

```rust
else {
    // Convert BigFloat deltas to HDRComplex
    let delta_origin = HDRComplex {
        re: HDRFloat::from_bigfloat(&delta_c_origin.0),
        im: HDRFloat::from_bigfloat(&delta_c_origin.1),
    };

    for _py in 0..tile.height {
        for _px in 0..tile.width {
            if bla_enabled {
                if let Some(ref bla_table) = cached.bla_table {
                    let (result, stats) = compute_pixel_perturbation_hdr_bla(
                        &orbit, bla_table, delta_c, max_iterations, tau_sq,
                    );
                    // ...
                }
            }
            // ...
        }
    }
}
```

**Note:** HDRFloat path can use BLA, but ALL iteration arithmetic is HDRFloat.

### 9.3 BLA Implementation

**File:** `fractalwonder-compute/src/bla.rs`

#### BLA Entry Structure

```rust
pub struct BlaEntry {
    pub a: HDRComplex,     // Coefficient A (multiplies δz)
    pub b: HDRComplex,     // Coefficient B (multiplies δc)
    pub l: u32,            // Iterations to skip
    pub r_sq: HDRFloat,    // Validity radius squared
}
```

#### Level 1 BLA Creation (lines 27-45)

```rust
pub fn from_orbit_point(z_re: f64, z_im: f64) -> Self {
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
        l: 1,
        r_sq: HDRFloat::from_f64(r * r),  // r² stored, not r
    }
}
```

#### BLA Merging (lines 52-90)

```rust
pub fn merge(x: &BlaEntry, y: &BlaEntry, dc_max: &HDRFloat) -> BlaEntry {
    let a = y.a.mul(&x.a);
    let b = y.a.mul(&x.b).add(&y.b);

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

#### BLA Table Construction (lines 104-163)

```rust
pub fn compute(orbit: &ReferenceOrbit, dc_max: &HDRFloat) -> Self {
    let m = orbit.orbit.len();
    let num_levels = ((m as f64).log2().ceil() as usize).max(1) + 1;

    // Level 0: single-iteration BLAs
    for &(z_re, z_im) in &orbit.orbit {
        entries.push(BlaEntry::from_orbit_point(z_re, z_im));
    }

    // Build higher levels by merging pairs
    for _level in 1..num_levels {
        for i in 0..this_level_size {
            let merged = BlaEntry::merge(&entries[x_idx], &entries[y_idx], dc_max);
            entries.push(merged);
        }
    }
}
```

#### BLA Search - THE CRITICAL LIMITATION (lines 175-241)

```rust
pub fn find_valid(&self, m: usize, dz_mag_sq: &HDRFloat, dc_max: &HDRFloat) -> Option<&BlaEntry> {
    // *** LINE 188-189: THE CRIPPLING LIMITATION ***
    let max_skip: u32 = 1;

    // Maximum allowed |B| * dc_max
    let max_b_dc_exp = 0;  // 2^0 = 1

    // Search from highest level down to level 0
    for level in (0..self.num_levels).rev() {
        let skip_size = 1usize << level;  // 2^level iterations per entry

        // *** LINES 201-203: SKIPS ALL USEFUL LEVELS ***
        if skip_size > max_skip as usize {
            continue;  // Skip levels 1, 2, 3, ... (all levels that skip >1 iteration)
        }

        // ... validity check ...
        let diff = dz_mag_sq.sub(&entry.r_sq);
        let validity_check = diff.is_negative();  // |δz|² < r²

        if validity_check {
            // Check B coefficient magnitude
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

**Comment on line 186-188:**
```rust
// Limit BLA to level-0 only (skip 1 iteration at a time).
// This fixes center tile uniform color bug at deep zoom but is slow.
// TODO: Find a way to only limit center tile, not all tiles.
```

### 9.4 Perturbation Iteration

**File:** `fractalwonder-compute/src/perturbation.rs`

#### HDRFloat BLA Path (lines 117-307)

```rust
pub fn compute_pixel_perturbation_hdr_bla(
    orbit: &ReferenceOrbit,
    bla_table: &BlaTable,
    delta_c: HDRComplex,       // HDRComplex input
    max_iterations: u32,
    tau_sq: f64,
) -> (MandelbrotData, BlaStats) {
    let mut dz = HDRComplex::ZERO;   // δz is HDRComplex
    let mut drho = HDRComplex::ZERO; // δρ is HDRComplex

    while n < max_iterations {
        // ... escape check, glitch detection, rebase check ...

        // Line 214: Try BLA
        let bla_entry = bla_table.find_valid(m, &dz_mag_sq, bla_table.dc_max());

        if let Some(bla) = bla_entry {
            // Apply BLA: δz_new = A·δz + B·δc (HDRComplex operations)
            let a_dz = bla.a.mul(&dz);
            let b_dc = bla.b.mul(&delta_c);
            dz = a_dz.add(&b_dc);
            // ...
        } else {
            // Standard iteration - ALL HDRFloat arithmetic
            // Lines 229-244
            let two_z_dz_re = dz.re.mul_f64(z_m_re)
                .sub(&dz.im.mul_f64(z_m_im))
                .mul_f64(2.0);
            let two_z_dz_im = dz.re.mul_f64(z_m_im)
                .add(&dz.im.mul_f64(z_m_re))
                .mul_f64(2.0);
            let dz_sq = dz.square();

            dz = HDRComplex {
                re: two_z_dz_re.add(&dz_sq.re).add(&delta_c.re),
                im: two_z_dz_im.add(&dz_sq.im).add(&delta_c.im),
            };
            // ... derivative iteration also all HDRFloat ...
        }
    }
}
```

#### Generic f64 Path (lines 312-408)

```rust
pub fn compute_pixel_perturbation<D: ComplexDelta>(
    orbit: &ReferenceOrbit,
    delta_c: D,              // Generic - can be F64Complex
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    // ... iteration loop ...
    // NO BLA - just standard perturbation
}
```

---

## 10. Precise Differences Between Our Implementation and Professional Renderers

### 10.1 BLA Skip Limitation

| Aspect | Professional Renderers | Our Implementation |
|--------|----------------------|-------------------|
| **Max iterations skipped per BLA** | 2^(log₂M) for M-iteration orbit | **1** |
| **BLA levels used** | All levels (highest to lowest) | **Level 0 only** |
| **Expected skip percentage** | 90-99% at deep zoom | **0% effective** |
| **Result** | Massive speedup | No speedup from BLA |

**Our code (bla.rs:188-203):**
```rust
let max_skip: u32 = 1;  // HARD-CODED TO 1
// ...
if skip_size > max_skip as usize {
    continue;  // SKIPS ALL LEVELS EXCEPT LEVEL 0
}
```

**Professional renderers:** Search all levels from highest (largest skip) to lowest (single iteration).

### 10.2 Precision During Iteration

| Aspect | Professional Renderers | Our Implementation |
|--------|----------------------|-------------------|
| **f64 iteration** | Used when values fit f64 range | Used only when initial deltas fit f64 |
| **HDRFloat iteration** | Only when f64 would underflow/overflow | Used for ALL operations if initial deltas don't fit f64 |
| **Hybrid approach** | Switch precision mid-iteration as needed | **No mid-iteration switching** |
| **Validity check precision** | HDRFloat/FloatExp for comparison only | HDRFloat for comparison only (correct) |

**rust-fractal approach (perturbation.rs):**
```rust
if pixel.delta_current.exponent > -500 {
    // f64 path with escape checks
} else {
    // Switch to extended precision
}
```

**Our approach (worker.rs:389-393):**
```rust
let deltas_fit_f64 = delta_log2 > -900.0 && delta_log2 < 900.0;
// Decision made ONCE at tile start, not during iteration
```

**Key difference:** rust-fractal switches precision **during iteration** based on current δz exponent. We decide **once** based on initial δc, then use that precision for the entire tile.

### 10.3 Rescaled Perturbation

| Aspect | Professional Renderers | Our Implementation |
|--------|----------------------|-------------------|
| **Rescaling** | Periodic rescaling keeps values near 1 | **None** |
| **Formula** | w' = 2·Z·w + S·w² + d | Standard: δz' = 2·Z·δz + δz² + δc |
| **Benefit** | Can use f64 even at extreme zoom | Must use HDRFloat when δz underflows |

**KF/Fraktaler approach:**
```
When |δz| approaches underflow threshold:
  S = |δz|
  w = δz / S  (now |w| ≈ 1)
  d = δc / S
  Continue iteration with rescaled formula
```

**Our approach:** No rescaling. When δz underflows f64, ALL arithmetic switches to HDRFloat.

### 10.4 Validity Check: |δz|² vs |δz|

| Aspect | Professional Renderers | Our Implementation |
|--------|----------------------|-------------------|
| **Comparison** | `\|δz\| < r` (linear) | `\|δz\|² < r²` (quadratic) |
| **Storage** | r stored directly | **r² stored** |
| **Implication** | No squaring in hot path | Squaring introduces error |

**Our code (bla.rs:43-44, 225-226):**
```rust
// Storage: r² not r
r_sq: HDRFloat::from_f64(r * r),

// Check: compare squared values
let diff = dz_mag_sq.sub(&entry.r_sq);
let validity_check = diff.is_negative();
```

**Phil Thompson's article:**
> "The validity radius formula determines if a BLA applies: |δz| < r"

**Note:** Comparing squared values is mathematically equivalent, but:
1. Requires computing δz² (which we already do for norm_sq_hdr)
2. Introduces additional rounding in HDRFloat squaring
3. Doesn't match the published formulas (potential source of bugs)

### 10.5 Alternative Acceleration Methods

| Method | Professional Renderers | Our Implementation |
|--------|----------------------|-------------------|
| **Series Approximation** | KF, rust-fractal have SA as primary/fallback | **None** |
| **Reference compression** | Imagina, FractalShark implement | **None** |
| **Probe-based validation** | rust-fractal validates SA with corner probes | **None** |

**rust-fractal SA storage:**
```rust
// Coefficients stored every 100 iterations
Vec<Vec<ComplexExtended>>
coefficients[(iteration - 1) / data_storage_interval]
```

**Our implementation:** BLA only. No series approximation fallback. No probe validation.

### 10.6 Summary Table

| Feature | KF | Fraktaler-3 | rust-fractal | FractalShark | **Ours** |
|---------|----|----|--------------|--------------|----------|
| BLA/LA multi-level | SA instead | ✓ Full | SA | ✓ LAv2 | **Level 0 only** |
| Precision switching | Zoom-based | Auto | Exponent-based | Template | **Tile start only** |
| Rescaled perturbation | ✓ | ✓ | ✓ | ✓ | **✗** |
| Series approximation | ✓ Primary | ✗ | ✓ Fallback | ✓ | **✗** |
| Mid-iteration precision switch | ✓ | ✓ | ✓ | ✓ | **✗** |
| Reference compression | ✗ | ✗ | ✗ | ✓ | **✗** |

### 10.7 The Root Causes of Our Performance Problem

1. **BLA is disabled** (`max_skip = 1`): We build the full BLA table with all levels, then refuse to use any level except level 0. This provides zero benefit over not having BLA.

2. **All-or-nothing precision**: When initial deltas don't fit f64, we use HDRFloat for EVERY operation in EVERY iteration. Professional renderers switch precision only when needed during iteration.

3. **No rescaling**: Professional renderers keep δz near magnitude 1 by periodic rescaling, allowing continued use of fast f64 arithmetic. We don't rescale, forcing full HDRFloat arithmetic.

4. **No fallback acceleration**: When BLA isn't working (which is always, due to #1), professional renderers fall back to Series Approximation. We have no fallback.

The comment in `bla.rs:186-188` reveals the underlying bug:
```rust
// This fixes center tile uniform color bug at deep zoom but is slow.
// TODO: Find a way to only limit center tile, not all tiles.
```

**The "center tile uniform color bug" is the actual problem to fix** - not the BLA skip count. Limiting BLA to skip=1 is a workaround that destroys performance while hiding the real bug.