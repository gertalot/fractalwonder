# GPU FloatExp Implementation Design

> **Increment 2.1** - GPU-accelerated rendering with extended-range arithmetic

---

## Summary

Implement FloatExp (f32 mantissa + i32 exponent) in WGSL shaders to enable GPU-accelerated Mandelbrot rendering from shallow zoom through ~10^300.

**Two shaders:**
- **Part A:** Direct Mandelbrot (`z = z² + c`) for zoom < 10^20
- **Part B:** Perturbation (`δz = 2Zδz + δz² + δc`) for zoom > 10^20

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Mantissa precision | f32 (24-bit) | Native GPU speed; upgrade to 2×f32 later if needed |
| Shader organization | Separate files | Cleaner than branching; optimized paths |
| Uniform layout | Flat fields | Keeps exponents as true i32 |
| Renderer selection | Automatic by zoom | Seamless UX; threshold ~10^20 |

---

## 1. FloatExp Type in WGSL

```wgsl
struct FloatExp {
    m: f32,    // mantissa, normalized to [0.5, 1.0) or 0
    e: i32,    // exponent (base 2)
}
// Value = m × 2^e

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}
```

---

## 2. FloatExp Operations

### Normalization

```wgsl
fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FloatExp(0.0, 0); }

    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));

    return FloatExp(new_m, x.e + e_adjust);
}
```

### Multiplication

```wgsl
fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FloatExp(0.0, 0); }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}
```

### Addition

```wgsl
fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 { return b; }
    if b.m == 0.0 { return a; }

    let exp_diff = a.e - b.e;

    // If difference > 24 bits, smaller is negligible
    if exp_diff > 24 { return a; }
    if exp_diff < -24 { return b; }

    if exp_diff >= 0 {
        let scaled_b = b.m * exp2(f32(-exp_diff));
        return fe_normalize(FloatExp(a.m + scaled_b, a.e));
    } else {
        let scaled_a = a.m * exp2(f32(exp_diff));
        return fe_normalize(FloatExp(scaled_a + b.m, b.e));
    }
}
```

### Subtraction & Negation

```wgsl
fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}
```

### Complex Operations

```wgsl
fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    // (a.re + a.im*i) * (b.re + b.im*i)
    return ComplexFE(
        fe_sub(fe_mul(a.re, b.re), fe_mul(a.im, b.im)),
        fe_add(fe_mul(a.re, b.im), fe_mul(a.im, b.re))
    );
}

fn cfe_add(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(fe_add(a.re, b.re), fe_add(a.im, b.im));
}

fn cfe_norm_sq(a: ComplexFE) -> f32 {
    let re_sq = fe_mul(a.re, a.re);
    let im_sq = fe_mul(a.im, a.im);
    let sum = fe_add(re_sq, im_sq);
    return sum.m * exp2(f32(sum.e));
}

fn vec2_to_cfe(v: vec2<f32>) -> ComplexFE {
    return ComplexFE(fe_from_f32(v.x), fe_from_f32(v.y));
}

fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FloatExp(0.0, 0); }
    return fe_normalize(FloatExp(x, 0));
}
```

---

## 3. Part A: Direct Mandelbrot Shader

**File:** `fractalwonder-gpu/src/shaders/direct_floatexp.wgsl`

### Uniforms

```wgsl
struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,

    c_origin_re_m: f32,
    c_origin_re_e: i32,
    c_origin_im_m: f32,
    c_origin_im_e: i32,

    c_step_re_m: f32,
    c_step_re_e: i32,
    c_step_im_m: f32,
    c_step_im_e: i32,

    adam7_step: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> results: array<u32>;
@group(0) @binding(2) var<storage, read_write> z_norm_sq: array<f32>;
```

### Main Function

```wgsl
fn compute_pixel_c(px: u32, py: u32) -> ComplexFE {
    let origin_re = FloatExp(uniforms.c_origin_re_m, uniforms.c_origin_re_e);
    let origin_im = FloatExp(uniforms.c_origin_im_m, uniforms.c_origin_im_e);
    let step_re = FloatExp(uniforms.c_step_re_m, uniforms.c_step_re_e);
    let step_im = FloatExp(uniforms.c_step_im_m, uniforms.c_step_im_e);

    // c = origin + pixel * step
    let px_fe = fe_from_f32(f32(px));
    let py_fe = fe_from_f32(f32(py));

    return ComplexFE(
        fe_add(origin_re, fe_mul(px_fe, step_re)),
        fe_add(origin_im, fe_mul(py_fe, step_im))
    );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height { return; }

    let idx = gid.y * uniforms.width + gid.x;

    // Adam7 filtering (if enabled)
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        return;
    }

    let c = compute_pixel_c(gid.x, gid.y);
    var z = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let z_sq = cfe_norm_sq(z);

        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            z_norm_sq[idx] = z_sq;
            return;
        }

        z = cfe_add(cfe_mul(z, z), c);
    }

    results[idx] = uniforms.max_iterations;
    z_norm_sq[idx] = cfe_norm_sq(z);
}
```

---

## 4. Part B: Perturbation Shader with FloatExp

**File:** `fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl`

### Uniforms

```wgsl
struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,

    dc_origin_re_m: f32,
    dc_origin_re_e: i32,
    dc_origin_im_m: f32,
    dc_origin_im_e: i32,

    dc_step_re_m: f32,
    dc_step_re_e: i32,
    dc_step_im_m: f32,
    dc_step_im_e: i32,

    adam7_step: u32,
    reference_escaped: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;
```

### Main Function

```wgsl
fn compute_pixel_dc(px: u32, py: u32) -> ComplexFE {
    let origin_re = FloatExp(uniforms.dc_origin_re_m, uniforms.dc_origin_re_e);
    let origin_im = FloatExp(uniforms.dc_origin_im_m, uniforms.dc_origin_im_e);
    let step_re = FloatExp(uniforms.dc_step_re_m, uniforms.dc_step_re_e);
    let step_im = FloatExp(uniforms.dc_step_im_m, uniforms.dc_step_im_e);

    let px_fe = fe_from_f32(f32(px));
    let py_fe = fe_from_f32(f32(py));

    return ComplexFE(
        fe_add(origin_re, fe_mul(px_fe, step_re)),
        fe_add(origin_im, fe_mul(py_fe, step_im))
    );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height { return; }

    let idx = gid.y * uniforms.width + gid.x;

    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

    let dc = compute_pixel_dc(gid.x, gid.y);
    var dz = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        if uniforms.reference_escaped != 0u && m >= orbit_len {
            glitched = true;
        }

        let Z = reference_orbit[m % orbit_len];
        let z = cfe_add(vec2_to_cfe(Z), dz);

        let z_sq = cfe_norm_sq(z);
        let Z_sq = dot(Z, Z);
        let dz_sq = cfe_norm_sq(dz);

        // Escape
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            z_norm_sq[idx] = z_sq;
            return;
        }

        // Glitch detection
        if Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq {
            glitched = true;
        }

        // Rebase
        if z_sq < dz_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: δz' = 2Zδz + δz² + δc
        let two_Z = vec2_to_cfe(Z * 2.0);
        let two_Z_dz = cfe_mul(two_Z, dz);
        let dz_squared = cfe_mul(dz, dz);
        dz = cfe_add(cfe_add(two_Z_dz, dz_squared), dc);

        m += 1u;
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
}
```

---

## 5. Rust Integration

### New Pipeline Struct

```rust
// fractalwonder-gpu/src/direct_floatexp.rs

pub struct DirectFloatExpPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    results_buffer: wgpu::Buffer,
    z_norm_sq_buffer: wgpu::Buffer,
}
```

### Uniform Conversion

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DirectFloatExpUniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,

    c_origin_re_m: f32,
    c_origin_re_e: i32,
    c_origin_im_m: f32,
    c_origin_im_e: i32,

    c_step_re_m: f32,
    c_step_re_e: i32,
    c_step_im_m: f32,
    c_step_im_e: i32,

    adam7_step: u32,
    _padding: u32,
}

impl DirectFloatExpUniforms {
    pub fn from_viewport(
        viewport: &Viewport,
        width: u32,
        height: u32,
        max_iterations: u32,
        adam7_step: u32,
    ) -> Self {
        let step_re = viewport.width() / BigFloat::from(width);
        let step_im = viewport.height() / BigFloat::from(height);

        let c_origin_re = FloatExp::from_bigfloat(&viewport.min_re);
        let c_origin_im = FloatExp::from_bigfloat(&viewport.max_im);
        let c_step_re = FloatExp::from_bigfloat(&step_re);
        let c_step_im = FloatExp::from_bigfloat(&step_im.neg());

        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 4.0,
            c_origin_re_m: c_origin_re.mantissa() as f32,
            c_origin_re_e: c_origin_re.exp() as i32,
            c_origin_im_m: c_origin_im.mantissa() as f32,
            c_origin_im_e: c_origin_im.exp() as i32,
            c_step_re_m: c_step_re.mantissa() as f32,
            c_step_re_e: c_step_re.exp() as i32,
            c_step_im_m: c_step_im.mantissa() as f32,
            c_step_im_e: c_step_im.exp() as i32,
            adam7_step,
            _padding: 0,
        }
    }
}
```

### Renderer Selection

```rust
pub enum GpuPipelineKind {
    DirectFloatExp,       // z = z² + c with FloatExp
    PerturbationF32,      // existing f32 perturbation
    PerturbationFloatExp, // δz iteration with FloatExp
}

impl ParallelRenderer {
    fn select_gpu_pipeline(&self, zoom_exponent: i64) -> GpuPipelineKind {
        if zoom_exponent < 20 {
            GpuPipelineKind::DirectFloatExp
        } else {
            GpuPipelineKind::PerturbationFloatExp
        }
    }
}
```

---

## 6. Testing Strategy

### Visual Validation

| Test | Location | Expected |
|------|----------|----------|
| Shallow zoom (10^2) | (-0.75, 0.1) | No mosaic artifacts |
| Moderate zoom (10^8) | (-0.5, 0.5) | Smooth iteration bands |
| Deep zoom (10^14) | Seahorse valley | Matches CPU reference |
| Transition zone (~10^20) | Any | Seamless pipeline switch |

### Cross-validation Test

```rust
#[test]
fn gpu_floatexp_matches_cpu_floatexp() {
    let viewport = create_test_viewport(zoom: 1e10);

    let cpu_result = render_cpu_floatexp(&viewport);
    let gpu_result = render_gpu_floatexp(&viewport);

    for (cpu, gpu) in cpu_result.iter().zip(gpu_result.iter()) {
        assert!((cpu.iterations as i32 - gpu.iterations as i32).abs() <= 1);
    }
}
```

### Performance Benchmarks

| Metric | Target |
|--------|--------|
| Direct FloatExp vs f32 | < 2× slower |
| Perturbation FloatExp vs f32 | < 2× slower |
| 4K render at 10^10 zoom | < 1s |

---

## 7. Acceptance Criteria

1. No mosaic/stripe artifacts at 10^4 zoom
2. Renders correctly up to 10^300 zoom
3. GPU matches CPU iteration counts (±1)
4. All existing tests pass
5. Performance within 2× of f32 path
6. No clippy warnings
7. Code formatted with rustfmt

---

## 8. Implementation Order

1. **Part A: Direct FloatExp shader**
   - FloatExp type and operations in WGSL
   - `direct_floatexp.wgsl` shader
   - Rust pipeline struct and uniform conversion
   - Integration with renderer selection

2. **Part B: Perturbation FloatExp shader**
   - `delta_iteration_floatexp.wgsl` shader
   - Rust uniform struct for perturbation
   - Integration with existing perturbation pipeline

3. **Testing & validation**
   - Visual tests at multiple zoom levels
   - Cross-validation with CPU
   - Performance benchmarks

---

*Design complete. Ready for implementation planning.*
