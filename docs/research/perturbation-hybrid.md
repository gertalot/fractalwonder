# Hybrid Rendering for Mid-Zoom Depths (10^4 to 10^15)

> **Research Document** - Comprehensive analysis of the "mid-zoom dead zone" and tile-based hybrid rendering strategies for world-class Mandelbrot exploration.

---

## 1. Executive Summary

### 1.1 The Problem

Between zoom depths of approximately **10^4 and 10^15**, single-reference GPU perturbation produces severe visual artifacts:

- **Curved mosaic tiles** following iteration contours
- **Sharp color discontinuities** within iso-iteration bands
- **Broken smooth iteration coloring** and histogram normalization

This is the **"worst possible" zone** for GPU-only FP32 perturbation:

| Condition | Why It's Bad |
|-----------|--------------|
| Too deep for direct FP32 iteration | Coordinates require more than 24-bit precision |
| Not deep enough for perturbation to be well-conditioned | δc values are "medium-sized", not truly small |
| Reference–delta magnitudes in awkward range | GPU precision loses significance, but deltas don't dominate |

### 1.2 The Solution

**No modern renderer uses a single reference orbit in this range.**

Professional fractal renderers solve this with hybrid strategies:

1. **Tile-based local reference orbits** - Multiple references, one per tile
2. **Series approximation** - High-order Taylor series for stability
3. **Double-double arithmetic** - ~48-bit precision on GPU
4. **Split computation domain** - Different algorithms per zoom range

### 1.3 Recommendation for Fractal Wonder

Implement **tile-based local reference orbits** with GPU perturbation:

1. Divide screen into tiles (64×64 or 128×128 pixels)
2. Compute a local reference orbit at each tile center (CPU, BigFloat)
3. GPU computes perturbation only for pixels within each tile
4. δc values stay small relative to local reference → stable computation

This is the most common solution used by Kalles Fraktaler, SuperFractalThing successors, and other production renderers.

---

## 2. Root Cause Analysis

### 2.1 Why Single-Reference Perturbation Fails at Mid-Zoom

The perturbation formula is:

```
δzₙ₊₁ = 2Zₙδzₙ + δzₙ² + δc
```

This works when δc is **truly small** relative to the reference. At deep zoom (>10^15):

- δc ≈ 10^-15 or smaller
- δz stays tiny throughout iteration
- The linear term `2Zₙδzₙ` dominates
- Perturbation is well-conditioned

At mid-zoom (10^4 to 10^15):

- δc ≈ 10^-4 to 10^-15 (not small enough)
- δz can grow to significant size
- Rebasing triggers frequently (different pixels rebase at different times)
- F32 precision (24 bits) loses significant digits in `Z + δz` computation

### 2.2 The Rebasing Discontinuity Problem

When rebasing occurs in the GPU shader:

```wgsl
if z_sq < dz_sq {
    dz = z;      // Absorb Z into delta
    m = 0u;      // Reset reference index
    continue;
}
```

Different pixels with the **same iteration count** can have **different m values** at escape due to different rebasing histories. This causes:

1. **Different Z_m values** at final iteration
2. **Different floating-point computation paths** for `z = Z_m + dz`
3. **Discontinuous final_z_norm_sq** values for smooth coloring
4. **Visible "tile" boundaries** where m changes discretely

### 2.3 Current Implementation Issues

The current Fractal Wonder GPU renderer has several problems in this zoom range:

#### Issue 1: Single Global Reference

```rust
// worker_pool.rs - uses viewport CENTER as sole reference
let c_ref_json = serde_json::to_string(&viewport.center).unwrap_or_default();
```

All pixels use the same reference orbit, regardless of distance from center.

#### Issue 2: F32 Precision Loss

The GPU shader uses `vec2<f32>` for both orbit values and deltas:

```wgsl
let Z = reference_orbit[m % orbit_len];  // f32
let z = Z + dz;                           // f32 + f32 addition
```

At mid-zoom with large δc values spanning the viewport, f32 precision is insufficient.

#### Issue 3: Missing `reference_escaped` Uniform

The shader expects:
```wgsl
struct Uniforms {
    // ...
    reference_escaped: u32,   // Expected by shader
    _padding: u32,
}
```

But the Rust `Uniforms` struct omits this field, causing incorrect glitch detection.

### 2.4 Symptom Analysis

The **"curved mosaic tiles following iteration contours"** pattern indicates:

1. **Tiles correspond to regions with same m value at escape**
2. **Boundaries are sharp** because m is discrete (changes by ±1)
3. **Boundaries are orthogonal to iteration contours** because m changes in the direction where n is constant
4. **Within tiles, colors are continuous** because same Z_m is used

This is a classic signature of rebasing-induced discontinuities in perturbation rendering.

---

## 3. How Professional Renderers Solve This

### 3.1 Tile-Based Multi-Reference (Most Common)

Used by: Kalles Fraktaler, SuperFractalThing successors, UltraFractal deep zoom plugins

**Algorithm:**

```
1. Divide screen into tiles (e.g., 64×64 or 128×128 pixels)
2. For each tile:
   a. Compute tile center in fractal coordinates
   b. Compute reference orbit at tile center (CPU, arbitrary precision)
   c. Compute perturbation for all pixels in tile using LOCAL reference
3. Each tile's δc values are small relative to its local reference
4. Rebasing becomes rare or unnecessary
```

**Why It Works:**

At 10^10 zoom with 1920×1080 viewport divided into 64×64 tiles (30×17 = 510 tiles):

| Without Tiling | With Tiling |
|----------------|-------------|
| max δc ≈ viewport_width/2 ≈ 10^-10 | max δc ≈ tile_width/2 ≈ 10^-10 / 30 ≈ 3×10^-12 |
| δc spans ~20 bits of magnitude | δc spans ~5 bits of magnitude |
| Frequent rebasing, precision loss | Rare rebasing, stable computation |

### 3.2 Series Approximation (Taylor Series)

Used by: Many precision-focused renderers

**Algorithm:**

The delta iteration generates a polynomial in δc:

```
δzₙ = Aₙδc + Bₙδc² + Cₙδc³ + O(δc⁴)
```

Pre-compute coefficients A, B, C for the first N iterations. Then for each pixel:

```
δzₙ = Aₙδc + Bₙδc² + Cₙδc³  // Direct evaluation, no iteration
```

**Why It Works:**

- Series approximation is **extremely stable** in mid-zoom range
- Error decreases rapidly with series order
- Can skip hundreds of iterations with one polynomial evaluation
- No rebasing needed during approximated iterations

**Limitations:**

- Accuracy degrades after many iterations; requires "probe points" to validate
- More complex implementation than tile-based approach
- Best combined with perturbation for later iterations

### 3.3 Double-Double (Float-Float) Arithmetic

Used by: FractalShark, homebrew GPU deep zooms

**Data Structure:**

```
struct DoubleDouble {
    hi: f32,  // High part of mantissa
    lo: f32,  // Low part of mantissa (error term)
}
// Value = hi + lo, where |lo| ≤ ulp(hi)/2
// Provides ~48-52 bits of effective precision
```

**Why It Works:**

- Much faster than native f64 on consumer GPUs (1:64 f64:f32 ratio)
- Provides ~48 bits of precision vs 24 bits for f32
- Handles 10^4 to 10^8 with low noise
- Can be combined with extended exponent for deep zoom

**Limitations:**

- More complex arithmetic operations
- ~3x slower than pure f32
- Still benefits from tile-based references at deeper zoom

### 3.4 Split Computation Domain

The **clean engineering solution** used by all stable deep-zoom renderers:

| Zoom Range | Algorithm |
|------------|-----------|
| < 10^4 | Direct float or double iteration |
| 10^4 to 10^15 | Tile-based arbitrary-precision reference + GPU perturbation |
| > 10^15 | Pure perturbation with single global reference (GPU works flawlessly) |

This is what Fractal Wonder should implement.

---

## 4. Detailed Solution: Tile-Based Local References

### 4.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Fractal Wonder                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    CPU (Rust/WASM Workers)                          │ │
│  │                                                                      │ │
│  │  For each tile:                                                      │ │
│  │    1. Compute tile center in fractal coords (BigFloat)              │ │
│  │    2. Compute reference orbit at tile center (BigFloat → f32)       │ │
│  │    3. Pack orbit into tile's section of orbit buffer                │ │
│  │                                                                      │ │
│  │  Output: Packed orbit buffer + tile metadata                         │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                    │                                     │
│                                    ▼                                     │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    GPU (wgpu/WebGPU)                                │ │
│  │                                                                      │ │
│  │  Inputs:                                                             │ │
│  │    - packed_orbits: array<vec2<f32>>     // All tile orbits         │ │
│  │    - tile_infos: array<TileInfo>         // Per-tile metadata       │ │
│  │    - uniforms: global render params                                  │ │
│  │                                                                      │ │
│  │  For each pixel:                                                     │ │
│  │    1. Determine which tile this pixel belongs to                     │ │
│  │    2. Look up tile's orbit offset and dc_origin                      │ │
│  │    3. Compute δc relative to TILE center (not viewport center)      │ │
│  │    4. Run perturbation iteration using tile's local orbit           │ │
│  │                                                                      │ │
│  │  Output: iteration counts, escaped flags, z_norm_sq per pixel        │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Tile Configuration

**Recommended tile sizes:**

| Canvas Size | Tile Size | Tile Count | Orbit Memory (10K iter) |
|-------------|-----------|------------|-------------------------|
| 1920×1080 | 64×64 | 30×17 = 510 | 510 × 80KB = 40MB |
| 1920×1080 | 128×128 | 15×9 = 135 | 135 × 80KB = 11MB |
| 3840×2160 | 64×64 | 60×34 = 2040 | 2040 × 80KB = 160MB |
| 3840×2160 | 128×128 | 30×17 = 510 | 510 × 80KB = 40MB |

**Trade-offs:**

| Smaller Tiles (64×64) | Larger Tiles (128×128) |
|-----------------------|------------------------|
| Smaller δc per tile → more stable | Larger δc per tile → may need rebasing |
| More tiles → more orbit computation | Fewer tiles → faster orbit computation |
| More GPU memory for orbits | Less GPU memory |
| Better for mid-zoom (10^6-10^12) | Better for shallow-mid zoom (10^4-10^8) |

**Recommendation:** Start with 128×128, fall back to 64×64 if artifacts appear.

### 4.3 GPU Data Structures

```wgsl
// Per-tile metadata
struct TileInfo {
    orbit_offset: u32,      // Index into packed_orbits where this tile's orbit starts
    orbit_len: u32,         // Length of this tile's orbit
    reference_escaped: u32, // 1 if reference escaped, 0 otherwise
    dc_origin_re: f32,      // δc at tile's top-left pixel (relative to tile center)
    dc_origin_im: f32,
    _padding: u32,
}

// Global uniforms
struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    dc_step_re: f32,        // δc step per pixel (same for all tiles)
    dc_step_im: f32,
    tile_size: u32,         // e.g., 64 or 128
    tiles_per_row: u32,     // Number of tiles horizontally
    adam7_step: u32,        // 0 = all pixels, 1-7 = Adam7 pass
    _padding: u32,
}

// Bindings
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> packed_orbits: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read> tile_infos: array<TileInfo>;
@group(0) @binding(3) var<storage, read_write> results: array<u32>;
@group(0) @binding(4) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(5) var<storage, read_write> z_norm_sq: array<f32>;
```

### 4.4 GPU Compute Shader

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Adam7 early exit
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

    // Determine which tile this pixel belongs to
    let tile_x = gid.x / uniforms.tile_size;
    let tile_y = gid.y / uniforms.tile_size;
    let tile_idx = tile_y * uniforms.tiles_per_row + tile_x;
    let tile = tile_infos[tile_idx];

    // Compute δc relative to THIS tile's center
    let local_x = gid.x % uniforms.tile_size;
    let local_y = gid.y % uniforms.tile_size;
    let dc = vec2<f32>(
        tile.dc_origin_re + f32(local_x) * uniforms.dc_step_re,
        tile.dc_origin_im + f32(local_y) * uniforms.dc_step_im
    );

    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        // Reference exhaustion detection
        if tile.reference_escaped != 0u && m >= tile.orbit_len {
            glitched = true;
        }

        // Get Z from THIS TILE's orbit (not global orbit)
        let Z = packed_orbits[tile.orbit_offset + (m % tile.orbit_len)];
        let z = Z + dz;

        let z_sq = dot(z, z);
        let Z_sq = dot(Z, Z);
        let dz_sq = dot(dz, dz);

        // Escape check
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            z_norm_sq[idx] = z_sq;
            return;
        }

        // Pauldelbrot glitch detection
        if Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq {
            glitched = true;
        }

        // Rebase check (rarely needed with tile-local references)
        if z_sq < dz_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration
        let two_Z_dz_re = 2.0 * (Z.x * dz.x - Z.y * dz.y);
        let two_Z_dz_im = 2.0 * (Z.x * dz.y + Z.y * dz.x);
        let dz_sq_re = dz.x * dz.x - dz.y * dz.y;
        let dz_sq_im = 2.0 * dz.x * dz.y;

        dz = vec2<f32>(
            two_Z_dz_re + dz_sq_re + dc.x,
            two_Z_dz_im + dz_sq_im + dc.y
        );

        m = m + 1u;
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
}
```

### 4.5 CPU Orbit Computation

```rust
/// Compute reference orbits for all tiles.
pub fn compute_tile_orbits(
    viewport: &Viewport,
    canvas_size: (u32, u32),
    tile_size: u32,
    max_iterations: u32,
) -> TileOrbitData {
    let tiles_x = canvas_size.0.div_ceil(tile_size);
    let tiles_y = canvas_size.1.div_ceil(tile_size);
    let tile_count = tiles_x * tiles_y;

    let precision = viewport.width.precision_bits();
    let mut packed_orbits = Vec::new();
    let mut tile_infos = Vec::with_capacity(tile_count as usize);

    for tile_y in 0..tiles_y {
        for tile_x in 0..tiles_x {
            // Compute tile center in pixel coordinates
            let center_px_x = (tile_x * tile_size + tile_size / 2) as f64;
            let center_px_y = (tile_y * tile_size + tile_size / 2) as f64;

            // Convert to fractal coordinates (BigFloat)
            let (c_ref_x, c_ref_y) = pixel_to_fractal(
                center_px_x,
                center_px_y,
                viewport,
                canvas_size,
                precision,
            );

            // Compute reference orbit at tile center
            let c_ref = (c_ref_x, c_ref_y);
            let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

            // Record orbit offset
            let orbit_offset = packed_orbits.len() as u32;

            // Pack orbit values (convert to f32)
            for (re, im) in &orbit.orbit {
                packed_orbits.push([*re as f32, *im as f32]);
            }

            // Compute dc_origin for tile's top-left pixel relative to tile center
            let top_left_px_x = (tile_x * tile_size) as f64;
            let top_left_px_y = (tile_y * tile_size) as f64;
            let dc_origin_re = ((top_left_px_x - center_px_x) / canvas_size.0 as f64
                * viewport.width.to_f64()) as f32;
            let dc_origin_im = ((top_left_px_y - center_px_y) / canvas_size.1 as f64
                * viewport.height.to_f64()) as f32;

            tile_infos.push(TileInfo {
                orbit_offset,
                orbit_len: orbit.orbit.len() as u32,
                reference_escaped: orbit.escaped_at.is_some() as u32,
                dc_origin_re,
                dc_origin_im,
                _padding: 0,
            });
        }
    }

    TileOrbitData {
        packed_orbits,
        tile_infos,
        tiles_x,
        tiles_y,
        tile_size,
    }
}
```

---

## 5. Integration with Adam7 Progressive Rendering

### 5.1 The Challenge

Adam7 renders scattered pixels across the **entire image** per pass:

```
Pass 1: One pixel per 8×8 block, spread across ALL tiles
Pass 2: More pixels, still scattered across ALL tiles
...
Pass 7: Fill remaining pixels
```

Tile-based references want to process **all pixels in a tile** with one reference orbit.

### 5.2 Solution: Packed Orbits + Single Dispatch

With tile-based orbits packed into a single GPU buffer:

1. **Upload all tile orbits once** (before pass 1)
2. **Each Adam7 pass is ONE dispatch** for the whole image
3. **Shader looks up which tile each pixel belongs to**
4. **Adam7 + tile-local references work together**

```
Pass 1: Single dispatch, all tiles' orbits already in GPU memory
        Each pixel looks up its tile, uses tile's local orbit

Pass 2-7: Same pattern, no orbit re-upload needed
```

This preserves the Adam7 user experience (whole image sharpens progressively) while using per-tile references.

### 5.3 Memory and Performance

| Phase | Memory Transfer | GPU Work |
|-------|-----------------|----------|
| Pre-render | Upload packed_orbits (~40MB) + tile_infos (~20KB) | None |
| Pass 1 | None (data already on GPU) | ~1/64 of pixels |
| Pass 2-6 | None | Progressive |
| Pass 7 | None | ~1/2 of pixels |

**Key insight:** Orbit upload is done ONCE before all Adam7 passes. The orbit data is reused across all passes.

---

## 6. Zoom Range Strategy

### 6.1 Algorithm Selection by Zoom Depth

```rust
fn select_render_strategy(viewport: &Viewport) -> RenderStrategy {
    let zoom_exponent = (4.0 / viewport.width.to_f64()).log10();

    match zoom_exponent {
        z if z < 4.0 => {
            // Direct iteration (no perturbation needed)
            RenderStrategy::Direct
        }
        z if z < 15.0 => {
            // Mid-zoom: tile-based multi-reference
            RenderStrategy::TiledPerturbation {
                tile_size: if z < 8.0 { 128 } else { 64 },
            }
        }
        _ => {
            // Deep zoom: single reference works well
            RenderStrategy::SingleReferencePerturbation
        }
    }
}
```

### 6.2 Adaptive Tile Size

At shallower mid-zoom, larger tiles work fine. At deeper mid-zoom, smaller tiles are needed:

| Zoom Exponent | Tile Size | Rationale |
|---------------|-----------|-----------|
| 4-8 | 128×128 | δc still relatively small per tile |
| 8-12 | 64×64 | Need smaller δc for stability |
| 12-15 | 64×64 or 32×32 | Transitioning to deep zoom |
| >15 | Full frame | Single reference works |

### 6.3 Smooth Transitions

Avoid jarring transitions between render strategies:

1. **Hysteresis:** Don't switch strategies on every frame during zoom
2. **Overlap:** Both strategies produce correct results in their range
3. **Gradual tile size:** Adjust tile size smoothly, not discretely

---

## 7. Implementation Increments

### 7.1 Increment 1: Fix Current GPU Renderer

**Deliverable:** Correct single-reference GPU rendering (fixes current bugs).

**Changes:**

1. Add `reference_escaped` field to Rust `Uniforms` struct
2. Verify uniform layout matches WGSL shader exactly
3. Test at shallow zoom where single reference should work

**Acceptance Criteria:**
- Uniform struct matches shader layout
- Glitch detection works correctly
- No artifacts at zoom < 10^4

---

### 7.2 Increment 2: Tile Infrastructure

**Deliverable:** CPU-side tile orbit computation infrastructure.

**Changes:**

1. Add `TileInfo` struct and `TileOrbitData` container
2. Implement `compute_tile_orbits()` function
3. Add tile configuration to renderer config
4. Unit tests for tile center computation

**Acceptance Criteria:**
- Tile orbits computed correctly (match single-reference at tile center)
- Memory usage within expected bounds
- Tile count scales correctly with canvas size

---

### 7.3 Increment 3: Packed Orbit GPU Buffers

**Deliverable:** GPU buffer management for multi-tile orbits.

**Changes:**

1. New buffer layout: `packed_orbits` + `tile_infos`
2. Buffer upload logic in GPU renderer
3. Buffer size validation against GPU limits

**Acceptance Criteria:**
- Buffers upload correctly
- Memory usage matches predictions
- Graceful handling when buffer size exceeds limits

---

### 7.4 Increment 4: Tile-Aware Compute Shader

**Deliverable:** GPU shader using tile-local references.

**Changes:**

1. New shader: `delta_iteration_tiled.wgsl`
2. Tile lookup logic in shader
3. Per-tile dc_origin computation
4. Integration with existing Adam7 logic

**Acceptance Criteria:**
- Shader compiles and runs
- Iteration counts match CPU reference at tile centers
- Adam7 progressive rendering still works

---

### 7.5 Increment 5: Integration and Testing

**Deliverable:** Full tile-based rendering pipeline.

**Changes:**

1. Integrate tile orbit computation with render flow
2. Automatic tile size selection based on zoom
3. Performance benchmarking
4. Visual quality testing at problematic zoom depths

**Acceptance Criteria:**
- No mosaic artifacts at 10^6, 10^10, 10^14 zoom
- Smooth iteration coloring works correctly
- Performance within 2x of single-reference at deep zoom
- Clean transitions between zoom ranges

---

### 7.6 Increment 6: Optimization

**Deliverable:** Production-quality performance.

**Changes:**

1. Parallel tile orbit computation (workers)
2. Tile orbit caching across frames
3. Incremental tile updates for small viewport changes
4. Memory pooling for orbit buffers

**Acceptance Criteria:**
- Interactive frame rates at mid-zoom
- Memory stable across many zoom operations
- Orbit reuse when panning within same zoom level

---

## 8. Memory and Performance Analysis

### 8.1 Memory Budget

For 3840×2160 canvas with 128×128 tiles (510 tiles) at 10,000 iterations:

| Component | Size | Calculation |
|-----------|------|-------------|
| Packed orbits | 40 MB | 510 × 10,000 × 8 bytes |
| Tile infos | 12 KB | 510 × 24 bytes |
| Results buffer | 33 MB | 8.3M × 4 bytes |
| Glitch flags | 33 MB | 8.3M × 4 bytes |
| z_norm_sq | 33 MB | 8.3M × 4 bytes |
| **Total** | **~140 MB** | |

This is well within typical GPU memory limits (2GB+).

### 8.2 Performance Expectations

**Orbit computation (CPU):**
- 510 tiles × 10,000 iterations = 5.1M orbit points
- ~100ms on modern CPU (parallelizable)

**GPU rendering:**
- Same as current single-reference approach
- Tile lookup adds ~5% overhead
- Overall: 10-100x faster than CPU

**Bottleneck analysis:**

| Zoom Depth | Primary Bottleneck | Secondary |
|------------|-------------------|-----------|
| 10^4 - 10^8 | GPU iteration | Orbit upload |
| 10^8 - 10^12 | Orbit computation | GPU iteration |
| 10^12 - 10^15 | Orbit computation | Precision management |

### 8.3 Comparison with Current Approach

| Metric | Current (Single Reference) | Tile-Based |
|--------|---------------------------|------------|
| Mid-zoom quality | Severe artifacts | Clean |
| Deep zoom quality | Good | Good |
| Memory usage | ~100 MB | ~140 MB |
| Orbit compute time | ~1ms | ~100ms |
| GPU render time | Same | Same + 5% |

---

## 9. Alternative Approaches

### 9.1 Series Approximation

**When to consider:**
- If tile-based approach still shows artifacts
- If orbit computation becomes bottleneck
- For hybrid: use SA for first N iterations, perturbation after

**Pros:**
- More stable than pure perturbation
- Can skip many iterations
- Well-understood mathematically

**Cons:**
- More complex implementation
- Needs probe point validation
- Harder to parallelize

### 9.2 Double-Double Precision

**When to consider:**
- If f32 tile-based still shows precision artifacts
- For zoom depths 10^8 to 10^12 specifically
- As enhancement to tile-based approach

**Pros:**
- ~48 bits of precision
- Faster than f64 on consumer GPUs
- Compatible with tile-based architecture

**Cons:**
- ~3x slower than f32
- More complex shader code
- May not be needed if tiles are small enough

### 9.3 Hybrid: Tiles + Series Approximation

**Architecture:**
```
1. Compute tile orbits (same as tile-based)
2. Compute series coefficients for first N iterations
3. GPU: Apply series approximation for iterations 0..N
4. GPU: Switch to perturbation for iterations N..max
```

**When to consider:**
- Very high iteration counts (100K+)
- When BLA is insufficient
- For maximum stability at mid-zoom

---

## 10. Testing Strategy

### 10.1 Known Problem Coordinates

Test at these coordinates where single-reference fails:

| Location | Zoom | Problem |
|----------|------|---------|
| (-0.75, 0.1) | 10^8 | Boundary region, rebasing issues |
| (-0.5, 0.5) | 10^10 | Mixed dynamics |
| (-1.25, 0.0) | 10^6 | Antenna region |
| Seahorse valley | 10^12 | Complex boundary |

### 10.2 Visual Quality Tests

1. **Smooth coloring continuity:** No sharp jumps within iso-iteration bands
2. **Histogram coloring:** Smooth distribution, no quantization artifacts
3. **Tile boundary invisibility:** No visible grid pattern
4. **Comparison with CPU:** Match iteration counts within ±1

### 10.3 Performance Benchmarks

1. **Orbit computation time** vs tile count
2. **GPU render time** vs single-reference baseline
3. **Memory usage** vs canvas size and iteration count
4. **End-to-end latency** from navigation to display

---

## 11. References

### Primary Sources

1. **ChatGPT Analysis** - Mid-zoom dead zone explanation
   - Identified zoom range 10^4 to 10^15 as problematic
   - Listed tile-based, series approximation, and double-double solutions

2. **Kalles Fraktaler / KFB-based renderers**
   - https://mathr.co.uk/kf/kf.html
   - Uses tile-based multi-reference approach
   - Up to 10,000 references for complex renders

3. **SuperFractalThing successors**
   - Original perturbation theory popularization
   - Tile-based reference approach

4. **FractalShark**
   - https://github.com/mattsaccount364/FractalShark
   - 2x32 double-double type for GPU precision
   - Reference orbit compression

5. **UltraFractal deep zoom plugins**
   - CPU multi-precision
   - Series approximation
   - Local orbit recalculations

### Fractal Wonder Documentation

6. **perturbation-theory.md** - Core perturbation algorithm
7. **webgpu-rendering.md** - GPU architecture and precision emulation

### Research Papers

8. **Phil Thompson** - Perturbation Theory and BLA
   - https://philthompson.me/2022/Perturbation-Theory-and-the-Mandelbrot-set.html

9. **mathr.co.uk** - Deep Zoom Theory and Practice
   - https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html

---

## 12. Conclusion

The mid-zoom range (10^4 to 10^15) is fundamentally ill-conditioned for single-reference GPU perturbation. The artifacts observed in Fractal Wonder are not bugs in the implementation—they are inherent limitations of the approach.

**The solution is architectural, not incremental:**

1. **Accept that single reference fails** in this range
2. **Implement tile-based local references** as the primary fix
3. **Consider series approximation or double-double** as enhancements
4. **Design for smooth transitions** between zoom range strategies

This matches what all production fractal renderers do. The tile-based approach integrates cleanly with the existing Adam7 progressive rendering and GPU architecture.

---

*Document created: November 2025*
*Based on analysis of production renderers, ChatGPT consultation, and Fractal Wonder codebase investigation*
