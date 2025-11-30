# Deep Zoom Rendering: Comprehensive Technical Reference

> **Master Document** - Unified reference for perturbation theory, GPU acceleration, and multi-reference strategies in Fractal Wonder. Supersedes individual research documents.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current Implementation Status](#2-current-implementation-status)
3. [Mathematical Foundation](#3-mathematical-foundation)
4. [Precision Strategy by Zoom Depth](#4-precision-strategy-by-zoom-depth)
5. [Architecture Overview](#5-architecture-overview)
6. [GPU Rendering Pipeline](#6-gpu-rendering-pipeline)
7. [Multi-Reference Strategies](#7-multi-reference-strategies)
8. [BLA Acceleration](#8-bla-acceleration)
9. [Unified Implementation Roadmap](#9-unified-implementation-roadmap)
10. [Testing Strategy](#10-testing-strategy)
11. [References](#11-references)

---

## 1. Executive Summary

### 1.1 Goal

Fractal Wonder aims to render the Mandelbrot set at zoom depths up to 10^2000, matching or exceeding world-class renderers like Kalles Fraktaler, FractalShark, and Mandel Machine.

### 1.2 Core Challenge

Standard floating-point types have limited precision and range:

| Type | Mantissa Bits | Exponent Range | Max Zoom Depth |
|------|---------------|----------------|----------------|
| f32 (GPU native) | 24 bits | ~10^±38 | ~10^7 |
| f64 (CPU native) | 53 bits | ~10^±308 | ~10^15 |
| FloatExp | 53 bits | Unlimited | Unlimited |
| BigFloat | Arbitrary | Unlimited | Unlimited |

At zoom depth 10^100, coordinate deltas are ~10^-100. Standard f32 underflows to zero at ~10^-38.

### 1.3 Solution Architecture

All world-class deep-zoom renderers use the same fundamental approach:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CPU (High Precision)                         │
│  ┌─────────────────────┐    ┌─────────────────────────────────────┐ │
│  │ Reference Orbit     │    │ BLA Table Construction              │ │
│  │ (BigFloat)          │    │ (can be parallel)                   │ │
│  └─────────────────────┘    └─────────────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ Upload reference orbit + BLA table
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       GPU (Massively Parallel)                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │ Delta Iteration (one thread per pixel)                          │ │
│  │ - Uses FloatExp or f32 pairs for extended range                 │ │
│  │ - Applies BLA skipping when valid                               │ │
│  │ - Rebases to avoid precision loss                               │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.4 Key Insight

**Perturbation theory** transforms deep-zoom rendering from impossible to tractable:
- Reference orbit: Computed ONCE at arbitrary precision, stored as f64 (bounded by escape radius)
- Delta iteration: Computed PER-PIXEL using extended-range arithmetic (FloatExp)
- Result: GPU can render at 10^1000+ zoom with proper precision emulation

---

## 2. Current Implementation Status

### 2.1 What's Complete

| Component | Location | Status |
|-----------|----------|--------|
| **GPU Compute Pipeline** | `fractalwonder-gpu/` | ✅ Functional |
| **Delta Iteration Shader** | `shaders/delta_iteration.wgsl` | ✅ f32 only |
| **Reference Orbit (BigFloat)** | `perturbation.rs` | ✅ Complete |
| **Perturbation (3 precision levels)** | `perturbation.rs` | ✅ f64, FloatExp, BigFloat |
| **BLA Table Construction** | `bla.rs` | ✅ Complete |
| **BLA-Enabled CPU Perturbation** | `perturbation.rs` | ✅ 3 functions |
| **Adam7 Progressive Rendering** | `pass.rs`, `stretch.rs`, shader | ✅ Complete |
| **Quadtree Spatial Tracking** | `quadtree.rs` | ✅ Infrastructure |
| **Multi-Reference Orbit Storage** | `worker_pool.rs` | ✅ HashMap exists |
| **Orbit Caching** | `renderer.rs`, `worker_pool.rs` | ✅ By orbit_id |

### 2.2 What's Partially Complete

| Component | Status | Gap |
|-----------|--------|-----|
| **GPU FloatExp** | 🟡 CPU only | Shader uses f32, not FloatExp |
| **GPU BLA** | 🟡 Table exists | Shader doesn't receive BLA table |
| **Multi-Reference GPU** | 🟡 Infrastructure | Not wired to shader |
| **Quadtree Glitch Resolution** | 🟡 Tracking exists | No automatic re-render |

### 2.3 Current GPU Shader Capabilities

```wgsl
// Current Uniforms (from delta_iteration.wgsl)
struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,              // Pauldelbrot threshold
    dc_origin_re: f32,        // δc at top-left pixel
    dc_origin_im: f32,
    dc_step_re: f32,          // Per-pixel δc increment
    dc_step_im: f32,
    adam7_step: u32,          // 0=all, 1-7=specific pass
    _padding: vec2<u32>,
}
```

**Current GPU Limitations:**
- Single reference orbit per render
- f32 precision only (24-bit mantissa, 10^±38 range)
- No BLA iteration skipping
- No extended-exponent arithmetic

### 2.4 Current CPU Perturbation Functions

```rust
// Three precision levels (perturbation.rs)
pub fn compute_pixel_perturbation(orbit, delta_c: (f64, f64), ...) -> MandelbrotData
pub fn compute_pixel_perturbation_floatexp(orbit, delta_c: FloatExpComplex, ...) -> MandelbrotData
pub fn compute_pixel_perturbation_bigfloat(orbit, delta_c: BigComplex, ...) -> MandelbrotData

// Three BLA-enabled versions
pub fn compute_pixel_perturbation_bla(orbit, bla_table, delta_c: (f64, f64), ...) -> MandelbrotData
pub fn compute_pixel_perturbation_floatexp_bla(orbit, bla_table, delta_c: FloatExpComplex, ...) -> MandelbrotData
pub fn compute_pixel_perturbation_bigfloat_bla(orbit, bla_table, delta_c: BigComplex, ...) -> MandelbrotData
```

---

## 3. Mathematical Foundation

### 3.1 The Mandelbrot Iteration

```
z₀ = 0
zₙ₊₁ = zₙ² + c
```

A point `c` is in the Mandelbrot set if `|zₙ| ≤ 2` for all n.

### 3.2 Perturbation Theory Derivation

Let:
- `C` = reference point (computed at high precision)
- `c` = pixel point = `C + δc` where `δc` is small
- `Zₙ` = reference orbit (sequence for C)
- `zₙ` = pixel orbit (sequence for c)
- `δzₙ` = perturbation: `zₙ = Zₙ + δzₙ`

**Derivation:**
```
zₙ₊₁ = zₙ² + c
     = (Zₙ + δzₙ)² + (C + δc)
     = Zₙ² + 2Zₙδzₙ + δzₙ² + C + δc
     = Zₙ₊₁ + 2Zₙδzₙ + δzₙ² + δc

Therefore:
δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc
```

**The Delta Iteration Formula:**
```
δz₀ = 0
δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc
```

### 3.3 Why f64 is Sufficient for Reference Orbit Storage

Reference orbit values `Zₙ` are bounded by the escape radius (typically 2 or 256). Even at 10^1000 zoom:
- The CENTER coordinate C requires ~3300 bits of precision
- But each orbit VALUE `Zₙ` satisfies `|Zₙ| ≤ 256`
- f64 range (10^±308) easily contains values up to 256
- The high precision of C determines WHICH sequence occurs, not the magnitude of individual values

**Evidence:** All production renderers (Kalles Fraktaler, FractalShark, Mandel Machine) store orbits as f64.

### 3.4 Rebasing (Zhuoran, 2021)

When precision loss is detected, reset to the beginning of the same reference orbit:

**Condition:**
```
IF |Zₘ + δzₙ| < |δzₙ|  THEN  rebase
```

**Operation:**
```
δz_new = Zₘ + δzₙ    // Absorb reference value into delta
m = 0                 // Reset reference index to 0
```

**Why it works:** The pixel orbit and reference orbit are both orbits of the same dynamical system. When they diverge, rebasing finds where they align again.

### 3.5 Glitch Detection (Pauldelbrot Criterion)

**Condition:**
```
IF |Zₙ + δzₙ| < τ × |Zₙ|  THEN  pixel is glitched
```

Where `τ` (tau) is typically 10⁻³ (conservative) to 10⁻⁶ (aggressive).

**Why glitches occur:** When `|z| ≈ 0`, the perturbation math loses significant digits. Nearby pixels "stick together" - insufficient precision to distinguish them.

**Visual symptom:** Flat "blobs" of solid color where there should be detail.

---

## 4. Precision Strategy by Zoom Depth

### 4.1 Unified Zoom Depth Classification

| Zoom Exponent | Range Name | Reference Precision | Delta Precision | GPU Strategy |
|---------------|------------|--------------------:|-----------------|--------------|
| < 7 | Shallow | f64 | f32 | Native f32, single reference |
| 7 - 15 | Moderate | f64 | f32 or FloatExp | f32 with frequent rebasing, OR FloatExp |
| 15 - 300 | Deep | BigFloat | FloatExp | FloatExp in shader |
| 300 - 2000+ | Extreme | BigFloat | FloatExp or BigFloat | FloatExp (2×f32) in shader |

### 4.2 The "Moderate Zoom" Challenge (10^7 to 10^15)

This range is problematic for single-reference f32 GPU perturbation:

| Condition | Problem |
|-----------|---------|
| δc values are "medium-sized" | Not small enough for well-conditioned perturbation |
| Rebasing triggers frequently | Different pixels rebase at different iterations |
| f32 loses significance | 24-bit mantissa insufficient for `Z + δz` when both are similar magnitude |

**Symptom:** "Curved mosaic tiles" following iteration contours - sharp color discontinuities within iso-iteration bands.

### 4.3 Solutions for Moderate Zoom

**Option A: GPU FloatExp (Recommended)**
- Implement FloatExp type in WGSL shader
- Provides unlimited range with 24-bit precision
- Works with single reference
- Already implemented in CPU code

**Option B: Multi-Reference Tiling**
- Divide screen into tiles (64×64 or 128×128)
- Compute local reference orbit per tile
- δc values stay small relative to local reference
- More complex, but works with existing f32 shader

**Option C: Double-Double Arithmetic**
- Use two f32 values for ~48-bit mantissa
- Faster than f64 on consumer GPUs
- Can be combined with extended exponent

**Recommendation:** Implement FloatExp first (simpler, single reference). Add multi-reference tiling only if FloatExp proves insufficient for specific edge cases.

### 4.4 f64 Reference Orbit: When It's Insufficient

At extreme zoom (>10^300), reference orbit VALUES near zero can underflow:

```
Z_n ≈ 10^-400  →  Underflows in f64
```

**Solution (Nanoscope approach):** Store f64 normally, but keep FloatExp copies only for iterations where `Zₙ` underflows. This is a sparse/lazy optimization.

---

## 5. Architecture Overview

### 5.1 Current Fractal Wonder Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            Main Thread                                   │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ ParallelRenderer                                                  │   │
│  │  - Coordinates GPU and CPU rendering                             │   │
│  │  - Manages Adam7 progressive passes                              │   │
│  │  - Handles colorization pipeline                                 │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                    │                              │                      │
│                    ▼                              ▼                      │
│  ┌────────────────────────────┐    ┌────────────────────────────────┐   │
│  │ GpuRenderer (wgpu)         │    │ WorkerPool                     │   │
│  │  - Compute shader dispatch │    │  - WASM workers                │   │
│  │  - Buffer management       │    │  - Tile distribution           │   │
│  │  - Orbit caching           │    │  - Orbit broadcast             │   │
│  │  - Adam7 accumulation      │    │  - Quadtree tracking           │   │
│  └────────────────────────────┘    └────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Data Flow: GPU Rendering Path

```
1. User navigates to new viewport
2. ParallelRenderer checks zoom depth, selects strategy
3. If GPU path:
   a. CPU computes reference orbit at center (BigFloat → f64)
   b. CPU optionally builds BLA table
   c. GPU receives: orbit, dc_origin, dc_step, uniforms
   d. GPU dispatches Adam7 passes (1-7)
   e. Each pass: compute shader runs, results accumulated
   f. CPU reads back iteration counts, glitch flags, z_norm_sq
   g. Colorization applied
   h. Display updated
```

### 5.3 Data Flow: CPU Perturbation Path

```
1. WorkerPool divides canvas into tiles
2. For each tile:
   a. Convert tile to fractal-space viewport
   b. Worker computes pixels using perturbation
   c. Precision selected based on zoom (f64/FloatExp/BigFloat)
   d. BLA acceleration applied if enabled
   e. Results sent back via message
3. Tiles colorized as they complete
4. Progress callbacks fired
```

### 5.4 Existing Multi-Reference Infrastructure

The codebase has infrastructure for multi-reference rendering that's not fully utilized:

```rust
// In WorkerPool (worker_pool.rs)
pub struct WorkerPool {
    quadtree: Option<QuadtreeCell>,              // Spatial partitioning
    cell_orbits: HashMap<Bounds, ReferenceOrbit>, // Per-cell orbits
    cell_orbit_ids: HashMap<Bounds, u32>,        // For worker distribution
    cell_orbit_confirmations: HashMap<u32, HashSet<usize>>, // Tracking
}

// In QuadtreeCell (quadtree.rs)
pub struct QuadtreeCell {
    pub bounds: Bounds,                          // [x, y, width, height]
    pub depth: u32,                              // 0 to MAX_DEPTH (10)
    pub children: Option<Box<[QuadtreeCell; 4]>>,
}
```

This infrastructure can be leveraged for multi-reference GPU rendering without building from scratch.

---

## 6. GPU Rendering Pipeline

### 6.1 Current Shader Implementation

**File:** `fractalwonder-gpu/src/shaders/delta_iteration.wgsl`

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // 1. Bounds check
    if gid.x >= uniforms.width || gid.y >= uniforms.height { return; }

    // 2. Adam7 pass filtering
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        return;
    }

    // 3. Compute δc for this pixel
    let dc = vec2<f32>(
        uniforms.dc_origin_re + f32(gid.x) * uniforms.dc_step_re,
        uniforms.dc_origin_im + f32(gid.y) * uniforms.dc_step_im
    );

    // 4. Delta iteration loop
    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let Z = reference_orbit[m % orbit_len];
        let z = Z + dz;

        // Escape check
        if dot(z, z) > uniforms.escape_radius_sq { ... }

        // Glitch detection (Pauldelbrot)
        if dot(z, z) < uniforms.tau_sq * dot(Z, Z) { glitched = true; }

        // Rebase check
        if dot(z, z) < dot(dz, dz) { dz = z; m = 0u; continue; }

        // Delta iteration: δz' = 2Zδz + δz² + δc
        dz = 2.0 * complex_mul(Z, dz) + complex_mul(dz, dz) + dc;
        m += 1u;
    }

    // 5. Store results
    results[idx] = n;
    glitch_flags[idx] = glitched;
    z_norm_sq[idx] = dot(z, z);
}
```

### 6.2 FloatExp Type for WGSL

**Structure:**
```wgsl
struct FloatExp {
    mantissa: f32,   // Normalized: 0.5 ≤ |mantissa| < 1.0
    exp: i32,        // Extended exponent (base 2)
};
// Value = mantissa × 2^exp

struct ComplexFloatExp {
    re: FloatExp,
    im: FloatExp,
};
```

**Key Operations:**
```wgsl
fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    var result = FloatExp(a.mantissa * b.mantissa, a.exp + b.exp);
    return fe_normalize(result);
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    let exp_diff = a.exp - b.exp;
    if exp_diff > 24 { return a; }  // b is negligible
    if exp_diff < -24 { return b; } // a is negligible

    // Align exponents and add
    let scaled_b = ldexp(b.mantissa, -exp_diff);
    return fe_normalize(FloatExp(a.mantissa + scaled_b, a.exp));
}

fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.mantissa == 0.0 { return FloatExp(0.0, 0); }
    var e: i32;
    let m = frexp(x.mantissa, &e);
    return FloatExp(m, x.exp + e);
}
```

### 6.3 Double-Double Arithmetic (2×f32)

For ~48-bit precision without f64:

```wgsl
struct FloatExp2x32 {
    hi: f32,         // High part of mantissa
    lo: f32,         // Low part of mantissa (error term)
    exp: i32,        // Extended exponent
};
// Provides ~48-bit mantissa with unlimited range
```

**Knuth's TwoSum (error-free addition):**
```wgsl
fn two_sum(a: f32, b: f32) -> vec2<f32> {
    let s = a + b;
    let v = s - a;
    let e = (a - (s - v)) + (b - v);
    return vec2<f32>(s, e);
}
```

**When to use:**
- Consumer GPUs have 1:64 f64:f32 performance ratio
- 2×f32 is ~3× slower than f32, but ~20× faster than f64
- Use when 24-bit FloatExp precision is insufficient

### 6.4 Adam7 Progressive Rendering

**Pass Distribution (8×8 pattern):**
```
1 6 4 6 2 6 4 6
7 7 7 7 7 7 7 7
5 6 5 6 5 6 5 6
7 7 7 7 7 7 7 7
3 6 4 6 3 6 4 6
7 7 7 7 7 7 7 7
5 6 5 6 5 6 5 6
7 7 7 7 7 7 7 7
```

**Coverage per pass:**
| Pass | Pixels | Cumulative |
|------|--------|------------|
| 1 | 1/64 (1.56%) | 1.56% |
| 2 | 1/64 (1.56%) | 3.12% |
| 3 | 2/64 (3.12%) | 6.25% |
| 4 | 4/64 (6.25%) | 12.5% |
| 5 | 8/64 (12.5%) | 25% |
| 6 | 16/64 (25%) | 50% |
| 7 | 32/64 (50%) | 100% |

**Accumulator:** `Adam7Accumulator` merges pass results, fills gaps from neighbors for progressive display.

---

## 7. Multi-Reference Strategies

### 7.1 When Multi-Reference is Needed

Even with FloatExp and rebasing, some scenarios benefit from multiple references:

1. **Reference escapes early:** All pixels need constant rebasing
2. **Heterogeneous iteration counts:** Wide variance across image
3. **Complex boundaries:** Single reference insufficient for all regions

### 7.2 Strategy A: Quadtree-Based (Existing Infrastructure)

Leverage the existing `QuadtreeCell` and `cell_orbits` infrastructure:

```
1. Start with single reference at center
2. Detect glitched regions via Pauldelbrot criterion
3. Subdivide quadtree in glitched areas
4. Compute new reference at each cell center
5. Re-render only glitched cells with local reference
6. Iterate until glitch-free
```

**Advantages:**
- Adaptive: Fine cells only where needed
- Memory efficient: Fewer orbits than fixed tiling
- Infrastructure already exists

### 7.3 Strategy B: Fixed Tile Grid

Divide screen into fixed-size tiles, each with local reference:

```
1. Divide screen into tiles (64×64 or 128×128)
2. Compute reference orbit at each tile center (CPU, BigFloat)
3. Pack all orbits into single GPU buffer
4. Shader looks up which tile each pixel belongs to
5. Uses tile's local orbit for perturbation
```

**GPU Data Structures:**
```wgsl
struct TileInfo {
    orbit_offset: u32,      // Index into packed_orbits
    orbit_len: u32,         // This tile's orbit length
    dc_origin_re: f32,      // δc at tile's top-left (relative to tile center)
    dc_origin_im: f32,
};

@group(0) @binding(1) var<storage, read> packed_orbits: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read> tile_infos: array<TileInfo>;
```

**Trade-offs:**

| Fixed Tiles | Adaptive Quadtree |
|-------------|-------------------|
| Simpler implementation | More complex |
| Predictable memory | Variable memory |
| May over-compute (unnecessary tiles) | Computes only where needed |
| Good for moderate zoom | Good for all zoom depths |

### 7.4 Recommended Approach

1. **First:** Implement GPU FloatExp (Increment 3 in roadmap)
2. **If insufficient:** Add quadtree-based multi-reference (leverages existing code)
3. **If still insufficient:** Add fixed tiling as fallback for extreme cases

This avoids duplicating infrastructure and builds on what exists.

---

## 8. BLA Acceleration

### 8.1 Mathematical Foundation

When `|δz²| ≪ |2Zδz|`, delta iteration becomes approximately linear:

```
δzₙ₊₁ ≈ 2Zₙδzₙ + δc    (dropping δz² term)
```

This allows skipping multiple iterations:
```
δzₘ₊ₗ = Aₗ·δzₘ + Bₗ·δc
```

Where (A, B, l) define a BLA that skips l iterations.

### 8.2 Current BLA Implementation

**File:** `fractalwonder-compute/src/bla.rs`

```rust
pub struct BlaEntry {
    pub a_re: f64, pub a_im: f64,   // Complex A coefficient
    pub b_re: f64, pub b_im: f64,   // Complex B coefficient
    pub l: u32,                      // Iterations skipped
    pub r_sq: f64,                   // Validity radius squared
}

pub struct BlaTable {
    pub entries: Vec<BlaEntry>,      // Binary tree
    pub level_offsets: Vec<usize>,   // Level boundaries
    pub num_levels: usize,
    pub dc_max: f64,                 // Maximum |δc| for this table
}
```

**Algorithm:**
1. Level 0: Create M single-iteration BLAs from orbit
2. Higher levels: Merge pairs `(A_merged = A_y × A_x, B_merged = A_y × B_x + B_y)`
3. Validity radius propagated: `r_merged = min(r_x, (r_y - |B_x|×|δc|_max) / |A_x|)`

### 8.3 BLA Table for GPU

**GPU Buffer Layout:**
```wgsl
struct BlaEntry {
    A_re: f32, A_im: f32,           // Linear coefficient
    B_re: f32, B_im: f32,           // Constant coefficient
    validity_radius_sq: f32,        // When |δz|² < r², BLA is valid
    skip_length: u32,               // Iterations to skip
};

@group(0) @binding(3) var<storage, read> bla_table: array<BlaEntry>;
@group(0) @binding(4) var<storage, read> bla_level_offsets: array<u32>;
```

**Shader Integration:**
```wgsl
fn try_bla_skip(m: u32, dz_sq: f32, dc: vec2<f32>) -> BlaResult {
    // Find largest valid BLA at reference index m
    for (var level = num_levels - 1; level >= 0; level--) {
        let entry = get_bla_entry(m, level);
        if dz_sq < entry.validity_radius_sq {
            // Apply BLA: δz' = A × δz + B × δc
            let new_dz = complex_mul(entry.A, dz) + complex_mul(entry.B, dc);
            return BlaResult(true, new_dz, entry.skip_length);
        }
    }
    return BlaResult(false, dz, 0u);
}
```

### 8.4 Performance Impact

| Scenario | Without BLA | With BLA | Speedup |
|----------|-------------|----------|---------|
| 10K iterations | 10K ops | ~500 ops (avg) | ~20× |
| 100K iterations | 100K ops | ~1K ops (avg) | ~100× |
| 1M iterations | 1M ops | ~2K ops (avg) | ~500× |

BLA provides O(log n) iteration complexity instead of O(n).

---

## 9. Unified Implementation Roadmap

This roadmap supersedes individual increment lists from other documents. Each increment is self-contained and shippable.

### Phase 1: Foundation (Complete)

These increments are already implemented in the codebase.

#### ✅ Increment 1.1: Reference Orbit Computation
- BigFloat computation of reference orbit
- Storage as f64 (bounded by escape radius)
- Escaped-at tracking for orbit exhaustion

#### ✅ Increment 1.2: CPU Perturbation (3 Precision Levels)
- f64 delta iteration
- FloatExp delta iteration
- BigFloat delta iteration
- Pauldelbrot glitch detection
- Rebasing logic

#### ✅ Increment 1.3: BLA Table Construction
- Binary tree structure
- Level merging algorithm
- Validity radius computation
- Three BLA-enabled perturbation functions

#### ✅ Increment 1.4: GPU Infrastructure
- wgpu device/queue initialization
- Compute pipeline setup
- Buffer management (uniforms, orbit, results)
- Workgroup dispatch

#### ✅ Increment 1.5: GPU Delta Iteration (f32)
- Basic delta iteration shader
- Escape check, glitch detection, rebasing
- Adam7 pass filtering
- Results readback

#### ✅ Increment 1.6: Adam7 Progressive Rendering
- Pass enumeration and coverage
- Adam7Accumulator for merging
- Gap filling for progressive display
- Integration with GPU renderer

---

### Phase 2: GPU Precision Extension

#### Increment 2.1: GPU FloatExp Type
**Deliverable:** GPU-accelerated rendering up to ~10^300 zoom.

**Implementation:**
1. Add FloatExp struct to WGSL shader
2. Implement fe_add, fe_sub, fe_mul, fe_norm_sq, fe_normalize
3. Implement ComplexFloatExp operations
4. Update delta iteration loop to use FloatExp for δz and δc
5. Keep reference orbit as vec2<f32> (bounded values)

**New Uniforms:**
```wgsl
struct Uniforms {
    // ... existing fields ...
    dc_origin_re_mantissa: f32,
    dc_origin_re_exp: i32,
    dc_origin_im_mantissa: f32,
    dc_origin_im_exp: i32,
    dc_step_re_mantissa: f32,
    dc_step_re_exp: i32,
    dc_step_im_mantissa: f32,
    dc_step_im_exp: i32,
    use_floatexp: u32,        // Toggle for precision mode
};
```

**Test Strategy:**
1. FloatExp operations match CPU FloatExp within 1 ULP
2. Iteration counts at 10^100 zoom match CPU within ±1
3. No delta underflow at any tested zoom depth
4. Performance: GPU FloatExp ≥5× faster than CPU FloatExp

**Acceptance Criteria:**
- Renders at 10^100, 10^200 zoom match CPU reference
- No artifacts from precision loss
- Documented speedup over CPU

---

#### Increment 2.2: Robust GPU Rebasing
**Deliverable:** Correct rebasing preventing precision loss at all zoom depths.

**Implementation:**
1. Ensure rebase condition uses FloatExp magnitude comparison
2. After rebase, δz absorbs Z correctly in FloatExp
3. Reference index m resets to 0
4. Continue iteration with new δz

**Test Strategy:**
1. Create test case where |z| < |δz| at specific iteration
2. Verify GPU triggers rebase at exact iteration
3. Post-rebase iteration counts match CPU

**Acceptance Criteria:**
- Rebase-induced glitches reduced by >90%
- Iteration counts match CPU at 10^100, 10^200, 10^300 zoom
- No infinite rebase loops

---

#### Increment 2.3: GPU BLA Integration
**Deliverable:** BLA acceleration on GPU for O(log n) iteration complexity.

**Implementation:**
1. Add BLA table buffer to GPU pipeline
2. Add BLA level offsets buffer
3. Implement `find_valid_bla()` in shader
4. Integrate BLA skip into main iteration loop
5. Fallback to per-iteration when BLA invalid

**GPU Buffers:**
```rust
// New buffers in GpuBuffers
bla_table: wgpu::Buffer,           // BLA entries
bla_level_offsets: wgpu::Buffer,   // Level boundaries
bla_num_levels: u32,               // For shader
```

**Test Strategy:**
1. Iteration counts identical with/without BLA
2. Performance scales sub-linearly with max_iterations
3. BLA never applied outside validity radius

**Acceptance Criteria:**
- Identical results with/without BLA
- ≥10× speedup at max_iter = 100,000
- No visual artifacts from BLA approximation

---

### Phase 3: Multi-Reference (If Needed)

These increments should only be implemented if Phase 2 proves insufficient for specific use cases.

#### Increment 3.1: Quadtree-Based Glitch Resolution
**Deliverable:** Automatic glitch correction using existing quadtree infrastructure.

**Implementation:**
1. After GPU render, collect glitched pixel locations
2. Map glitched pixels to quadtree cells
3. For cells with high glitch density:
   - Compute new reference at cell center
   - Re-render cell with local reference
4. Iterate until glitch count below threshold

**Leverages:**
- Existing `QuadtreeCell` structure
- Existing `cell_orbits` HashMap
- Existing `cell_orbit_ids` tracking

**Test Strategy:**
1. Known glitch-prone coordinates render without glitches
2. Multi-reference loop terminates (no infinite subdivision)
3. Only glitched regions re-rendered (not entire image)

**Acceptance Criteria:**
- Zero glitched pixels in final output
- Reasonable reference count (<100 for typical renders)
- Performance within 2× of single-reference for non-pathological cases

---

#### Increment 3.2: Fixed Tile Multi-Reference
**Deliverable:** GPU multi-reference for moderate zoom edge cases.

**Implementation:**
1. Divide screen into fixed tiles (64×64 or 128×128)
2. Compute reference orbit at each tile center (parallel via workers)
3. Pack all orbits into single GPU buffer
4. Add TileInfo buffer with per-tile metadata
5. Shader looks up tile for each pixel, uses local orbit

**When to use:**
- Moderate zoom (10^7 to 10^15) where FloatExp + rebasing still shows artifacts
- Heterogeneous iteration counts across image
- User-configurable option

**Test Strategy:**
1. No tile boundary artifacts (smooth coloring across tiles)
2. Iteration counts match CPU at tile centers
3. Memory usage within GPU limits

**Acceptance Criteria:**
- No mosaic artifacts at 10^8, 10^10, 10^12 zoom
- Clean integration with Adam7 progressive rendering
- Documented tile size recommendations

---

### Phase 4: Extreme Precision (Future)

#### Increment 4.1: FloatExp 2×f32 (Double-Double)
**Deliverable:** ~48-bit precision for extreme accuracy requirements.

**Implementation:**
1. Implement FloatExp2x32 type in WGSL
2. Implement Knuth's TwoSum and double-double arithmetic
3. Update delta iteration to use 2×f32 when needed
4. Add precision mode selection uniform

**When to use:**
- Zoom depths >10^300 where 24-bit FloatExp shows artifacts
- Coordinates near Misiurewicz points (precision-sensitive)
- High-iteration renders where errors accumulate

**Test Strategy:**
1. 2×f32 iteration counts match BigFloat at 10^500 zoom
2. Performance overhead <3× compared to f32 FloatExp
3. Graceful fallback to f32 FloatExp when 2×f32 not needed

---

#### Increment 4.2: Reference Orbit Compression
**Deliverable:** Support for 100M+ iterations within GPU memory.

**Implementation:**
1. Store keyframe orbit values at intervals
2. Reconstruct intermediate values on-the-fly in shader
3. Configurable compression ratio (trade memory for speed)

**When to use:**
- Iteration counts exceeding GPU memory for uncompressed orbit
- Example: 100M iterations × 8 bytes = 800MB (may exceed limits)

---

### Summary Table

| Increment | Zoom Depth | Key Capability | Status |
|-----------|------------|----------------|--------|
| 1.1-1.6 | ~10^7 (GPU), unlimited (CPU) | Foundation | ✅ Complete |
| 2.1 | ~10^300 | GPU FloatExp | 🔲 Next |
| 2.2 | ~10^300 | Robust GPU Rebasing | 🔲 Next |
| 2.3 | ~10^300 | GPU BLA | 🔲 Next |
| 3.1 | ~10^300 | Quadtree Glitch Resolution | 🔲 If needed |
| 3.2 | 10^7-10^15 | Fixed Tile Multi-Reference | 🔲 If needed |
| 4.1 | ~10^2000 | FloatExp 2×f32 | 🔲 Future |
| 4.2 | ~10^2000 | Orbit Compression | 🔲 Future |

---

## 10. Testing Strategy

### 10.1 Mathematical Validation

**Principle:** Tests verify mathematical invariants, not just "produces output."

**Core Invariants:**
1. `δz' = 2Zδz + δz² + δc` matches direct BigFloat computation
2. Rebasing preserves pixel orbit: `z_pixel(n) = Z_ref(0) + δz_new`
3. Pauldelbrot criterion catches precision loss BEFORE visual artifacts
4. BLA skip produces identical results to full iteration
5. GPU and CPU produce matching iteration counts (within ±1 due to precision)

### 10.2 Test Coordinates

| Location | Zoom | Expected Behavior |
|----------|------|-------------------|
| (-0.75, 0.1) | 10^8 | Boundary region, tests rebasing |
| (-0.5, 0.5) | 10^10 | Mixed dynamics, tests precision |
| (-1.25, 0.0) | 10^6 | Antenna region, high iterations |
| (-0.743643887037151, 0.131825904205330) | 10^14 | Seahorse valley |
| (-1.749999999999..., 0.0) | 10^50 | Period-2 minibrot boundary |

### 10.3 Visual Quality Tests

1. **Smooth coloring continuity:** No sharp jumps within iso-iteration bands
2. **Histogram coloring:** Smooth distribution, no quantization artifacts
3. **Tile/cell boundary invisibility:** No grid patterns from multi-reference
4. **Progressive rendering:** Each Adam7 pass improves detail, no regressions

### 10.4 Performance Benchmarks

| Metric | Target |
|--------|--------|
| GPU f32 vs CPU f64 | ≥50× speedup |
| GPU FloatExp vs CPU FloatExp | ≥10× speedup |
| GPU with BLA vs without | ≥10× at 100K iterations |
| Adam7 Pass 1 latency | <50ms for 4K |
| Full render (4K, 10K iter) | <2s |

### 10.5 Cross-Validation

Compare against known-good renderers:
- **Kalles Fraktaler:** Iteration counts at documented coordinates
- **Mandel Machine:** Deep zoom reference images
- **FractalShark:** Performance benchmarks

---

## 11. References

### Primary Sources

1. **K.I. Martin** - SuperFractalThing and sft_maths.pdf (2013)
   - Original perturbation theory popularization

2. **Claude Heiland-Allen (mathr)**
   - [Deep Zoom Theory and Practice](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
   - [Deep Zoom Theory and Practice (Again)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
   - Kalles Fraktaler 2+, Fraktaler 3

3. **Pauldelbrot** - Glitch Detection Criterion (2014)
   - Fractal Forums: `|z| < τ|Z|` criterion

4. **Zhuoran** - Rebasing and BLA (2021)
   - Fractal Forums contributions
   - Implemented in Imagina renderer

### Software References

5. **FractalShark** - [GitHub](https://github.com/mattsaccount364/FractalShark)
   - 2×f32 type, reference orbit compression, CUDA implementation

6. **Kalles Fraktaler 2+** - [mathr.co.uk](https://mathr.co.uk/kf/kf.html)
   - Multi-reference, BLA, OpenCL acceleration

7. **DeepDrill** - [Documentation](https://dirkwhoffmann.github.io/DeepDrill/)
   - Educational codebase for perturbation theory

### WebGPU Resources

8. **WebGPU Specification** - [W3C WGSL Spec](https://www.w3.org/TR/WGSL/)
9. **wgpu** - [wgpu.rs](https://wgpu.rs/)
10. **WebGPU Fundamentals** - [Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)

### Additional Resources

11. **Phil Thompson**
    - [Perturbation Theory](https://philthompson.me/2022/Perturbation-Theory-and-the-Mandelbrot-set.html)
    - [BLA Explanation](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html)

12. **Precision Emulation**
    - [Godot: Emulating Double on GPU](https://godotengine.org/article/emulating-double-precision-gpu-render-large-worlds/)
    - [metal-float64](https://github.com/philipturner/metal-float64)

---

*Document created: November 2025*
*Supersedes: perturbation-theory.md, webgpu-rendering.md, perturbation-hybrid.md*
*Based on Fractal Wonder codebase analysis and research from fractal community*
