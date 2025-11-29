# WebGPU Rendering for Deep Mandelbrot Exploration

> **Research Document** - Comprehensive analysis of WebGPU for world-class deep zoom rendering, with comparison to existing GPU implementations.

---

## 1. Executive Summary

### 1.1 Key Findings

| Aspect | Assessment |
|--------|------------|
| **Feasibility** | Yes, with significant engineering effort |
| **Primary Challenge** | f32-only native support (no f64 in WGSL) |
| **Solution** | Perturbation theory + precision emulation |
| **Expected Benefit** | 10-100x speedup for delta iteration at deep zoom |
| **Implementation Complexity** | High - requires custom precision types in shaders |

### 1.2 Recommendation

WebGPU compute shaders can dramatically accelerate Fractal Wonder's rendering, but **only for the delta iteration phase** of perturbation theory. The reference orbit computation must remain on CPU using arbitrary precision (BigFloat).

**Optimal Architecture**:
1. **CPU**: Reference orbit computation (BigFloat), BLA table construction
2. **GPU**: Delta iteration for all pixels (FloatExp or f32 pairs)
3. **Hybrid**: Transfer reference orbit + BLA table to GPU, compute pixels in parallel

---

## 2. The Precision Problem

### 2.1 Why GPU Mandelbrot is Hard

Standard GPU floating-point types have limited precision and range:

| Type | Mantissa Bits | Exponent Range | Max Zoom Depth |
|------|---------------|----------------|----------------|
| f32 (GPU native) | 23 bits | ~10^±38 | ~10^7 |
| f64 (CPU/some GPUs) | 52 bits | ~10^±308 | ~10^15 |
| FloatExp | 52 bits | Unlimited | Unlimited |
| BigFloat | Arbitrary | Unlimited | Unlimited |

At zoom depth 10^100, coordinate deltas are ~10^-100. Standard f32 underflows to zero at ~10^-38.

### 2.2 WebGPU Native Types

WGSL (WebGPU Shading Language) supports:

```wgsl
// Native types
var a: f32;     // 32-bit float (always available)
var b: f16;     // 16-bit float (optional, via extension)
var c: i32;     // 32-bit signed integer
var d: u32;     // 32-bit unsigned integer

// No native f64 support in WGSL specification
```

**Critical limitation**: There is no native f64 (double precision) in WGSL. This is an active discussion in the WebGPU community ([GitHub Issue #2805](https://github.com/gpuweb/gpuweb/issues/2805)), but no timeline for implementation.

### 2.3 Why This Matters for Fractal Wonder

Fractal Wonder targets zoom depths up to 10^2000. Without special techniques:
- Direct Mandelbrot iteration: Fails at ~10^7 (f32) or ~10^15 (f64)
- Perturbation theory extends this dramatically, but delta values still underflow at deep zoom
- World-class renderers solve this with custom precision types

---

## 3. How World-Class Renderers Use GPUs

### 3.1 Architecture Overview

All successful deep-zoom GPU implementations follow this pattern:

```
┌─────────────────────────────────────────────────────────────────┐
│                         CPU (High Precision)                     │
│  ┌─────────────────────┐    ┌─────────────────────────────────┐ │
│  │ Reference Orbit     │    │ BLA Table Construction          │ │
│  │ (BigFloat/MPFR)     │    │ (can be parallel)               │ │
│  └─────────────────────┘    └─────────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────────┘
                               │ Upload reference orbit + BLA table
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                       GPU (Low Precision, Massively Parallel)    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ Delta Iteration (one thread per pixel)                      │ │
│  │ - Uses FloatExp or double-double for extended range         │ │
│  │ - Applies BLA skipping when valid                           │ │
│  │ - Rebases to avoid precision loss                           │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Kalles Fraktaler / Fraktaler 3

**Implementation**: OpenCL (cross-platform GPU)

**Key Features**:
- OpenCL mode for GPU acceleration
- Supports single and double precision modes
- Rescaled perturbation for arbitrarily deep zooms
- Can use CPU via PoCL (Portable OpenCL) when GPU unavailable

**Precision Strategy**:
- Uses "rescaled perturbation" to keep delta values in representable range
- Supports `long double` on x86 for zoom depths 10^300 to 10^4920
- OpenCL kernels work with both f32 and f64 depending on hardware

**Performance Note** (from [mathr.co.uk](https://mathr.co.uk/kf/kf.html)):
> "OpenCL may be faster depending on hardware. OpenCL with CPU is typically faster than the regular CPU code, possibly apart from zoom depths between 1e300 and 1e4920 where regular CPU code can use the long double number type."

### 3.3 FractalShark (CUDA)

**Implementation**: NVIDIA CUDA (proprietary, but fastest)

**Key Features** (from [GitHub](https://github.com/mattsaccount364/FractalShark)):
- Most advanced algorithmic enhancements
- Linear approximation (BLA)
- High-precision MPIR for reference orbit
- Reference orbit compression (reduces GB of RAM for high-period orbits)
- Periodicity detection

**Precision Strategy** - The "2x32" Type:
```
// FractalShark's custom type
struct Float2x32 {
    float hi;      // High 24 bits of mantissa
    float lo;      // Low 24 bits of mantissa
    int exp;       // Extended exponent
};
// Provides ~48-bit mantissa with unlimited range
```

**Why 2x32?** From the documentation:
> "A pair of 32-bit floating point numbers + an exponent, to provide a combined ~48-bit mantissa without using the native 64-bit type. The benefit is significant performance improvements on consumer video cards, with nearly the same precision."

Consumer NVIDIA GPUs have 1:64 f64:f32 performance ratio. Using f32 pairs is **dramatically faster** than native f64.

**Reference Orbit Compression**:
> "An example period-600,000,000 spot can see a memory reduction of multiple gigabytes, which can make the difference between being able to render it and not being able to render it."

### 3.4 DeepDrill

**Implementation**: C++ with OpenCL GPU filters

**Key Features** (from [documentation](https://dirkwhoffmann.github.io/DeepDrill/)):
- Perturbation and series approximation
- Educational codebase ("lean and easy-to-comprehend")
- GPU filters for post-processing
- Distance estimation support

**Philosophy**:
> "DeepDrill tries to provide a lean and easy-to-comprehend code base that allows students to learn modern Mandelbrot algorithms from a concrete implementation."

### 3.5 WebGL/WebGPU Implementations

**bertbaron/mandelbrot** ([GitHub](https://github.com/bertbaron/mandelbrot)):
- JavaScript reference calculation with BigInt
- WebGPU for perturbation iteration
- Uses "perturbation with extended float" algorithm

**par-fractal** ([GitHub](https://github.com/paulrobello/par-fractal)):
- Rust + wgpu (WebGPU)
- 34 fractal types
- Cross-platform (native + WASM)

**LeandroSQ/js-mandelbrot** ([GitHub](https://github.com/LeandroSQ/js-mandelbrot)):
- Comparison of WebGPU, WebGL, WASM performance
- Note: "For WebGPU where there is no support" for f64

---

## 4. Precision Emulation Techniques

### 4.1 Double-Double Arithmetic

Represent a high-precision value as the unevaluated sum of two f32 values:

```
value = hi + lo
where |lo| ≤ ulp(hi)/2
```

**WGSL Implementation Sketch**:

```wgsl
struct DoubleFloat {
    hi: f32,
    lo: f32,
};

// Addition (Knuth's TwoSum algorithm)
fn df_add(a: DoubleFloat, b: DoubleFloat) -> DoubleFloat {
    let s = a.hi + b.hi;
    let v = s - a.hi;
    let e = (a.hi - (s - v)) + (b.hi - v);
    let t = e + a.lo + b.lo;
    return DoubleFloat(s + t, t - (s + t - s));
}

// Multiplication (Veltkamp's split + TwoProd)
fn df_mul(a: DoubleFloat, b: DoubleFloat) -> DoubleFloat {
    let p = a.hi * b.hi;
    // Split a.hi and b.hi for error-free product
    // ... (complex implementation)
}
```

**Precision**: ~48 bits mantissa (from 2×24 bits)
**Range**: Same as f32 (~10^±38) - **NOT sufficient for deep zoom**

**Critical Limitation**: Double-double extends precision but NOT range. For deep zoom, you need FloatExp.

### 4.2 FloatExp (Extended Exponent)

Separate the mantissa and exponent:

```wgsl
struct FloatExp {
    mantissa: f32,   // Normalized: 0.5 ≤ |mantissa| < 1.0
    exp: i32,        // Extended exponent (base 2)
};
// Value = mantissa × 2^exp
```

**Operations**:

```wgsl
fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    var result: FloatExp;
    result.mantissa = a.mantissa * b.mantissa;
    result.exp = a.exp + b.exp;
    return fe_normalize(result);
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    let exp_diff = a.exp - b.exp;

    // If exponents differ by more than mantissa bits, smaller value is negligible
    if (exp_diff > 24) { return a; }
    if (exp_diff < -24) { return b; }

    // Align exponents and add mantissas
    let scaled_b = ldexp(b.mantissa, -exp_diff);
    var result: FloatExp;
    result.mantissa = a.mantissa + scaled_b;
    result.exp = a.exp;
    return fe_normalize(result);
}

fn fe_normalize(x: FloatExp) -> FloatExp {
    if (x.mantissa == 0.0) {
        return FloatExp(0.0, 0);
    }
    // Use frexp to extract exponent, keeping mantissa in [0.5, 1.0)
    var e: i32;
    let m = frexp(x.mantissa, &e);
    return FloatExp(m, x.exp + e);
}
```

**Precision**: 24 bits (single f32 mantissa)
**Range**: Unlimited (limited only by i32 exponent, ~10^±646,000,000)

### 4.3 FloatExp with Double-Double (2x32 + Exponent)

Combines both techniques for maximum precision with unlimited range:

```wgsl
struct FloatExp2x32 {
    hi: f32,         // High part of mantissa
    lo: f32,         // Low part of mantissa
    exp: i32,        // Extended exponent
};
// Provides ~48-bit precision with unlimited range
```

This is FractalShark's approach and represents the **state of the art** for GPU deep zoom rendering.

**Performance** (from FractalShark):
- ~12x speedup vs single-threaded CPU on RTX 4090 vs overclocked 5950X
- Significant improvement over native f64 on consumer GPUs

### 4.4 Comparison

| Technique | Precision | Range | GPU Speed | Complexity |
|-----------|-----------|-------|-----------|------------|
| Native f32 | 24 bits | 10^±38 | Fastest | None |
| Native f64 | 53 bits | 10^±308 | 1/64 of f32 | None |
| Double-Double | ~48 bits | 10^±38 | ~1/20 of f32 | Medium |
| FloatExp (f32) | 24 bits | Unlimited | ~1/10 of f32 | Medium |
| FloatExp (2x32) | ~48 bits | Unlimited | ~1/30 of f32 | High |

---

## 5. BLA Acceleration on GPU

### 5.1 Why BLA is GPU-Friendly

BLA (Bivariate Linear Approximation) is ideal for GPU implementation:

1. **Table construction is parallel**: Each BLA entry is independent
2. **Table merging is parallel**: Each level can be computed in parallel
3. **Per-pixel lookup is uniform**: Same algorithm for all pixels
4. **Memory access is predictable**: Sequential orbit access

### 5.2 BLA Table Structure

For a reference orbit of M iterations:

```
Level 0: M entries, each skipping 1 iteration
Level 1: M/2 entries, each skipping 2 iterations
Level 2: M/4 entries, each skipping 4 iterations
...
Level log₂(M): 1 entry, skipping M-1 iterations

Total entries: ~2M (O(M) space)
```

**GPU Memory Layout**:

```wgsl
struct BlaEntry {
    A_re: f32,          // Linear coefficient (real)
    A_im: f32,          // Linear coefficient (imag)
    B_re: f32,          // Constant coefficient (real)
    B_im: f32,          // Constant coefficient (imag)
    validity_radius: f32, // When |δz| < r, BLA is valid
    skip_length: u32,   // How many iterations this skips
};

@group(0) @binding(0) var<storage, read> bla_table: array<BlaEntry>;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
```

### 5.3 GPU BLA Algorithm

```wgsl
fn iterate_with_bla(
    delta_c: vec2<f32>,
    max_iter: u32
) -> u32 {
    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;  // Reference orbit index
    var n: u32 = 0u;  // Iteration count

    while (n < max_iter) {
        let Z_m = reference_orbit[m];
        let z = Z_m + dz;

        // Escape check
        if (dot(z, z) > 4.0) {
            return n;
        }

        // Try BLA skip
        let bla = find_largest_valid_bla(m, dz, delta_c);
        if (bla.skip_length > 1u) {
            // Apply BLA: δz' = A×δz + B×δc
            dz = complex_mul(bla.A, dz) + complex_mul(bla.B, delta_c);
            m += bla.skip_length;
            n += bla.skip_length;
            continue;
        }

        // Rebase check: |z| < |dz|
        if (dot(z, z) < dot(dz, dz)) {
            dz = z;
            m = 0u;
            continue;
        }

        // Standard delta iteration: δz' = 2Z×δz + δz² + δc
        dz = 2.0 * complex_mul(Z_m, dz) + complex_mul(dz, dz) + delta_c;
        m += 1u;
        n += 1u;
    }

    return max_iter;
}
```

### 5.4 BLA Table Construction (CPU or GPU)

From [mathr.co.uk](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html):

```
1. Create M single-iteration BLAs (parallel on GPU)
2. For level = 1 to log₂(M):
   - Merge adjacent pairs from previous level (parallel on GPU)
   - Each merge: (A_y∘x, B_y∘x) = (A_y × A_x, A_y × B_x + B_y)
   - Update validity radius
3. Result: Binary tree of BLAs
```

The merging formula:
```
l_{y∘x} = l_y + l_x
A_{y∘x} = A_y × A_x
B_{y∘x} = A_y × B_x + B_y
r_{y∘x} = min(r_x, max(0, (r_y − |B_x| × |δc|_max) / |A_x|))
```

---

## 6. WebGPU-Specific Considerations

### 6.1 Compute Shader Limits

From the [WebGPU specification](https://www.w3.org/TR/webgpu/) and [wgpu limits](https://wgpu.rs/doc/wgpu/struct.Limits.html):

| Limit | Default Value | Implication |
|-------|---------------|-------------|
| maxComputeWorkgroupSizeX | 256 | Max threads per workgroup in X |
| maxComputeWorkgroupSizeY | 256 | Max threads per workgroup in Y |
| maxComputeWorkgroupSizeZ | 64 | Max threads per workgroup in Z |
| maxComputeInvocationsPerWorkgroup | 256 | Total threads per workgroup |
| maxComputeWorkgroupsPerDimension | 65535 | Dispatch limit |
| maxStorageBufferBindingSize | 128 MB (minimum) | Reference orbit storage |
| maxStorageBuffersPerShaderStage | 8 | Bindings available |

**Recommended workgroup size**: 64 (general GPU efficiency)

### 6.2 Memory Considerations

**Reference Orbit Storage**:
- At 100M iterations: 100M × 8 bytes (vec2<f32>) = 800 MB
- WebGPU minimum storage buffer: 128 MB
- May need reference orbit compression for extreme iterations

**BLA Table Storage**:
- ~2M entries × 24 bytes per entry = ~48 MB for 1M iteration reference

### 6.3 Precision Without f64

**Strategy 1: FloatExp in WGSL**

Implement FloatExp as a struct with explicit operations:

```wgsl
struct FloatExp {
    mantissa: f32,
    exp: i32,
};

struct ComplexFloatExp {
    re: FloatExp,
    im: FloatExp,
};

fn cfe_mul(a: ComplexFloatExp, b: ComplexFloatExp) -> ComplexFloatExp {
    // (a.re + i*a.im) * (b.re + i*b.im)
    // = (a.re*b.re - a.im*b.im) + i*(a.re*b.im + a.im*b.re)
    return ComplexFloatExp(
        fe_sub(fe_mul(a.re, b.re), fe_mul(a.im, b.im)),
        fe_add(fe_mul(a.re, b.im), fe_mul(a.im, b.re))
    );
}
```

**Strategy 2: Use wgpu-rs with Native Targets**

wgpu supports native backends (Vulkan, Metal, DX12) where f64 may be available:
- Vulkan: `shaderFloat64` feature
- Metal: Not supported on Apple Silicon, supported on Intel Macs
- DX12: Depends on hardware

**Strategy 3: Reference Orbit at Reduced Precision**

For moderate zoom depths (up to ~10^300):
- Store reference orbit as vec2<f32> (sufficient for |Z| ≤ 2)
- Use FloatExp only for delta values
- This is the standard approach used by all production renderers

### 6.4 WebGPU vs WebGL for Fractals

| Aspect | WebGL | WebGPU |
|--------|-------|--------|
| Compute Shaders | Via fragment shader hack | Native support |
| f64 Support | Via GL_ARB_gpu_shader_fp64 (limited) | No native, requires emulation |
| Performance | Good | ~3.5x faster for compute |
| Thread Count | Limited by render target | 65535³ workgroups |
| Memory Access | Texture sampling | Direct buffer access |
| Parallelism | Single draw call | Multi-threaded command encoding |

From [PixelsCommander](https://pixelscommander.com/javascript/webgpu-computations-performance-in-comparison-to-webgl/):
> "WebGPU compute shaders are in practice 3.5x faster than WebGL computing with pixel shaders while having significantly higher limits regarding the amount of data to process."

### 6.5 Cross-Platform Considerations

wgpu (Rust WebGPU implementation) targets:

| Backend | Platform | f64 Support |
|---------|----------|-------------|
| Vulkan | Windows, Linux, Android | Hardware-dependent |
| Metal | macOS, iOS | Apple Silicon: No, Intel: Yes |
| DX12 | Windows | Hardware-dependent |
| WebGPU | Browser | No (WGSL limitation) |
| OpenGL ES | Fallback | Rarely |

**Recommendation**: Design for f32-based FloatExp to ensure cross-platform compatibility.

---

## 7. Implementation Architecture for Fractal Wonder

### 7.1 Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Fractal Wonder                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    CPU (Rust/WASM)                               │ │
│  │  ┌─────────────────────┐  ┌─────────────────────────────────┐  │ │
│  │  │ ReferenceOrbit      │  │ BlaTable                        │  │ │
│  │  │ - BigFloat compute  │  │ - Construct from reference      │  │ │
│  │  │ - Store as f64      │  │ - Binary tree structure         │  │ │
│  │  │ - Compress if large │  │ - Validity radii                │  │ │
│  │  └─────────────────────┘  └─────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                    │                                  │
│                                    ▼                                  │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                   GPU (wgpu/WebGPU)                              │ │
│  │  ┌─────────────────────────────────────────────────────────────┐ │
│  │  │ Compute Shader: delta_iteration.wgsl                        │ │
│  │  │                                                              │ │
│  │  │ Inputs:                                                      │ │
│  │  │ - reference_orbit: array<vec2<f32>>                         │ │
│  │  │ - bla_table: array<BlaEntry>                                │ │
│  │  │ - dc_buffer: array<FloatExp2> (one per pixel)               │ │
│  │  │                                                              │ │
│  │  │ Output:                                                      │ │
│  │  │ - iteration_counts: array<u32>                              │ │
│  │  │ - smooth_values: array<f32> (for coloring)                  │ │
│  │  │                                                              │ │
│  │  │ Algorithm:                                                   │ │
│  │  │ 1. Load δc for this pixel (FloatExp)                        │ │
│  │  │ 2. Initialize δz = 0                                         │ │
│  │  │ 3. Loop:                                                     │ │
│  │  │    a. Try BLA skip if |δz| < validity_radius                │ │
│  │  │    b. Check rebase condition                                 │ │
│  │  │    c. Standard iteration if no skip                          │ │
│  │  │    d. Check escape                                           │ │
│  │  │ 4. Store iteration count                                     │ │
│  │  └─────────────────────────────────────────────────────────────┘ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### 7.2 Data Flow

```
1. User navigates to new view
2. CPU: Compute reference orbit at center (BigFloat)
3. CPU: Build BLA table from reference orbit
4. CPU: Compute δc for each pixel corner (FloatExp)
5. GPU: Upload reference orbit, BLA table, δc values
6. GPU: Dispatch compute shader (one thread per pixel)
7. GPU: Each thread iterates with BLA + rebasing
8. CPU: Read back iteration counts
9. CPU: Apply coloring algorithm
10. Display
```

### 7.3 Zoom-Depth Strategy

| Zoom Depth | Reference Precision | Delta Precision | GPU Benefit |
|------------|---------------------|-----------------|-------------|
| < 10^7 | f64 | f32 | Massive (native f32) |
| 10^7 - 10^15 | f64 | f32 or f64 | Large |
| 10^15 - 10^300 | BigFloat | FloatExp (f32) | Moderate |
| > 10^300 | BigFloat | FloatExp (2x32) | Depends on iteration count |

### 7.4 When GPU Helps Most

GPU acceleration provides the greatest benefit when:
1. **High iteration counts**: 10,000+ iterations
2. **Large images**: 1000×1000+ pixels
3. **BLA effectiveness**: Deep zoom with smooth regions

GPU may not help much when:
1. **Reference orbit dominates**: Very deep zoom with short iterations
2. **Glitch correction**: Multi-reference scenarios require CPU coordination
3. **Extreme precision**: Beyond FloatExp (2x32) capability

---

## 8. Implementation Roadmap

### 8.1 Phase 1: Basic GPU Perturbation (f32)

**Deliverable**: GPU-accelerated rendering up to ~10^7 zoom.

**Implementation**:
1. Compute shader for standard Mandelbrot iteration (f32)
2. Reference orbit on CPU (f64), uploaded as vec2<f32>
3. Delta iteration on GPU (f32)
4. No BLA, no extended precision

**Expected Speedup**: 50-200x vs CPU for large images

### 8.2 Phase 2: FloatExp Deltas

**Deliverable**: GPU-accelerated rendering up to ~10^300 zoom.

**Implementation**:
1. FloatExp type in WGSL
2. δc and δz use FloatExp
3. Reference orbit remains vec2<f32>
4. Rebasing support

**Expected Speedup**: 10-50x vs CPU (FloatExp overhead)

### 8.3 Phase 3: BLA Acceleration

**Deliverable**: Dramatically faster rendering at high iteration counts.

**Implementation**:
1. BLA table construction on CPU
2. BLA table upload to GPU
3. BLA lookup and application in compute shader
4. Fallback to standard iteration when BLA invalid

**Expected Speedup**: 10-100x for high-iteration renders

### 8.4 Phase 4: FloatExp 2x32

**Deliverable**: GPU-accelerated rendering up to ~10^2000 zoom with ~48-bit precision.

**Implementation**:
1. Double-double arithmetic in WGSL
2. Combined with extended exponent
3. All delta operations use FloatExp2x32

**Expected Speedup**: 5-20x vs CPU BigFloat

### 8.5 Phase 5: Reference Orbit Compression

**Deliverable**: Support for extremely high iteration counts (100M+).

**Implementation**:
1. Lossy compression of reference orbit
2. On-the-fly decompression in shader
3. Reduced memory bandwidth

**Expected Benefit**: Enable renders that would otherwise exceed GPU memory

---

## 9. WGSL Code Examples

### 9.1 Basic Complex Operations

```wgsl
// Complex number as vec2<f32>
fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        a.x * b.x - a.y * b.y,
        a.x * b.y + a.y * b.x
    );
}

fn complex_square(z: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        z.x * z.x - z.y * z.y,
        2.0 * z.x * z.y
    );
}

fn complex_norm_sq(z: vec2<f32>) -> f32 {
    return z.x * z.x + z.y * z.y;
}
```

### 9.2 FloatExp Type

```wgsl
struct FloatExp {
    mantissa: f32,
    exp: i32,
};

fn fe_from_f32(x: f32) -> FloatExp {
    if (x == 0.0) {
        return FloatExp(0.0, 0);
    }
    // Normalize to [0.5, 1.0)
    var e: i32 = 0;
    var m = x;
    while (abs(m) >= 1.0) {
        m *= 0.5;
        e += 1;
    }
    while (abs(m) < 0.5 && m != 0.0) {
        m *= 2.0;
        e -= 1;
    }
    return FloatExp(m, e);
}

fn fe_to_f32(x: FloatExp) -> f32 {
    return ldexp(x.mantissa, x.exp);
}

fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if (a.mantissa == 0.0 || b.mantissa == 0.0) {
        return FloatExp(0.0, 0);
    }
    var result = FloatExp(a.mantissa * b.mantissa, a.exp + b.exp);
    // Renormalize
    if (abs(result.mantissa) >= 1.0) {
        result.mantissa *= 0.5;
        result.exp += 1;
    }
    return result;
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if (a.mantissa == 0.0) { return b; }
    if (b.mantissa == 0.0) { return a; }

    let exp_diff = a.exp - b.exp;

    if (exp_diff > 24) { return a; }
    if (exp_diff < -24) { return b; }

    var result: FloatExp;
    if (exp_diff >= 0) {
        let scaled_b = ldexp(b.mantissa, -exp_diff);
        result.mantissa = a.mantissa + scaled_b;
        result.exp = a.exp;
    } else {
        let scaled_a = ldexp(a.mantissa, exp_diff);
        result.mantissa = scaled_a + b.mantissa;
        result.exp = b.exp;
    }

    // Renormalize
    if (result.mantissa == 0.0) {
        return FloatExp(0.0, 0);
    }
    while (abs(result.mantissa) >= 1.0) {
        result.mantissa *= 0.5;
        result.exp += 1;
    }
    while (abs(result.mantissa) < 0.5) {
        result.mantissa *= 2.0;
        result.exp -= 1;
    }

    return result;
}
```

### 9.3 Delta Iteration Compute Shader

```wgsl
struct Uniforms {
    max_iterations: u32,
    escape_radius_sq: f32,
    width: u32,
    height: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read> delta_c: array<vec2<f32>>;  // Per-pixel δc
@group(0) @binding(3) var<storage, read_write> results: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    let pixel_index = y * uniforms.width + x;
    let dc = delta_c[pixel_index];

    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;
    let ref_len = arrayLength(&reference_orbit);

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let Z_m = reference_orbit[m];
        let z = Z_m + dz;
        let z_norm_sq = complex_norm_sq(z);

        // Escape check
        if (z_norm_sq > uniforms.escape_radius_sq) {
            results[pixel_index] = n;
            return;
        }

        // Rebase check: |z| < |dz|
        let dz_norm_sq = complex_norm_sq(dz);
        if (z_norm_sq < dz_norm_sq) {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: δz' = 2Z×δz + δz² + δc
        let two_Z_dz = 2.0 * complex_mul(Z_m, dz);
        let dz_sq = complex_square(dz);
        dz = two_Z_dz + dz_sq + dc;

        m = m + 1u;
        if (m >= ref_len) {
            m = 0u;  // Wrap around for non-escaping reference
        }
    }

    results[pixel_index] = uniforms.max_iterations;
}
```

---

## 10. Performance Expectations

### 10.1 Benchmarks from Existing Implementations

**FractalShark** (CUDA, RTX 4090):
- ~12x faster than single-threaded 5950X CPU
- Reference orbit is the main bottleneck at extreme zoom

**WebGPU vs WebGL** (from research):
- WebGPU compute: 3.5x faster than WebGL pixel shader hack
- 100,000 particles in <2ms vs 10,000 particles in 30ms

**Perturbation vs Direct**:
- 100x+ faster at 10^100 zoom
- Speedup increases with zoom depth

### 10.2 Expected Fractal Wonder Performance

| Configuration | Pixels/Second (estimated) |
|---------------|---------------------------|
| CPU BigFloat (current) | ~10K at 10^100 zoom |
| CPU f64 perturbation | ~1M at 10^15 zoom |
| GPU f32 perturbation | ~100M at 10^7 zoom |
| GPU FloatExp perturbation | ~10M at 10^300 zoom |
| GPU FloatExp + BLA | ~50M+ at 10^300 zoom with high iterations |

### 10.3 Bottleneck Analysis

At different zoom depths, different components dominate:

| Zoom Depth | Primary Bottleneck | Secondary |
|------------|-------------------|-----------|
| < 10^7 | Iteration count | Memory bandwidth |
| 10^7 - 10^300 | Reference orbit compute | Delta iteration |
| > 10^300 | Reference orbit compute | Precision overhead |

---

## 11. Risks and Mitigations

### 11.1 Precision Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| FloatExp precision insufficient | Medium | Artifacts at deep zoom | Fall back to CPU BigFloat for affected regions |
| WGSL compiler optimizes away precision | High | Incorrect results | Careful testing, use volatile patterns |
| Cross-platform precision differences | High | Inconsistent results | Extensive testing on multiple backends |

### 11.2 Performance Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| FloatExp overhead negates GPU benefit | Low | No speedup | Only use GPU when beneficial |
| Memory transfer dominates | Medium | Limited speedup | Keep data on GPU across frames |
| BLA table too large | Low | Memory exhaustion | Reference orbit compression |

### 11.3 Compatibility Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| WebGPU not available | Decreasing | No GPU path | Fall back to CPU (current implementation) |
| wgpu bugs | Medium | Crashes, artifacts | Test thoroughly, report bugs |
| Browser differences | Medium | Inconsistent behavior | Test on Chrome, Firefox, Safari |

---

## 12. Implementation Increments

This section defines self-contained, shippable increments that build progressively toward GPU-accelerated deep zoom rendering. Each increment is complete—no "this will be fixed later" dependencies.

> **Prerequisite**: The perturbation theory increments from `perturbation-theory.md` (Sections 13.1-13.4) must be complete before starting GPU work. GPU acceleration optimizes correct math—it cannot fix incorrect math.

### High-Level Architecture

The GPU path works **alongside** the existing web worker system, not replacing it:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            Main Thread                                  │
│                                                                         │
│  ┌──────────────┐         ┌──────────────────────────────────────────┐  │
│  │ Worker Pool  │         │ GPU Renderer (wgpu)                      │  │
│  │              │         │                                          │  │
│  │ ┌──────────┐ │         │  Inputs:                                 │  │
│  │ │ Worker 1 │ │         │  - reference_orbit: array<vec2<f32>>     │  │
│  │ └──────────┘ │         │  - bla_table: array<BlaEntry>            │  │
│  │ ┌──────────┐ │         │  - dc_origin, dc_step: FloatExp          │  │
│  │ │ Worker N │ │         │                                          │  │
│  │ └──────────┘ │         │  Outputs (per pixel):                    │  │
│  │              │         │  - iterations: u32                       │  │
│  │ Handles:     │         │  - escaped: bool                         │  │
│  │ - Ref orbit  │         │  - glitched: bool                        │  │
│  │   (BigFloat) │         │                                          │  │
│  │ - Glitch     │         │  Renders ALL pixels in one dispatch      │  │
│  │   correction │         │  (thousands of GPU threads in parallel)  │  │
│  │ - Fallback   │         │                                          │  │
│  └──────────────┘         └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key architectural decisions:**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| GPU location | Main thread | wgpu is async (won't block), simpler than GPU-in-worker |
| Precision | FloatExp with f32 mantissa (24-bit) | Start simple, add 2×f32 double-double later if needed |
| δc computation | On GPU from origin + step | Avoids uploading millions of δc values per frame |
| Reference orbit | Persistent on GPU, invalidate on change | Most frames reuse same orbit during zoom/pan |
| Worker role | Reference orbit, glitch correction, fallback | GPU can't do BigFloat; glitch correction needs coordination |

**Render flow:**

1. Worker computes reference orbit (BigFloat) → sends to main thread
2. Main thread uploads orbit + BLA table to GPU (if changed)
3. GPU renders **all pixels** in one compute dispatch
4. GPU returns: iterations, escaped, glitched per pixel
5. If glitched pixels exist:
   - Main thread selects new reference point(s)
   - Workers re-render glitched regions with new references
6. Colorize and display

**Why GPU renders all pixels, not tiles:**

Unlike CPU workers that process one tile at a time, GPU executes thousands of threads in parallel. A 3840×2160 frame
(8.3M pixels) dispatches as ~130,000 workgroups of 64 threads each. Batching by tile adds complexity with no
benefit—GPU handles the entire frame as one unit of work.

### Progressive Rendering

> **Implementation**: See [Increment 2: Progressive Multi-Pass Rendering](#122-increment-2-progressive-multi-pass-rendering) for the detailed implementation specification.

GPU rendering loses the tile-by-tile progress feedback of CPU workers. To restore visual responsiveness, we use
**multi-pass progressive refinement** where both resolution and iteration count scale together:

| Pass | Resolution | Max Iterations | Pixels (4K) | Relative Work |
|------|------------|----------------|-------------|---------------|
| 1 | 1/16 | 1/16 | ~32K | Tiny |
| 2 | 1/8 | 1/8 | ~130K | Small |
| 3 | 1/4 | 1/4 | ~520K | Medium |
| 4 | Full | Full | ~8.3M | Full |

**How it works:**

1. Each pass renders to a smaller buffer at reduced resolution
2. The GPU stretches the result to fill the screen (GPU texture sampling, essentially free)
3. User sees a blocky preview almost instantly, which sharpens through each pass
4. Passes 1-3 combined are <10% of pass 4's work, so "wasted" recomputation is negligible

**Why this works with the architecture:**

- **Reference orbit**: Computed once, reused across all passes (center point unchanged)
- **BLA table**: Computed once, reused across all passes
- **δc values**: Computed on GPU from `origin + step × pixel_index`; just change `step` per pass
- **Iteration scaling + BLA**: With BLA's O(log n) skipping, later passes may be faster than the iteration ratio suggests

**UX characteristics:**

- Shallow zoom: All passes complete so fast the preview is barely visible
- Deep zoom with high iterations: User sees composition immediately, detail fills in over ~1-2 seconds
- Familiar aesthetic: Kalles Fraktaler, XaoS, and other explorers use similar blocky→sharp progression

---

### ✅ 12.1 Increment 1: GPU Infrastructure and Basic f32 Perturbation

**Deliverable:** GPU-accelerated perturbation rendering up to ~10^7 zoom depth.

**What This Builds:**

| Component | Implementation |
|-----------|----------------|
| wgpu pipeline | Device, queue, compute pipeline, bind groups |
| Compute shader | Basic delta iteration in WGSL (f32) |
| Data transfer | Reference orbit upload, iteration count readback |
| Integration | GPU path alongside existing CPU path |

**Architecture:**

```
CPU                              GPU
────────────────────────────────────────────────
ReferenceOrbit (f64)  ──────►  reference_orbit: array<vec2<f32>>
delta_c per pixel     ──────►  delta_c: array<vec2<f32>>
                               ▼
                      ┌─────────────────────────┐
                      │ Compute Shader          │
                      │ - δz' = 2Z·δz + δz² + δc│
                      │ - Escape check          │
                      │ - Rebase check          │
                      └─────────────────────────┘
                               │
iteration_counts      ◄──────  results: array<u32>
```

**Why Self-Contained:**
- f32 range (~10^±38) sufficient for zoom depths up to ~10^7
- Provides massive speedup (50-200x) for shallow zoom
- Establishes GPU pipeline that all subsequent increments build upon
- User can toggle between CPU and GPU rendering
- GPU glitches detected same as CPU (via Pauldelbrot criterion, computed on GPU)

**Test Strategy (Mathematically Grounded):**

1. **Bit-exact iteration match at low iterations:**
   - For n < 100 iterations, GPU and CPU must produce identical iteration counts
   - Test at 1000 random coordinates within f32-safe zoom range
   - Tolerance: 0 (must match exactly)

2. **Statistical agreement at high iterations:**
   - For renders with max_iter > 1000, average iteration count difference < 0.1%
   - GPU may differ by ±1 iteration due to f32 vs f64 precision in escape check

3. **Rebase correctness:**
   - Create test case where rebasing triggers (δz exceeds reference)
   - Verify GPU and CPU rebase at same iteration
   - Verify post-rebase iteration counts match

4. **Escape radius equivalence:**
   - All pixels that escape on CPU must escape on GPU (within ±1 iteration)
   - No pixels should escape on GPU but not CPU (false escapes indicate precision loss)

5. **Performance validation:**
   - GPU render time < CPU render time / 10 for 1000×1000 image
   - Measure and log actual speedup

**Acceptance Criteria:**
- GPU iteration counts match CPU within tolerance at 10^5 zoom
- Glitch detection produces identical pixel masks on CPU and GPU
- Documented speedup of at least 10x for 1M pixel image
- All existing CPU tests still pass
- GPU path can be disabled via configuration

---

### ✅ 12.2 Increment 2: Progressive Multi-Pass Rendering

**NOTE: THIS HAS CHANGED TO IMPLEMENTING ADAM7 INTERLACED/PROGRESSIVE RENDERING**

**Deliverable:** Responsive GPU rendering with blocky→sharp visual refinement.

**What This Builds:**

| Component | Implementation |
|-----------|----------------|
| Multi-resolution dispatch | 4 passes at 1/16, 1/8, 1/4, full resolution |
| Iteration scaling | Max iterations scales proportionally with resolution |
| Texture upsampling | GPU stretches low-res result to fill screen |
| Pass orchestration | Async dispatch with intermediate display |
| Interrupt handling | Cancel remaining passes on navigation |

**Architecture:**

```
Pass 1: 240×135 @ 1/16 max_iter → stretch to 3840×2160 (instant, ~16ms)
Pass 2: 480×270 @ 1/8 max_iter  → stretch to 3840×2160 (~50ms)
Pass 3: 960×540 @ 1/4 max_iter  → stretch to 3840×2160 (~200ms)
Pass 4: 3840×2160 @ full        → final image (1-10s for deep zoom)
```

**Key Implementation Details:**

1. **Same reference orbit for all passes**: Computed once before pass 1, reused by all passes
2. **Same BLA table for all passes** (when Increment 5 is implemented): No recomputation
3. **δc computed per-pixel on GPU**: `origin + step × pixel_index` where `step` varies per pass
4. **Cumulative overhead is minimal**: Passes 1-3 combined are <10% of pass 4's computational work
5. **Single output texture**: Each pass overwrites; no accumulation needed

**Why Self-Contained:**
- Restores progressive feedback lost when moving from tile-based CPU to single-dispatch GPU
- Users see composition immediately (~16ms for pass 1 at 4K)
- Interruptible: navigation cancels pending passes, starts new render sequence
- All subsequent increments (FloatExp, BLA, etc.) automatically inherit this UX pattern
- No dependency on precision features—works with basic f32 from Increment 1

**Test Strategy (Mathematically Grounded):**

1. **Visual equivalence**: Pass 4 output matches single-pass GPU output exactly (bit-identical iteration counts)
2. **Performance targets**:
   - Pass 1 completes in <50ms for 4K viewport
   - Pass 1-3 combined complete in <500ms
3. **Interruption correctness**:
   - Navigation during pass 2 cancels passes 3-4 cleanly
   - No GPU resource leaks from canceled dispatches
   - New render starts from pass 1 with updated viewport
4. **Memory stability**: No memory growth across 100 interrupt/restart cycles

**Acceptance Criteria:**
- User sees blocky preview within 50ms of render start
- Full render (pass 4) matches single-pass GPU output exactly
- Navigation interrupts in-progress renders without visual artifacts or resource leaks
- Memory stable across many interrupt/restart cycles
- Performance overhead of multi-pass vs single-pass is <5%

---

### 12.3 Increment 3: FloatExp Type in WGSL

**Deliverable:** GPU-accelerated rendering up to ~10^300 zoom depth.

**What This Builds:**

```wgsl
struct FloatExp {
    mantissa: f32,   // Normalized: 0.5 ≤ |mantissa| < 1.0
    exp: i32,        // Extended exponent (base 2)
};

struct ComplexFloatExp {
    re: FloatExp,
    im: FloatExp,
};
```

**Operations Implemented:**
- `fe_add(a, b)` - Addition with exponent alignment
- `fe_sub(a, b)` - Subtraction
- `fe_mul(a, b)` - Multiplication (mantissa multiply, exponent add)
- `fe_norm_sq(z)` - Squared magnitude for escape/rebase checks
- `fe_normalize(x)` - Renormalize after operation
- `cfe_add`, `cfe_mul`, `cfe_square` - Complex variants

**Mathematical Foundation:**

FloatExp represents: `value = mantissa × 2^exp`

Key invariants:
1. `0.5 ≤ |mantissa| < 1.0` (or mantissa = 0)
2. After any operation, result is normalized
3. Addition: align exponents, add mantissas, renormalize
4. Multiplication: multiply mantissas, add exponents, renormalize

**Why Self-Contained:**
- Extends GPU rendering to ~10^300 zoom (i32 exponent range: ±2^31)
- 24-bit precision (f32 mantissa) sufficient for perturbation deltas
- Falls back gracefully: if FloatExp insufficient, rebase or flag as glitch
- Full delta iteration in extended precision on GPU

**Test Strategy (Mathematically Grounded):**

1. **FloatExp operation correctness:**
   ```
   For random a, b in range [10^-100, 10^100]:
     fe_add(a, b) ≈ CPU_add(a, b) within 1 ULP of f32
     fe_mul(a, b) ≈ CPU_mul(a, b) within 1 ULP of f32
   ```

2. **Normalization edge cases:**
   - `fe_normalize(0.0, any_exp)` → `(0.0, 0)`
   - `fe_normalize(1.5, e)` → `(0.75, e+1)`
   - `fe_normalize(0.25, e)` → `(0.5, e-1)`

3. **Exponent overflow/underflow:**
   - Exponent approaching i32::MAX: verify graceful handling
   - Exponent approaching i32::MIN: verify no wraparound bugs

4. **Iteration count equivalence:**
   - At zoom 10^100: GPU iteration counts match CPU FloatExp within ±1
   - At zoom 10^200: GPU iteration counts match CPU FloatExp within ±1
   - Test at 10 known coordinates from fractal databases

5. **Delta underflow prevention:**
   - At zoom 10^100, verify δc and δz never become (0.0, 0)
   - Log minimum exponent encountered during render

**Acceptance Criteria:**
- All FloatExp unit tests pass on GPU
- Renders at 10^100 zoom match CPU reference implementation
- No delta underflow at any tested zoom depth
- Performance: GPU FloatExp at least 5x faster than CPU FloatExp

---

### 12.4 Increment 4: Robust GPU Rebasing

**Deliverable:** Correct rebasing on GPU preventing precision loss at all zoom depths.

**Rebasing Algorithm (GPU):**

```wgsl
// In compute shader main loop:
let Z_m = reference_orbit[m];
let z = cfe_add(Z_m, dz);  // Full pixel value in FloatExp
let z_norm_sq = cfe_norm_sq(z);
let dz_norm_sq = cfe_norm_sq(dz);

// Rebase condition: |z| < |dz| (delta has "overtaken" reference)
if (fe_less_than(z_norm_sq, dz_norm_sq)) {
    dz = z;      // Absorb Z into delta
    m = 0u;      // Reset to start of reference orbit
    continue;
}
```

**Why Rebasing is Critical on GPU:**

Without rebasing, precision loss occurs when the pixel orbit diverges from reference orbit. On GPU with limited precision (24 bits), this happens more frequently than on CPU.

**Mathematical Invariant:**

After rebasing at iteration n:
```
z_pixel(n) = Z_reference(0) + δz_new
where δz_new = Z_reference(m) + δz_old
```

The pixel orbit continues from `Z_reference(0)`, maintaining mathematical correctness.

**Why Self-Contained:**
- Prevents silent precision loss that causes visual artifacts
- Combined with FloatExp, enables correct rendering at any zoom depth
- Glitch detection (Pauldelbrot criterion) only needed for edge cases
- Each pixel is independent—rebasing doesn't affect other pixels

**Test Strategy (Mathematically Grounded):**

1. **Rebase trigger correctness:**
   - Create synthetic case where |z| < |dz| at iteration k
   - Verify GPU triggers rebase at exactly iteration k
   - Verify m resets to 0 after rebase

2. **Post-rebase orbit correctness:**
   - After rebase, continue iteration on both CPU and GPU
   - Final iteration counts must match within ±1

3. **Rebase count statistics:**
   - For known coordinates, count rebases on GPU and CPU
   - Rebase counts should match (±10% due to precision differences)

4. **No precision loss without rebasing (negative test):**
   - Disable rebasing, render at 10^50 zoom
   - Verify glitch detection catches precision loss pixels
   - Enable rebasing, verify glitched pixel count drops to near zero

5. **Stress test: rapid rebasing:**
   - Find coordinate that causes 100+ rebases per pixel
   - Verify correct iteration count despite many rebases

**Acceptance Criteria:**
- Rebase-induced glitches reduced by >90% compared to no-rebase GPU path
- Iteration counts match CPU at 10^100, 10^200, 10^300 zoom
- Performance impact of rebasing: <20% overhead
- No infinite rebase loops (timeout detection)

---

### 12.5 Increment 5: BLA Table on GPU

**Deliverable:** BLA acceleration on GPU for O(log n) iteration complexity.

**BLA Data Structures:**

```wgsl
struct BlaEntry {
    A_re: f32,              // Linear coefficient A (real)
    A_im: f32,              // Linear coefficient A (imag)
    A_exp: i32,             // Exponent for A
    B_re: f32,              // Constant coefficient B (real)
    B_im: f32,              // Constant coefficient B (imag)
    B_exp: i32,             // Exponent for B
    validity_radius: f32,   // |δz| must be less than this
    validity_exp: i32,      // Exponent for validity radius
    skip_length: u32,       // How many iterations this BLA skips
};

@group(0) @binding(2) var<storage, read> bla_table: array<BlaEntry>;
```

**BLA Application:**

```wgsl
fn try_bla_skip(m: u32, dz: ComplexFloatExp, dc: ComplexFloatExp) -> BlaResult {
    // Find largest valid BLA at reference index m
    let bla = find_valid_bla(m, cfe_norm_sq(dz));

    if (bla.skip_length > 1u) {
        // Apply BLA: δz' = A × δz + B × δc
        let new_dz = cfe_add(
            cfe_mul(bla.A, dz),
            cfe_mul(bla.B, dc)
        );
        return BlaResult(true, new_dz, bla.skip_length);
    }
    return BlaResult(false, dz, 0u);
}
```

**Mathematical Foundation:**

BLA validity condition:
```
|δz²| < ε × |2Z × δz|
```
When satisfied, we can approximate `δz' ≈ 2Z × δz + δc` (dropping δz² term).

The validity radius `r` is precomputed such that if `|δz| < r`, the BLA is accurate.

**Why Self-Contained:**
- Massive speedup for high-iteration renders (10-100x)
- Mathematical validity guarantees correctness
- Falls back to per-iteration when BLA invalid
- BLA table constructed on CPU, uploaded once per reference orbit

**Test Strategy (Mathematically Grounded):**

1. **BLA skip equivalence:**
   ```
   For each pixel:
     result_with_bla = iterate_with_bla(dc)
     result_without_bla = iterate_without_bla(dc)
     assert |result_with_bla - result_without_bla| ≤ 1
   ```

2. **Validity radius respected:**
   - Log every BLA application where `|δz| < validity_radius`
   - Verify no BLA applications where `|δz| ≥ validity_radius`

3. **BLA table structure:**
   - Verify table has ~2M entries for M-iteration reference
   - Verify skip lengths form powers of 2 (1, 2, 4, 8, ...)

4. **Performance scaling:**
   - Render with max_iter = 1000, 10000, 100000
   - Verify render time grows sub-linearly with max_iter (O(log n) vs O(n))

5. **Edge case: BLA always invalid:**
   - Create coordinate where δz is always large
   - Verify graceful fallback to per-iteration (no crashes, correct result)

**Acceptance Criteria:**
- Iteration counts identical with/without BLA
- At least 10x speedup at max_iter = 100,000
- No visual artifacts from BLA approximation
- BLA table memory usage documented and within GPU limits

---

### 12.6 Increment 6: FloatExp 2x32 (Double-Double Precision)

**Deliverable:** ~48-bit precision delta iteration on GPU for extreme accuracy.

**Data Structure:**

```wgsl
struct FloatExp2x32 {
    hi: f32,       // High part of mantissa (~24 bits)
    lo: f32,       // Low part of mantissa (~24 bits)
    exp: i32,      // Extended exponent
};
// Total: ~48 bits of mantissa precision, unlimited range
```

**Double-Double Arithmetic:**

The key insight: `hi + lo` represents a value more precisely than either alone.

```wgsl
// Knuth's TwoSum: error-free addition
fn two_sum(a: f32, b: f32) -> vec2<f32> {
    let s = a + b;
    let v = s - a;
    let e = (a - (s - v)) + (b - v);
    return vec2<f32>(s, e);
}

// Double-double addition
fn dd_add(a: FloatExp2x32, b: FloatExp2x32) -> FloatExp2x32 {
    // Align exponents, then use TwoSum for error-free addition
    // ... (full implementation in shader)
}
```

**Why 2x32 Instead of Native f64:**

| Approach | Performance on Consumer GPU | Precision |
|----------|----------------------------|-----------|
| Native f64 | 1/64 of f32 speed | 53 bits |
| 2x32 + exp | ~1/30 of f32 speed | ~48 bits |

Consumer GPUs (NVIDIA GeForce, AMD Radeon) have severely limited f64 performance. Using two f32 values is **faster** while providing nearly the same precision.

**When Needed:**

- At zoom depths >10^100, f32 mantissa (24 bits) may show artifacts
- High-iteration renders where errors accumulate
- Coordinates near Misiurewicz points (chaotic, precision-sensitive)

**Why Self-Contained:**
- Addresses precision edge cases that f32 FloatExp misses
- Optional: only used when extra precision needed
- Same algorithm as FloatExp, just with double-double operations
- Can mix: use 2x32 for δz, f32 FloatExp for less critical values

**Test Strategy (Mathematically Grounded):**

1. **2x32 operation correctness:**
   ```
   For random a, b in range [10^-200, 10^200]:
     dd_add(a, b) matches BigFloat_add(a, b) within 2^-47 relative error
     dd_mul(a, b) matches BigFloat_mul(a, b) within 2^-47 relative error
   ```

2. **Error accumulation test:**
   - Iterate 1M times with both f32 FloatExp and 2x32 FloatExp
   - Compare final δz values
   - 2x32 should be closer to BigFloat reference

3. **Precision-sensitive coordinates:**
   - Render Misiurewicz points (known to be precision-sensitive)
   - Compare iteration counts: f32 FloatExp vs 2x32 FloatExp vs CPU BigFloat
   - 2x32 should match BigFloat more closely

4. **Performance overhead:**
   - Measure render time: f32 FloatExp vs 2x32 FloatExp
   - Overhead should be <3x (2x32 is ~3x slower than f32)

**Acceptance Criteria:**
- 2x32 iteration counts match CPU BigFloat at 10^500 zoom
- Visible reduction in artifacts at precision-sensitive locations
- Performance: 2x32 GPU still faster than CPU FloatExp
- Graceful fallback to f32 FloatExp when 2x32 not needed

---

### 12.7 Increment 7: Reference Orbit Compression

**Deliverable:** Support for extremely high iteration counts (100M+) within GPU memory.

**The Problem:**

At 100M iterations:
- Uncompressed: 100M × 8 bytes = 800 MB (exceeds many GPU limits)
- With BLA table: additional ~1.6 GB
- Total: >2 GB for reference data alone

**Compression Strategy (from FractalShark):**

Store only "keyframe" orbit values at intervals, reconstruct intermediate values on-the-fly:

```
Keyframes: Z_0, Z_1000, Z_2000, Z_3000, ...
Reconstruction: Z_1500 = iterate(Z_1000, c, 500)  // Compute when needed
```

**GPU Implementation:**

```wgsl
struct CompressedOrbit {
    keyframes: array<vec2<f32>, MAX_KEYFRAMES>,
    keyframe_interval: u32,
    total_iterations: u32,
};

fn get_reference_value(compressed: CompressedOrbit, m: u32) -> vec2<f32> {
    let keyframe_idx = m / compressed.keyframe_interval;
    let offset = m % compressed.keyframe_interval;

    if (offset == 0u) {
        return compressed.keyframes[keyframe_idx];
    }

    // Reconstruct: iterate from keyframe
    var z = compressed.keyframes[keyframe_idx];
    for (var i = 0u; i < offset; i++) {
        z = complex_square(z) + c_ref;  // c_ref stored separately
    }
    return z;
}
```

**Trade-offs:**

| Compression Ratio | Memory Savings | Reconstruction Overhead |
|-------------------|----------------|------------------------|
| 10:1 | 90% | ~10% slower |
| 100:1 | 99% | ~50% slower |
| 1000:1 | 99.9% | ~200% slower |

**Why Self-Contained:**
- Enables renders that would otherwise exceed GPU memory
- Configurable compression ratio (trade memory for speed)
- Transparent to rest of pipeline—just returns orbit values
- Can fall back to uncompressed for low iteration counts

**Test Strategy (Mathematically Grounded):**

1. **Reconstruction correctness:**
   - For every reconstructed value Z_m, verify:
     ```
     reconstructed_Z_m == iterate(keyframe, c, offset)
     ```
   - Test with 1000 random m values

2. **Iteration count equivalence:**
   - Render same image with compressed and uncompressed orbits
   - Iteration counts must match exactly (compression is lossless)

3. **Memory usage validation:**
   - At 100M iterations with 1000:1 compression:
     - Uncompressed: 800 MB
     - Compressed: <1 MB + reconstruction overhead
   - Verify actual GPU memory usage

4. **Performance scaling:**
   - Measure render time vs compression ratio
   - Verify overhead matches theoretical prediction

5. **Edge cases:**
   - m = 0 (first keyframe)
   - m = last keyframe (no reconstruction needed)
   - m = total_iterations - 1 (maximum offset)

**Acceptance Criteria:**
- Successfully render 100M iteration coordinate on GPU with <100 MB reference orbit
- Iteration counts match uncompressed reference
- Documented compression ratio and performance trade-off
- Automatic fallback to uncompressed when memory available

---

### 12.8 Summary

| Increment | Zoom Depth | Precision | Performance | Key Capability |
|-----------|------------|-----------|-------------|----------------|
| 1. Basic f32 GPU | ~10^7 | 24 bits | 50-200x CPU | GPU pipeline established |
| 2. Progressive Passes | ~10^7 | 24 bits | Same | Responsive blocky→sharp UX |
| 3. FloatExp | ~10^300 | 24 bits | 10-50x CPU | Extended range |
| 4. GPU Rebasing | ~10^300 | 24 bits | 10-50x CPU | Precision loss prevention |
| 5. BLA on GPU | ~10^300 | 24 bits | 100-1000x CPU at high iter | O(log n) iterations |
| 6. FloatExp 2x32 | ~10^2000 | ~48 bits | 5-20x CPU | Extreme precision |
| 7. Orbit Compression | ~10^2000 | ~48 bits | Variable | 100M+ iterations |

**Dependency Graph:**

```
perturbation-theory.md Increments 1-4 (CPU correctness)
              │
              ▼
    ┌─────────────────────┐
    │ Increment 1: Basic  │
    │ GPU Infrastructure  │
    └─────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Increment 2:        │
    │ Progressive Passes  │
    └─────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Increment 3:        │
    │ FloatExp in WGSL    │
    └─────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Increment 4:        │
    │ GPU Rebasing        │
    └─────────────────────┘
              │
              ├──────────────────────┐
              ▼                      ▼
    ┌─────────────────────┐  ┌─────────────────────┐
    │ Increment 5:        │  │ Increment 6:        │
    │ BLA on GPU          │  │ FloatExp 2x32       │
    └─────────────────────┘  └─────────────────────┘
              │                      │
              └──────────┬───────────┘
                         ▼
              ┌─────────────────────┐
              │ Increment 7:        │
              │ Orbit Compression   │
              └─────────────────────┘
```

**Key Principles:**

1. **Each increment is shippable**: User gets measurable value after each increment
2. **CPU remains ground truth**: GPU results validated against CPU at every step
3. **Performance is measured, not assumed**: Each increment includes performance tests
4. **Graceful degradation**: GPU failures fall back to CPU seamlessly
5. **Mathematical correctness first**: Speedups only accepted if correctness preserved

**Testing Philosophy:**

Tests verify mathematical invariants, not just "GPU produces some output":
- Iteration counts match CPU reference (within documented tolerance)
- Precision loss detected and handled (via rebasing or glitch flagging)
- Performance gains are real and measurable
- Edge cases (overflow, underflow, extreme values) handled correctly

---

## 13. References

### Primary Sources

1. **mathr.co.uk** - Claude Heiland-Allen
   - [Deep Zoom Theory and Practice](https://mathr.co.uk/blog/2021-05-14_deep_zoom_theory_and_practice.html)
   - [Deep Zoom Theory and Practice (Again)](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)
   - [Kalles Fraktaler 2+](https://mathr.co.uk/kf/kf.html)
   - [Fraktaler 3](https://mathr.co.uk/web/fraktaler.html)

2. **FractalShark** - Matt (mattsaccount364)
   - [GitHub Repository](https://github.com/mattsaccount364/FractalShark)
   - 2x32 type, reference orbit compression, BLA implementations

3. **DeepDrill** - Dirk Hoffmann
   - [Documentation](https://dirkwhoffmann.github.io/DeepDrill/)
   - Educational codebase for perturbation theory

4. **Phil Thompson**
   - [Perturbation Theory and the Mandelbrot Set](https://philthompson.me/2022/Perturbation-Theory-and-the-Mandelbrot-set.html)
   - [BLA Explanation](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html)

### WebGPU Resources

5. **WebGPU Specification**
   - [W3C WGSL Spec](https://www.w3.org/TR/WGSL/)
   - [GitHub Issue #2805 - f64 Support](https://github.com/gpuweb/gpuweb/issues/2805)

6. **wgpu (Rust WebGPU)**
   - [wgpu.rs](https://wgpu.rs/)
   - [wgpu Limits Documentation](https://wgpu.rs/doc/wgpu/struct.Limits.html)

7. **WebGPU Fundamentals**
   - [Compute Shader Basics](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)

### Precision Emulation

8. **Godot Engine**
   - [Emulating Double Precision on GPU](https://godotengine.org/article/emulating-double-precision-gpu-render-large-worlds/)

9. **metal-float64** (Apple GPUs)
   - [GitHub Repository](https://github.com/philipturner/metal-float64)

10. **Stack Overflow**
    - [Emulate double using 2 floats](https://stackoverflow.com/questions/6769881/emulate-double-using-2-floats)
    - [WebGPU workgroup size limits](https://stackoverflow.com/questions/74020273/what-are-webgpu-workgroup-size-limits)

### WebGPU Fractal Implementations

11. **par-fractal** - [GitHub](https://github.com/paulrobello/par-fractal)
    - Rust + wgpu fractal renderer

12. **Fractl** - [GitHub](https://github.com/Shapur1234/Fractl)
    - Rust fractal renderer with wgpu compute

13. **bertbaron/mandelbrot** - [GitHub](https://github.com/bertbaron/mandelbrot)
    - WebGPU perturbation implementation

14. **js-mandelbrot** - [GitHub](https://github.com/LeandroSQ/js-mandelbrot)
    - WebGPU vs WebGL vs WASM comparison

### Performance Analysis

15. **WebGPU vs WebGL Performance**
    - [Toji.dev Best Practices](https://toji.dev/webgpu-best-practices/webgl-performance-comparison.html)
    - [PixelsCommander Benchmark](https://pixelscommander.com/javascript/webgpu-computations-performance-in-comparison-to-webgl/)

16. **Research Paper**
    - [WebGL vs WebGPU Performance Analysis](https://www.researchgate.net/publication/379686570_WebGL_vs_WebGPU_A_Performance_Analysis_for_Web_30)

---

*Document created: November 2025*
*Based on research from fractal community, WebGPU specifications, and production renderer implementations*
