# Improving Glitches and Rendering Artifacts

This document analyzes the current perturbation implementation to identify causes of rendering artifacts,
particularly "streaks along gradients" that appear across the entire image rather than localized glitches.

## Problem Statement

The renderer produces visible streaks along areas that should show smooth color gradients. Unlike typical
perturbation glitches (localized "noisy" patches or flat color blobs near embedded Julia sets), these
artifacts span the entire image and follow iteration count gradients.

CRITICAL NOTES:

- CPU and GPU renderers show near identical results
- With 3D lighting enabled, the streaks/artefacts are visible. These are **NOT** normal 3D lighting effects. They
  are in the wrong location and don't appear to follow a "natural curvature" line. Without 3D lighting, the image
  shows smooth gradients.

## Diagnostic Test

We propose implementing a diagnostic test to see the difference between CPU and GPU renderers' MandelbrotData output
structure from the core algorithms.

We want to compute a single horizontal row of 773 pixels for each renderer and compare any differences, based on the
following viewport:

  Center X:  0.2730003074955790977152000943102539224941034901877971829668126297063303407832423613869955086599456315688065774822084767969195008271260025052238452032391007263549994293766362921954037893505083246289924017524293579630007968065405107950517380261032289242338419178553209271869818436155991127814312651370767039529790390371669202702173115030152440733903704744891978701000886412551973402111747412797857562890678439266725717111306584186451326456823944631946842223793250196344441718118616611401798736302156711562899499186682755344237831667472378161701650189215869661245649090085723648929497017803093570326388018031614422864596391699687669402025984504194187361856596306389038337007658182380034042958910838285175922191326544301967582883277101985182849925838785480664935840850519795743754970843257750371804341910672629844310833648878540083867067043508254608465780851461280479281295322114922286840675529678822383095381787353285116296030807330820596880639823206658671223760724173129025359067138182337171720749800569
  Center Y:  0.005838718497531293679839354462882728828030188792949767250660666951674130465532588091396071885418790682911941182466374117896236132252584247402520010866544094350570137907725338151684505273501026943769605906645454851269816759514725340456638922976512726483271644272744008492930125282597595902682072300413706251167948205508278816766531246881090368207825659538929519971798157560790147006473552430833630039884532920761031884789517289671816155304035266250863755423721764653016583392886117574382942448562303298550801320226101222506312293295494413926654014742720571573642434093735724674792020205169969930225827819813160130800252073572525841270036373827679763141080348247834453184608392940600601324493040055811134675921524066069185692051227223857184990089862739945119771885138220215168430246298458776311304431218420943676051927134299650314210673350332280119864537824887838799141455684401072852310865411143709690861261408593780956964263045314581718744417705960109543941488382510390531275524147521938803586050469584
  Width:     1.567986963032091957511281424805116538272768892701202673614342266835786917712324039272158637058068343435873961531906768065059361422357389087253764156964276998744107441489466253063061493446648220908611150065008350334154024679881994516667079141201802718989407489906152769121710130351337244511829885510034912122428610599660020231730166775811381349123319593591721347675170998159202835661172858831966295909447244131441175784210879403880487553151624197206718695522416894705011946848362198686898061055808378466837446709112162796376007366348103549908549808410374984710482906821010861557450858642961681159934837548418520051557388178637724062209133553951062425965992470608705268777721374709620602999510883256706271639464006107556602354343441734459384745539778022712343614452553504102695377339581347644990287838782966381534056677429047233709417303264286837858901505394021215044427007903746747968057106405785873411674050497539310889491902715463477128207414744064264487008966207432912410279686978913240873537109969E-301
  Height:    9.046858803522807376943115796714275461612707987334177667429547757668768011988907628063891622708356117967685045555700046420113754356418532342042939285148851379515969873489172226483383005389166730119625777557815864567264204609475885756455423207430661470858862452759424284453730843615825261193267015715221890877205583702933994735818885192198445874519112274657185887165170166784992675077231393669246542683795749560404761354682126638279730173616667503912062721635629816670137493990472026487378448059842133422410867919999399675806422748764817411645109964338292900360686664813238194262088222586782139356864305718913281819939546152958124072122700102599775924374943467675904234668399452222337545591778932056651862525777323494065202648679375115524674041058548833398687965834199402073042111712382706932023832617779318156077494262582725422536545113574712429588326705466203085504111911105643621072223159132228932036998937400817160437960572201377049777956516458241313303327125426958027408129822638276073307715462583E-302
  Zoom Depth: ~10^300 (2^999)
  Precision:  1139 bits

  Image dimensions 773x446

  Full-width horizontal line near top:
  Row y = 35, columns x = 0..772 (773 pixels)

  For each pixel (x, y), delta_c is computed as:
  norm_x = (x + 0.5) / image_width - 0.5
  norm_y = (y + 0.5) / image_height - 0.5
  delta_c_re = norm_x * viewport_width   (BigFloat → HDRFloat)
  delta_c_im = norm_y * viewport_height  (BigFloat → HDRFloat)

  Both renderers need (from production code):
  1. ReferenceOrbit - computed at viewport center with ReferenceOrbit::compute(&center, max_iterations)
  2. BlaTable - built from orbit with BlaTable::build(&orbit, dc_max)
  3. delta_c for each pixel (computed as above)
  4. max_iterations = 10_000_000
  5. tau_sq (glitch threshold, typically 1e-6)

  Production code paths to call:
  - CPU: fractalwonder_compute::perturbation::compute_pixel_perturbation_hdr_bla()
  - GPU: Need to extract from GPU result buffers after render_row_set()

---

## How Professional Renderers Handle Reference Orbits

### Pre-2021: Multi-Reference with Iterative Refinement

Renderers like Kalles Fraktaler use this approach:

1. Render with single reference at viewport center
2. Detect glitches using Pauldelbrot criterion: `|Z + δz|² < τ² × |Z|²`
3. Select new reference point within glitched region
4. Re-render only glitched pixels with closer reference
5. Repeat until zero glitches (up to 10,000 references for complex images)

**Reference selection methods (Kalles Fraktaler):**
- Original method: Center-based selection
- argmin|z|: Select point with minimum orbit magnitude
- Random: Sometimes works better for pathological cases

**Threshold values (τ) for Pauldelbrot criterion:**
| Value | Behavior |
|-------|----------|
| 10⁻² | Very conservative, catches everything, slow |
| 10⁻³ | Standard default |
| 10⁻⁶ | Moderate |
| 10⁻⁸ | Aggressive, fast but may miss edge cases |

### Post-2021: Rebasing (Zhuoran's Breakthrough)

**Implemented for both CPU and GPU renderers ✅**

Modern approach that avoids glitches rather than detecting/correcting them:

1. Use single reference orbit
2. When `|Z + δz| < |δz|` (delta dominates full value), reset:
   - Set `δz = Z + δz` (absorb reference into delta)
   - Reset reference iteration `m = 0`
   - Continue with same iteration count `n`
3. Only need as many reference orbits as critical points (1 for Mandelbrot)

**Sources:**
- [Deep zoom theory and practice (mathr)](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
- [Deep zoom theory and practice again (mathr)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
- [Imagina (GitHub)](https://github.com/5E-324/Imagina)

---

## Current Implementation Analysis

### 1. Delta Precision (δc and δz)

#### CPU Implementation: ✅ CORRECT

**Location:** `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs`

Uses `HDRComplex` (pair of `HDRFloat`) for delta values:
```rust
let mut dz = HDRComplex::ZERO;
// ...
let dz_mag_sq = dz.norm_sq_hdr();  // Uses HDRFloat for magnitude
```

`HDRFloat` provides:
- ~48-bit mantissa precision (head + tail as f32 pair)
- Extended exponent range (i32 exp, unlimited range)
- Proper saturating arithmetic

#### GPU Implementation: ✅ FIXED

**Location:** `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:463-464`

```wgsl
let x_hdr = hdr_from_f32(f32(col));
let y_hdr = hdr_from_f32(f32(global_row));
let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
```

The `hdr_from_f32` function (lines 199-217) properly normalizes f32 values to HDRFloat format,
ensuring `head` is in the [0.5, 1.0) range as required by HDRFloat arithmetic:

```wgsl
fn hdr_from_f32(val: f32) -> HDRFloat {
    if val == 0.0 { return HDR_ZERO; }

    let bits = bitcast<u32>(val);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    if biased_exp == 0 {
        // Subnormal - use normalize path
        return hdr_normalize(HDRFloat(val, 0.0, 0));
    }

    // Normal: adjust mantissa to [0.5, 1.0) range
    let exp = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let head = bitcast<f32>(new_mantissa_bits | sign);

    return HDRFloat(head, 0.0, exp);
}
```

---

### 2. Rebasing Logic

#### CPU Implementation: ✅ CORRECT

**Location:** `fractalwonder-compute/src/perturbation/pixel_hdr_bla.rs:109-120`

```rust
if z_mag_sq_hdr.sub(&dz_mag_sq).is_negative() {
    dz = HDRComplex { re: z_re, im: z_im };  // δz_new = Z + δz
    drho = HDRComplex { re: rho_re, im: rho_im };
    m = 0;  // Reset reference iteration
    rebase_count += 1;
    continue;  // Don't increment n
}
```

Correctly implements:
1. ✅ Sets `δz = Z + δz` (absorbs reference into delta)
2. ✅ Resets `m = 0` (restarts reference orbit index)
3. ✅ Does NOT reset `n` (iteration count continues)
4. ✅ Uses HDRFloat comparison to avoid f64 underflow

#### GPU Implementation: ✅ CORRECT

**Location:** `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl:572-581`

```wgsl
if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr) {
    dz = z;
    // Also rebase derivative
    drho = HDRComplex(
        hdr_add(der_m_hdr_re, drho.re),
        hdr_add(der_m_hdr_im, drho.im)
    );
    m = 0u;
    continue;
}
```

Same correct logic as CPU.

---

### 3. Reference Orbit Storage

#### Current Approach

**Location:** `fractalwonder-compute/src/perturbation/reference_orbit.rs:44-46`

```rust
// Computed with BigFloat, stored as f64
let orbit_val = (x.to_f64(), y.to_f64());
let der_val = (der_x.to_f64(), der_y.to_f64());
```

**CPU rendering** uses these f64 values directly and converts to HDRFloat on access:
```rust
let z_re = HDRFloat::from_f64(z_m_re).add(&dz.re);
```

**GPU rendering** receives orbit as full HDRFloat representation (12 floats per point):
```wgsl
// Z_re: head, tail, exp; Z_im: head, tail, exp
// Der_re: head, tail, exp; Der_im: head, tail, exp
```

#### Analysis

For orbit values (bounded by escape radius ~256), f64 storage is generally sufficient.

**Potential issue:** When Z_m passes very close to zero during iteration (near critical points),
f64 precision may be insufficient. Professional renderers like Nanoscope use **sparse wide-exponent
storage** where they store a pointer to extended-precision values for iterations where f64 would
underflow.

**Current status:** Not a major concern for typical deep zoom, but could cause issues at extreme
depths (10^1000+) near critical points.

---

### 4. Smooth Coloring

#### Implementation: ✅ CORRECT

**Location:** `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs:26-39`

```rust
pub fn compute_smooth_iteration(data: &MandelbrotData) -> f64 {
    if !data.escaped || data.max_iterations == 0 {
        return data.max_iterations as f64;
    }

    if data.final_z_norm_sq > 1.0 {
        let z_norm_sq = data.final_z_norm_sq as f64;
        let log_z = z_norm_sq.ln() / 2.0;
        let nu = log_z.ln() / std::f64::consts::LN_2;
        data.iterations as f64 + 1.0 - nu
    } else {
        data.iterations as f64
    }
}
```

This is the standard smooth iteration formula: `μ = n + 1 - log₂(ln|z|)`

**Data source:** `final_z_norm_sq` is stored as f32, computed at escape time from HDRFloat magnitude.

---

## Additional GPU Issues

### C1: hdr_to_f32 Exponent Handling: ✅ CORRECT

**Location:** `progressive_iteration.wgsl:140-149`

```wgsl
fn hdr_to_f32(x: HDRFloat) -> f32 {
    if x.head == 0.0 { return 0.0; }
    // Return 0 for underflow, ±infinity for overflow (instead of clamping)
    if x.exp < -149 { return 0.0; }
    if x.exp > 127 {
        return select(f32(-1e38), f32(1e38), x.head > 0.0);
    }
    let mantissa = x.head + x.tail;
    return mantissa * hdr_exp2(x.exp);
}
```

The function now properly returns 0.0 for underflow and ±1e38 for overflow instead of clamping
exponents, which preserves comparison correctness at extreme zoom levels.

### C3: Exponent Overflow Wrapping: ✅ CORRECT

**Location:** `progressive_iteration.wgsl:67-81`

The GPU now uses saturating exponent arithmetic matching CPU behavior:

```wgsl
// Saturating exponent addition (prevents i32 overflow wrapping)
fn saturating_exp_add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    // Detect overflow: if signs of a and b match but differ from sum
    if a > 0 && b > 0 && sum < 0 { return 2147483647; }  // i32::MAX
    if a < 0 && b < 0 && sum > 0 { return -2147483648; } // i32::MIN
    return sum;
}

// Saturating exponent multiplication (for squaring: exp * 2)
fn saturating_exp_mul2(a: i32) -> i32 {
    if a > 1073741823 { return 2147483647; }   // a * 2 would overflow
    if a < -1073741824 { return -2147483648; } // a * 2 would underflow
    return a * 2;
}
```

These functions are used in `hdr_mul` (line 88) and `hdr_square` (line 96) to prevent
exponent overflow at extreme zoom levels.

---

## Summary: Implementation Status

### ✅ COMPLETED Fixes

1. **GPU Un-normalized δc** (`progressive_iteration.wgsl:463-464`)
   - Added `hdr_from_f32()` function (lines 199-217)
   - Pixel coordinates now properly normalized to HDRFloat format
   - **Impact:** Fixed systematic precision loss that was affecting every pixel

2. **GPU hdr_to_f32 exponent handling** (`progressive_iteration.wgsl:140-149`)
   - Returns 0.0 for underflow instead of clamping
   - Returns ±1e38 for overflow
   - **Impact:** Fixed comparisons at extreme zoom

3. **GPU exponent overflow** (`progressive_iteration.wgsl:67-81`)
   - Added `saturating_exp_add()` and `saturating_exp_mul2()` functions
   - Used in `hdr_mul` and `hdr_square` operations
   - **Impact:** Prevents garbage at extreme zoom with high iteration counts

4. **GPU surface normal clamping** (`progressive_iteration.wgsl:569-586`)
   - Added `hdr_complex_direction()` function
   - Computes `u = z × conj(ρ)` in HDRFloat, then extracts direction
   - **Impact:** Fixed 3D lighting artifacts at 10^-281 zoom

5. **CPU surface normal overflow** (`perturbation/mod.rs:30-63`)
   - Changed function to accept `&HDRFloat` instead of `f64`
   - Mirrors GPU's `hdr_complex_direction()` approach
   - **Impact:** CPU now computes valid surface normals at 10^-301+ zoom

### LOW Priority (Future Improvement)

6. **Reference orbit extended precision**
   - Store HDRFloat for orbit values near zero
   - Only needed for extreme depths (10^1000+)

7. **CPU/GPU iteration count discrepancies**
   - ~5% of pixels escape at different iterations
   - May be due to BLA path differences or f64 vs f32 accumulation
   - Causes large surface normal diffs for affected pixels

---

## Why QuadTree Multi-Reference Won't Help

The QuadTree-based multi-reference approach (documented in `docs/archive/2025-01-27-multi-reference-*`)
is designed for **localized glitches** where specific regions have reference orbit exhaustion issues.

The original symptoms (whole-image streaks along gradients) indicated **systematic precision loss**
that affected every pixel uniformly. Adding more reference orbits wouldn't have helped because:

1. The error was in δc computation, not reference selection
2. Every pixel had the same bug, regardless of which reference was used
3. The streaks followed coordinate/iteration gradients, not embedded Julia set patterns

These GPU precision bugs have now been fixed (see Summary above). However, **streaking artifacts persist**
due to a newly identified issue with surface normal computation.

---

## Surface Normal Clamping Bug (2026-01-02) ✅ FIXED

### Root Cause

At deep zoom levels, the derivative ρ = Der_m + δρ can have magnitude ~10^86 or higher.
The GPU stores final ρ values via `hdr_to_f32()` which clamps exp > 127 to ±1e38:

```wgsl
// progressive_iteration.wgsl:543-548
final_values[final_base + 2u] = hdr_to_f32(rho_re);  // Clamps independently!
final_values[final_base + 3u] = hdr_to_f32(rho_im);
```

**Problem:** Independent clamping destroys the ratio between ρ_re and ρ_im.

Example from diagnostic test at 10^-281 zoom:
- Original: ρ = (3.54e85, -1.01e86), ratio = -0.35
- Clamped: ρ = (1e38, -1e38), ratio = -1.0

This corrupts the direction of ρ, which corrupts the surface normal `u = z/ρ`.

### Evidence (Before Fix)

CPU/GPU comparison test at row 350, cols 580-611 (32 pixels):
- Iterations: identical (6786)
- final_z_norm_sq: CPU 613820700 vs GPU 613820600 (diff: 64, ~0.00001%)
- surface_normal: CPU (-0.043, -0.999) vs GPU (-0.472, -0.882) ← **WRONG DIRECTION**

### Fix (Implemented)

Compute surface normal direction **ON THE GPU** before converting to f32. Added
`hdr_complex_direction()` function that scales both components to a common exponent
before normalizing, preserving the ratio even at extreme magnitudes:

```wgsl
// progressive_iteration.wgsl - at escape
// Compute u = z × conj(ρ) in HDRFloat (direction preserved, magnitude irrelevant)
let u_unnorm_re = hdr_add(hdr_mul(z_re_full, rho_re), hdr_mul(z_im_full, rho_im));
let u_unnorm_im = hdr_sub(hdr_mul(z_im_full, rho_re), hdr_mul(z_re_full, rho_im));
let u_unnorm = HDRComplex(u_unnorm_re, u_unnorm_im);

// Get normalized direction - scales to common exponent then normalizes in f32
let surface_normal = hdr_complex_direction(u_unnorm);
final_values[final_base + 2u] = surface_normal.x;  // Unit vector fits in f32
final_values[final_base + 3u] = surface_normal.y;
```

The `hdr_complex_direction()` function finds the max exponent of both components, scales
them to that common exponent (preserving ratio), converts to f32, and normalizes.

### Verification (at 10^-281 zoom)

CPU/GPU comparison test after GPU fix:
- Surface normal: CPU (-0.043, -0.999) vs GPU (-0.043, -0.999) ✓
- Difference: ~1e-8 (f32 precision, acceptable)

---

## CPU Surface Normal Overflow Bug (2026-01-02) ✅ FIXED

### Root Cause

At deeper zoom levels (~10^-301), the CPU surface normal computation was returning `(0.0, 0.0)` for
ALL pixels, even though the GPU was computing valid normals.

**Location:** `fractalwonder-compute/src/perturbation/mod.rs` and `pixel_hdr_bla.rs`

The CPU function `compute_surface_normal_direction` took f64 parameters:

```rust
// OLD - BROKEN at deep zoom
pub(crate) fn compute_surface_normal_direction(
    z_re: f64,
    z_im: f64,
    rho_re: f64,  // ← Overflows to ±infinity at 10^-301 zoom
    rho_im: f64,
) -> (f32, f32)
```

Called from `pixel_hdr_bla.rs`:
```rust
let (sn_re, sn_im) = compute_surface_normal_direction(
    z_re.to_f64(),   // z is fine, |z| > 256 at escape
    z_im.to_f64(),
    rho_re.to_f64(), // ← HDRFloat::to_f64() overflows!
    rho_im.to_f64(),
);
```

At 10^-301 zoom, the derivative ρ has magnitude ~10^100+. When `HDRFloat::to_f64()` is called,
values with exponent > 1024 overflow to infinity. The function then returns `(0.0, 0.0)` because
`rho_norm_sq` is infinite.

### Fix (Implemented)

Changed `compute_surface_normal_direction` to accept `&HDRFloat` parameters and compute
`u = z × conj(ρ)` entirely in HDRFloat, only converting to f32 at the final normalization step:

```rust
// NEW - Works at any zoom depth
pub(crate) fn compute_surface_normal_direction(
    z_re: &HDRFloat,
    z_im: &HDRFloat,
    rho_re: &HDRFloat,
    rho_im: &HDRFloat,
) -> (f32, f32) {
    // Compute u = z × conj(ρ) in HDRFloat
    let u_re = z_re.mul(rho_re).add(&z_im.mul(rho_im));
    let u_im = z_im.mul(rho_re).sub(&z_re.mul(rho_im));

    // Scale both components to common exponent to preserve ratio
    let max_exp = u_re.exp.max(u_im.exp);
    let re_scaled = re_mantissa * 2.0_f64.powi(u_re.exp - max_exp);
    let im_scaled = im_mantissa * 2.0_f64.powi(u_im.exp - max_exp);

    // Normalize to unit vector
    let norm = (re_scaled * re_scaled + im_scaled * im_scaled).sqrt();
    ((re_scaled / norm) as f32, (im_scaled / norm) as f32)
}
```

This mirrors the GPU's `hdr_complex_direction()` approach exactly.

### Verification (at 10^-301 zoom)

CPU/GPU comparison test of 773 pixels at row y=35:

**Before fix:**
```
col 0: normal CPU=(0.0000,0.0000) GPU=(-0.9946,0.1041)  ← CPU returned zero
```

**After fix:**
```
col 0: normal CPU=(-0.9946,0.1041) GPU=(-0.9946,0.1041)  ← Match ✓
```

**Surface normal difference distribution (773 pixels):**

| Percentile | Absolute Diff | Interpretation |
|------------|---------------|----------------|
| Median | 2.91e-6 | f32 precision noise |
| p99 | 1.85 | Large diff (see below) |
| Max | 2.00 | Opposite directions |

- **99% of pixels** have tiny differences (~3e-6) — expected f32 precision
- **~4% of pixels (32)** have large differences (>0.1) caused by iteration count discrepancies

The 32 pixels with large normal differences correlate with the 36 pixels where CPU and GPU
escape at different iterations. Different escape iterations → different z and ρ at escape →
different surface normals. This is expected behavior when iteration counts differ.

---

## Current State: CPU/GPU Comparison at 10^-301 Zoom

Test: 773 pixels, row y=35, full image width

| Metric | Result |
|--------|--------|
| Iteration diffs | 36 pixels (max diff: 224 iterations) |
| Escaped status diffs | 0 |
| Glitched status diffs | 9 |
| Z norm diffs | 773 (f32 precision differences) |
| Surface normal diffs | 772 (99% are ~3e-6, 4% are large due to iter diffs) |

**Key findings:**
1. Surface normal computation now matches between CPU and GPU (median diff: 3e-6)
2. Large surface normal diffs occur only where iteration counts differ
3. Iteration count differences (36 pixels, ~5%) may be due to BLA path differences or
   floating-point accumulation differences between CPU (f64) and GPU (f32)

### Remaining Investigation

If 3D lighting artifacts persist at this zoom level, potential causes to investigate:

1. **Iteration count discrepancies** — CPU and GPU escape at different iterations for ~5% of pixels
2. **BLA coefficient differences** — GPU may take different BLA shortcuts than CPU
3. **Floating-point accumulation** — Small differences compound over ~35,000 iterations
