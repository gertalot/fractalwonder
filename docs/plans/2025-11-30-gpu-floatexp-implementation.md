# GPU FloatExp Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement FloatExp (f32 mantissa + i32 exponent) in WGSL shaders for GPU-accelerated Mandelbrot rendering from shallow zoom through ~10^300.

**Architecture:** Two new GPU pipelines - Direct Mandelbrot for zoom < 10^20 (no perturbation), and Perturbation FloatExp for zoom > 10^20. Both use FloatExp arithmetic in WGSL to extend range beyond f32 limits.

**Tech Stack:** Rust, wgpu, WGSL compute shaders, bytemuck

---

## Part A: Direct FloatExp Shader

### Task 1: Add FloatExp Accessors

**Files:**
- Modify: `fractalwonder-core/src/floatexp.rs`

**Step 1: Add accessor methods to FloatExp**

Add these methods after the existing `is_zero()` method (around line 63):

```rust
    /// Get the mantissa value.
    pub fn mantissa(&self) -> f64 {
        self.mantissa
    }

    /// Get the exponent value.
    pub fn exp(&self) -> i64 {
        self.exp
    }
```

**Step 2: Run tests to verify no regressions**

Run: `cargo test -p fractalwonder-core -- --nocapture`
Expected: All existing FloatExp tests pass

**Step 3: Commit**

```bash
git add fractalwonder-core/src/floatexp.rs
git commit -m "feat(core): add mantissa/exp accessors to FloatExp"
```

---

### Task 2: Create FloatExp WGSL Library

**Files:**
- Create: `fractalwonder-gpu/src/shaders/floatexp.wgsl`

**Step 1: Create the FloatExp type and operations**

```wgsl
// FloatExp: Extended-range floating point for GPU.
// Value = m × 2^e where m is normalized to [0.5, 1.0) or 0.

struct FloatExp {
    m: f32,  // mantissa
    e: i32,  // exponent (base 2)
}

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}

// Zero constant
const FE_ZERO: FloatExp = FloatExp(0.0, 0);
const CFE_ZERO: ComplexFE = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

// Create FloatExp from f32
fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(x, 0));
}

// Normalize mantissa to [0.5, 1.0)
fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FE_ZERO; }

    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));

    return FloatExp(new_m, x.e + e_adjust);
}

// Negate
fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

// Multiply two FloatExp values
fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}

// Add two FloatExp values
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

// Subtract: a - b
fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}

// Convert FloatExp to f32 (may overflow/underflow)
fn fe_to_f32(x: FloatExp) -> f32 {
    if x.m == 0.0 { return 0.0; }
    // Clamp exponent to avoid inf/0
    let clamped_e = clamp(x.e, -126, 127);
    return x.m * exp2(f32(clamped_e));
}

// Complex multiplication: (a.re + a.im*i) * (b.re + b.im*i)
fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(
        fe_sub(fe_mul(a.re, b.re), fe_mul(a.im, b.im)),
        fe_add(fe_mul(a.re, b.im), fe_mul(a.im, b.re))
    );
}

// Complex addition
fn cfe_add(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(fe_add(a.re, b.re), fe_add(a.im, b.im));
}

// Complex squared magnitude |a|² = re² + im²
// Returns f32 since result is bounded for escape check
fn cfe_norm_sq(a: ComplexFE) -> f32 {
    let re_sq = fe_mul(a.re, a.re);
    let im_sq = fe_mul(a.im, a.im);
    let sum = fe_add(re_sq, im_sq);
    return fe_to_f32(sum);
}

// Convert vec2<f32> to ComplexFE
fn vec2_to_cfe(v: vec2<f32>) -> ComplexFE {
    return ComplexFE(fe_from_f32(v.x), fe_from_f32(v.y));
}
```

**Step 2: Verify shader compiles (will test with full shader later)**

This file will be included by other shaders. No standalone test needed yet.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/floatexp.wgsl
git commit -m "feat(gpu): add FloatExp WGSL library"
```

---

### Task 3: Create Direct FloatExp Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/direct_floatexp.wgsl`

**Step 1: Create the complete shader**

```wgsl
// Direct Mandelbrot iteration using FloatExp arithmetic.
// For zoom levels < 10^20 where perturbation is not needed.

// Include FloatExp library (copy contents from floatexp.wgsl here since WGSL doesn't have #include)
// --- BEGIN FLOATEXP LIBRARY ---
struct FloatExp {
    m: f32,
    e: i32,
}

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}

const FE_ZERO: FloatExp = FloatExp(0.0, 0);
const CFE_ZERO: ComplexFE = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(x, 0));
}

fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FE_ZERO; }
    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));
    return FloatExp(new_m, x.e + e_adjust);
}

fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 { return b; }
    if b.m == 0.0 { return a; }
    let exp_diff = a.e - b.e;
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

fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}

fn fe_to_f32(x: FloatExp) -> f32 {
    if x.m == 0.0 { return 0.0; }
    let clamped_e = clamp(x.e, -126, 127);
    return x.m * exp2(f32(clamped_e));
}

fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
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
    return fe_to_f32(sum);
}
// --- END FLOATEXP LIBRARY ---

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

// Adam7 interlacing pattern
fn get_adam7_pass(x: u32, y: u32) -> u32 {
    let row = y % 8u;
    let col = x % 8u;
    let matrix = array<array<u32, 8>, 8>(
        array<u32, 8>(1u, 6u, 4u, 6u, 2u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(3u, 6u, 4u, 6u, 3u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
    );
    return matrix[row][col];
}

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

fn compute_pixel_c(px: u32, py: u32) -> ComplexFE {
    let origin_re = FloatExp(uniforms.c_origin_re_m, uniforms.c_origin_re_e);
    let origin_im = FloatExp(uniforms.c_origin_im_m, uniforms.c_origin_im_e);
    let step_re = FloatExp(uniforms.c_step_re_m, uniforms.c_step_re_e);
    let step_im = FloatExp(uniforms.c_step_im_m, uniforms.c_step_im_e);

    let px_fe = fe_from_f32(f32(px));
    let py_fe = fe_from_f32(f32(py));

    return ComplexFE(
        fe_add(origin_re, fe_mul(px_fe, step_re)),
        fe_add(origin_im, fe_mul(py_fe, step_im))
    );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Adam7 early exit
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        return;
    }

    let c = compute_pixel_c(gid.x, gid.y);
    var z = CFE_ZERO;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let z_sq = cfe_norm_sq(z);

        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            z_norm_sq[idx] = z_sq;
            return;
        }

        // z = z² + c
        z = cfe_add(cfe_mul(z, z), c);
    }

    results[idx] = uniforms.max_iterations;
    z_norm_sq[idx] = cfe_norm_sq(z);
}
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/shaders/direct_floatexp.wgsl
git commit -m "feat(gpu): add direct Mandelbrot FloatExp shader"
```

---

### Task 4: Create DirectFloatExp Uniforms

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add DirectFloatExpUniforms struct**

Add after the existing `Uniforms` struct (around line 46):

```rust
/// Uniform data for direct FloatExp compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DirectFloatExpUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,

    pub c_origin_re_m: f32,
    pub c_origin_re_e: i32,
    pub c_origin_im_m: f32,
    pub c_origin_im_e: i32,

    pub c_step_re_m: f32,
    pub c_step_re_e: i32,
    pub c_step_im_m: f32,
    pub c_step_im_e: i32,

    pub adam7_step: u32,
    pub _padding: u32,
}

impl DirectFloatExpUniforms {
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        c_origin: (f32, i32, f32, i32),  // (re_m, re_e, im_m, im_e)
        c_step: (f32, i32, f32, i32),    // (re_m, re_e, im_m, im_e)
        adam7_step: u32,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0, // 256² for smooth coloring
            c_origin_re_m: c_origin.0,
            c_origin_re_e: c_origin.1,
            c_origin_im_m: c_origin.2,
            c_origin_im_e: c_origin.3,
            c_step_re_m: c_step.0,
            c_step_re_e: c_step.1,
            c_step_im_m: c_step.2,
            c_step_im_e: c_step.3,
            adam7_step,
            _padding: 0,
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add DirectFloatExpUniforms struct"
```

---

### Task 5: Create DirectFloatExp Buffers

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add DirectFloatExpBuffers struct**

Add after `DirectFloatExpUniforms`:

```rust
/// GPU buffers for direct FloatExp rendering.
/// Simpler than perturbation buffers - no reference orbit, no glitch flags.
pub struct DirectFloatExpBuffers {
    pub uniforms: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,
    pub pixel_count: u32,
}

impl DirectFloatExpBuffers {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let pixel_count = width * height;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_uniforms"),
            size: std::mem::size_of::<DirectFloatExpUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_staging_results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_staging_z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            results,
            z_norm_sq,
            staging_results,
            staging_z_norm_sq,
            pixel_count,
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add DirectFloatExpBuffers struct"
```

---

### Task 6: Create DirectFloatExp Pipeline

**Files:**
- Create: `fractalwonder-gpu/src/direct_pipeline.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create the pipeline module**

Create `fractalwonder-gpu/src/direct_pipeline.rs`:

```rust
//! Compute pipeline for direct Mandelbrot iteration with FloatExp.

/// Compute pipeline for direct FloatExp Mandelbrot.
pub struct DirectFloatExpPipeline {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl DirectFloatExpPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("direct_floatexp"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/direct_floatexp.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("direct_floatexp_layout"),
            entries: &[
                // binding 0: uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: results (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: z_norm_sq (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("direct_floatexp_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("direct_floatexp_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            compute_pipeline,
            bind_group_layout,
        }
    }
}
```

**Step 2: Add module to lib.rs**

In `fractalwonder-gpu/src/lib.rs`, add after `mod pipeline;`:

```rust
mod direct_pipeline;
```

And add to exports:

```rust
pub use direct_pipeline::DirectFloatExpPipeline;
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles (shader syntax validated)

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/direct_pipeline.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add DirectFloatExpPipeline"
```

---

### Task 7: Create DirectFloatExp Renderer

**Files:**
- Create: `fractalwonder-gpu/src/direct_renderer.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create the renderer module**

Create `fractalwonder-gpu/src/direct_renderer.rs`:

```rust
//! GPU renderer for direct Mandelbrot iteration with FloatExp.

use crate::buffers::{DirectFloatExpBuffers, DirectFloatExpUniforms};
use crate::device::GpuContext;
use crate::direct_pipeline::DirectFloatExpPipeline;
use crate::error::GpuError;
use crate::stretch::SENTINEL_NOT_COMPUTED;
use crate::Adam7Pass;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a direct FloatExp render.
pub struct DirectFloatExpResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

/// GPU renderer for direct Mandelbrot with FloatExp arithmetic.
pub struct DirectFloatExpRenderer {
    context: GpuContext,
    pipeline: DirectFloatExpPipeline,
    buffers: Option<DirectFloatExpBuffers>,
    current_dimensions: Option<(u32, u32)>,
}

impl DirectFloatExpRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = DirectFloatExpPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            current_dimensions: None,
        }
    }

    /// Render using direct Mandelbrot iteration with FloatExp.
    ///
    /// # Arguments
    /// * `c_origin` - Top-left corner as (re_mantissa, re_exp, im_mantissa, im_exp)
    /// * `c_step` - Per-pixel step as (re_mantissa, re_exp, im_mantissa, im_exp)
    #[allow(clippy::too_many_arguments)]
    pub async fn render(
        &mut self,
        c_origin: (f32, i32, f32, i32),
        c_step: (f32, i32, f32, i32),
        width: u32,
        height: u32,
        max_iterations: u32,
        pass: Adam7Pass,
    ) -> Result<DirectFloatExpResult, GpuError> {
        let start = Self::now();

        // Recreate buffers if dimensions changed
        if self.current_dimensions != Some((width, height)) {
            self.buffers = Some(DirectFloatExpBuffers::new(
                &self.context.device,
                width,
                height,
            ));
            self.current_dimensions = Some((width, height));
        }

        let buffers = self.buffers.as_ref().unwrap();

        // Write uniforms
        let uniforms = DirectFloatExpUniforms::new(
            width,
            height,
            max_iterations,
            c_origin,
            c_step,
            pass.step() as u32,
        );
        self.context
            .queue
            .write_buffer(&buffers.uniforms, 0, bytemuck::bytes_of(&uniforms));

        // Create bind group
        let bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("direct_floatexp_bind_group"),
                layout: &self.pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffers.uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers.results.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffers.z_norm_sq.as_entire_binding(),
                    },
                ],
            });

        // Dispatch compute shader
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("direct_floatexp_encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("direct_floatexp_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }

        // Copy results to staging buffers
        let pixel_count = (width * height) as usize;

        encoder.copy_buffer_to_buffer(
            &buffers.results,
            0,
            &buffers.staging_results,
            0,
            (pixel_count * std::mem::size_of::<u32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &buffers.z_norm_sq,
            0,
            &buffers.staging_z_norm_sq,
            0,
            (pixel_count * std::mem::size_of::<f32>()) as u64,
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let iterations = self
            .read_buffer_u32(&buffers.staging_results, pixel_count)
            .await?;
        let z_norm_sq_data = self
            .read_buffer_f32(&buffers.staging_z_norm_sq, pixel_count)
            .await?;

        // Convert to ComputeData
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(z_norm_sq_data.iter())
            .map(|(&iter, &z_sq)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations && iter != SENTINEL_NOT_COMPUTED,
                    glitched: false, // Direct iteration never glitches
                    final_z_norm_sq: z_sq,
                })
            })
            .collect();

        let end = Self::now();

        Ok(DirectFloatExpResult {
            data,
            compute_time_ms: end - start,
        })
    }

    async fn read_buffer_u32(
        &self,
        buffer: &wgpu::Buffer,
        _count: usize,
    ) -> Result<Vec<u32>, GpuError> {
        let slice = buffer.slice(..);

        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        let data = {
            let view = slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };
        buffer.unmap();

        Ok(data)
    }

    async fn read_buffer_f32(
        &self,
        buffer: &wgpu::Buffer,
        _count: usize,
    ) -> Result<Vec<f32>, GpuError> {
        let slice = buffer.slice(..);

        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        let data = {
            let view = slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };
        buffer.unmap();

        Ok(data)
    }

    #[cfg(target_arch = "wasm32")]
    fn now() -> f64 {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn now() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }
}
```

**Step 2: Add module to lib.rs**

In `fractalwonder-gpu/src/lib.rs`, add:

```rust
mod direct_renderer;
```

And add to exports:

```rust
pub use direct_renderer::{DirectFloatExpRenderer, DirectFloatExpResult};
```

Also update the buffers export to include the new types:

```rust
pub use buffers::{DirectFloatExpBuffers, DirectFloatExpUniforms, GpuBuffers, Uniforms};
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/direct_renderer.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add DirectFloatExpRenderer"
```

---

### Task 8: Add DirectFloatExp Tests

**Files:**
- Modify: `fractalwonder-gpu/src/tests.rs`

**Step 1: Add test imports**

At the top of `tests.rs`, add to imports:

```rust
use crate::DirectFloatExpRenderer;
use fractalwonder_core::FloatExp;
```

**Step 2: Add DirectFloatExp tests**

Add at the end of `tests.rs`:

```rust
/// Helper to convert FloatExp to tuple format for renderer.
fn floatexp_to_tuple(re: FloatExp, im: FloatExp) -> (f32, i32, f32, i32) {
    (
        re.mantissa() as f32,
        re.exp() as i32,
        im.mantissa() as f32,
        im.exp() as i32,
    )
}

/// Test that DirectFloatExp renderer initializes without panic.
#[test]
fn direct_floatexp_init_does_not_panic() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };
        let _renderer = DirectFloatExpRenderer::new(ctx);
        println!("DirectFloatExpRenderer initialized successfully");
    });
}

/// Test that DirectFloatExp produces correct results for known points.
#[test]
fn direct_floatexp_known_points() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = DirectFloatExpRenderer::new(ctx);

        let width = 3u32;
        let height = 1u32;
        let max_iter = 100;

        // Test 3 points: origin (in set), c=3 (escapes fast), c=-2 (boundary)
        let c_origin = floatexp_to_tuple(
            FloatExp::from_f64(0.0),
            FloatExp::from_f64(0.0),
        );
        let c_step = floatexp_to_tuple(
            FloatExp::from_f64(1.5),  // 0, 1.5, 3.0
            FloatExp::from_f64(0.0),
        );

        let result = renderer
            .render(c_origin, c_step, width, height, max_iter, Adam7Pass::all_pixels())
            .await
            .expect("Render should succeed");

        let iter_0 = as_mandelbrot(&result.data[0]).iterations;
        let iter_1 = as_mandelbrot(&result.data[1]).iterations;
        let iter_2 = as_mandelbrot(&result.data[2]).iterations;

        println!("c=0: {} iterations", iter_0);
        println!("c=1.5: {} iterations", iter_1);
        println!("c=3: {} iterations", iter_2);

        // Origin should reach max_iter (in set)
        assert_eq!(iter_0, max_iter, "c=0 should be in set");

        // c=3 should escape very quickly (1-2 iterations)
        assert!(iter_2 < 5, "c=3 should escape within 5 iterations");
    });
}

/// Test DirectFloatExp at moderate zoom (10^4) - the problematic range.
#[test]
fn direct_floatexp_moderate_zoom() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = DirectFloatExpRenderer::new(ctx);

        let width = 64u32;
        let height = 64u32;
        let max_iter = 500;

        // Zoom 10^4 near the main cardioid boundary
        let center_re = -0.75;
        let center_im = 0.1;
        let view_size = 1e-4;

        let c_origin = floatexp_to_tuple(
            FloatExp::from_f64(center_re - view_size / 2.0),
            FloatExp::from_f64(center_im - view_size / 2.0),
        );
        let c_step = floatexp_to_tuple(
            FloatExp::from_f64(view_size / width as f64),
            FloatExp::from_f64(view_size / height as f64),
        );

        let result = renderer
            .render(c_origin, c_step, width, height, max_iter, Adam7Pass::all_pixels())
            .await
            .expect("Render should succeed");

        // Count escaped vs in-set pixels
        let escaped = result.data.iter().filter(|d| as_mandelbrot(d).escaped).count();
        let in_set = result.data.iter().filter(|d| !as_mandelbrot(d).escaped).count();

        println!("Moderate zoom (10^4) at ({}, {}):", center_re, center_im);
        println!("  Escaped: {}", escaped);
        println!("  In set: {}", in_set);
        println!("  Compute time: {:.2}ms", result.compute_time_ms);

        // Should have a mix of escaped and in-set pixels at boundary
        assert!(escaped > 0, "Should have some escaped pixels");
        assert!(in_set > 0, "Should have some in-set pixels");
    });
}
```

**Step 3: Run the tests**

Run: `cargo test -p fractalwonder-gpu -- --nocapture`
Expected: All tests pass (or skip gracefully if no GPU)

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/tests.rs
git commit -m "test(gpu): add DirectFloatExp renderer tests"
```

---

## Part B: Perturbation FloatExp Shader

### Task 9: Create Perturbation FloatExp Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl`

**Step 1: Create the shader**

```wgsl
// Delta iteration compute shader with FloatExp arithmetic.
// For deep zoom (> 10^20) using perturbation theory.

// --- BEGIN FLOATEXP LIBRARY ---
struct FloatExp {
    m: f32,
    e: i32,
}

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}

const FE_ZERO: FloatExp = FloatExp(0.0, 0);
const CFE_ZERO: ComplexFE = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(x, 0));
}

fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FE_ZERO; }
    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));
    return FloatExp(new_m, x.e + e_adjust);
}

fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 { return b; }
    if b.m == 0.0 { return a; }
    let exp_diff = a.e - b.e;
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

fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}

fn fe_to_f32(x: FloatExp) -> f32 {
    if x.m == 0.0 { return 0.0; }
    let clamped_e = clamp(x.e, -126, 127);
    return x.m * exp2(f32(clamped_e));
}

fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
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
    return fe_to_f32(sum);
}

fn vec2_to_cfe(v: vec2<f32>) -> ComplexFE {
    return ComplexFE(fe_from_f32(v.x), fe_from_f32(v.y));
}
// --- END FLOATEXP LIBRARY ---

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

fn get_adam7_pass(x: u32, y: u32) -> u32 {
    let row = y % 8u;
    let col = x % 8u;
    let matrix = array<array<u32, 8>, 8>(
        array<u32, 8>(1u, 6u, 4u, 6u, 2u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(3u, 6u, 4u, 6u, 3u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
    );
    return matrix[row][col];
}

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

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
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

    let dc = compute_pixel_dc(gid.x, gid.y);
    var dz = CFE_ZERO;
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        if uniforms.reference_escaped != 0u && m >= orbit_len {
            glitched = true;
        }

        let Z = reference_orbit[m % orbit_len];
        let Z_cfe = vec2_to_cfe(Z);
        let z = cfe_add(Z_cfe, dz);

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

        // Rebase: when |z| < |dz|, we've lost precision
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

        m = m + 1u;
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
}
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl
git commit -m "feat(gpu): add perturbation FloatExp shader"
```

---

### Task 10: Create Perturbation FloatExp Uniforms

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add PerturbationFloatExpUniforms**

Add after `DirectFloatExpBuffers`:

```rust
/// Uniform data for perturbation FloatExp compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PerturbationFloatExpUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,

    pub dc_origin_re_m: f32,
    pub dc_origin_re_e: i32,
    pub dc_origin_im_m: f32,
    pub dc_origin_im_e: i32,

    pub dc_step_re_m: f32,
    pub dc_step_re_e: i32,
    pub dc_step_im_m: f32,
    pub dc_step_im_e: i32,

    pub adam7_step: u32,
    pub reference_escaped: u32,
    pub _padding: [u32; 2],
}

impl PerturbationFloatExpUniforms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: (f32, i32, f32, i32),
        dc_step: (f32, i32, f32, i32),
        adam7_step: u32,
        reference_escaped: bool,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0,
            tau_sq,
            dc_origin_re_m: dc_origin.0,
            dc_origin_re_e: dc_origin.1,
            dc_origin_im_m: dc_origin.2,
            dc_origin_im_e: dc_origin.3,
            dc_step_re_m: dc_step.0,
            dc_step_re_e: dc_step.1,
            dc_step_im_m: dc_step.2,
            dc_step_im_e: dc_step.3,
            adam7_step,
            reference_escaped: if reference_escaped { 1 } else { 0 },
            _padding: [0; 2],
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add PerturbationFloatExpUniforms"
```

---

### Task 11: Create Perturbation FloatExp Pipeline

**Files:**
- Create: `fractalwonder-gpu/src/perturbation_floatexp_pipeline.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create the pipeline module**

Create `fractalwonder-gpu/src/perturbation_floatexp_pipeline.rs`:

```rust
//! Compute pipeline for perturbation delta iteration with FloatExp.

/// Compute pipeline for perturbation with FloatExp arithmetic.
pub struct PerturbationFloatExpPipeline {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl PerturbationFloatExpPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("delta_iteration_floatexp"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/delta_iteration_floatexp.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("delta_iteration_floatexp_layout"),
            entries: &[
                // binding 0: uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: reference_orbit (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 2: results (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: glitch_flags (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 4: z_norm_sq (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("delta_iteration_floatexp_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("delta_iteration_floatexp_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            compute_pipeline,
            bind_group_layout,
        }
    }
}
```

**Step 2: Add to lib.rs**

Add module declaration:

```rust
mod perturbation_floatexp_pipeline;
```

Add export:

```rust
pub use perturbation_floatexp_pipeline::PerturbationFloatExpPipeline;
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/perturbation_floatexp_pipeline.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add PerturbationFloatExpPipeline"
```

---

### Task 12: Update Buffers Export

**Files:**
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Update buffers export to include all new types**

Replace the buffers export line with:

```rust
pub use buffers::{
    DirectFloatExpBuffers, DirectFloatExpUniforms, GpuBuffers,
    PerturbationFloatExpUniforms, Uniforms,
};
```

**Step 2: Verify build**

Run: `cargo check -p fractalwonder-gpu`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/lib.rs
git commit -m "chore(gpu): export all buffer types"
```

---

### Task 13: Run Full Test Suite

**Step 1: Run all quality checks**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace -- --nocapture
```

Expected: All pass without warnings

**Step 2: Commit any formatting fixes**

```bash
git add -A
git commit -m "style: apply formatting"
```

---

## Summary

**Part A (Direct FloatExp):**
- Task 1: FloatExp accessors
- Task 2: FloatExp WGSL library
- Task 3: Direct FloatExp shader
- Task 4: DirectFloatExpUniforms
- Task 5: DirectFloatExpBuffers
- Task 6: DirectFloatExpPipeline
- Task 7: DirectFloatExpRenderer
- Task 8: Tests

**Part B (Perturbation FloatExp):**
- Task 9: Perturbation FloatExp shader
- Task 10: PerturbationFloatExpUniforms
- Task 11: PerturbationFloatExpPipeline
- Task 12: Export updates
- Task 13: Full test suite

---

*Plan complete. Ready for execution.*
