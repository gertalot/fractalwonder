# GPU Progressive Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace tile-based GPU rendering with full row-set dispatches using venetian blinds pattern and iteration chunking for progressive rendering without GPU timeouts.

**Architecture:** New `ProgressiveGpuRenderer` processes entire row-sets (every Nth row across full image width) per dispatch. Iteration state persists on GPU between chunks. Each row-set completes fully before the next begins, with immediate canvas updates.

**Tech Stack:** Rust, wgpu, WGSL compute shaders, WebAssembly

---

## Task 1: Add GPU Config Fields to FractalConfig

**Files:**
- Modify: `fractalwonder-ui/src/config.rs:19-53`

**Step 1: Add the new fields to FractalConfig struct**

Add after `gpu_enabled: bool` (line 52):

```rust
    /// Iterations per GPU dispatch (prevents timeout).
    /// Default 100,000 keeps each dispatch under browser timeout threshold.
    pub gpu_iterations_per_dispatch: u32,
    /// Number of row-sets for progressive rendering (venetian blinds).
    /// Default 16 means rows 0,16,32... render first, then 1,17,33..., etc.
    pub gpu_progressive_row_sets: u32,
```

**Step 2: Update test_image config**

Add to the test_image FractalConfig (after `gpu_enabled: false`):

```rust
        gpu_iterations_per_dispatch: 100_000,
        gpu_progressive_row_sets: 16,
```

**Step 3: Update mandelbrot config**

Add to the mandelbrot FractalConfig (after `gpu_enabled: true`):

```rust
        gpu_iterations_per_dispatch: 100_000,
        gpu_progressive_row_sets: 16,
```

**Step 4: Run tests to verify compilation**

Run: `cargo test -p fractalwonder-ui config`
Expected: All config tests pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/config.rs
git commit -m "$(cat <<'EOF'
feat(config): add GPU progressive rendering config fields

Add gpu_iterations_per_dispatch and gpu_progressive_row_sets to
FractalConfig for controlling iteration chunking and venetian blinds
row-set count.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Create ProgressiveGpuUniforms

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add the new uniforms struct**

Add after `PerturbationHDRUniforms` impl block (after line 206):

```rust
/// Uniform data for progressive GPU rendering with row-sets.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ProgressiveGpuUniforms {
    // Image dimensions
    pub image_width: u32,
    pub image_height: u32,

    // Row-set info
    pub row_set_index: u32,
    pub row_set_count: u32,
    pub row_set_pixel_count: u32,
    pub _pad0: u32,

    // Iteration chunking
    pub chunk_start_iter: u32,
    pub chunk_size: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,
    pub _pad1: u32,

    // dc_origin as HDRFloat
    pub dc_origin_re_head: f32,
    pub dc_origin_re_tail: f32,
    pub dc_origin_re_exp: i32,
    pub _pad2: u32,
    pub dc_origin_im_head: f32,
    pub dc_origin_im_tail: f32,
    pub dc_origin_im_exp: i32,
    pub _pad3: u32,

    // dc_step as HDRFloat
    pub dc_step_re_head: f32,
    pub dc_step_re_tail: f32,
    pub dc_step_re_exp: i32,
    pub _pad4: u32,
    pub dc_step_im_head: f32,
    pub dc_step_im_tail: f32,
    pub dc_step_im_exp: i32,
    pub _pad5: u32,

    // Reference orbit info
    pub reference_escaped: u32,
    pub orbit_len: u32,
    pub _pad6: [u32; 2],
}

impl ProgressiveGpuUniforms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        image_width: u32,
        image_height: u32,
        row_set_index: u32,
        row_set_count: u32,
        row_set_pixel_count: u32,
        chunk_start_iter: u32,
        chunk_size: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        reference_escaped: bool,
        orbit_len: u32,
    ) -> Self {
        Self {
            image_width,
            image_height,
            row_set_index,
            row_set_count,
            row_set_pixel_count,
            _pad0: 0,
            chunk_start_iter,
            chunk_size,
            max_iterations,
            escape_radius_sq: 65536.0,
            tau_sq,
            _pad1: 0,
            dc_origin_re_head: dc_origin.0 .0,
            dc_origin_re_tail: dc_origin.0 .1,
            dc_origin_re_exp: dc_origin.0 .2,
            _pad2: 0,
            dc_origin_im_head: dc_origin.1 .0,
            dc_origin_im_tail: dc_origin.1 .1,
            dc_origin_im_exp: dc_origin.1 .2,
            _pad3: 0,
            dc_step_re_head: dc_step.0 .0,
            dc_step_re_tail: dc_step.0 .1,
            dc_step_re_exp: dc_step.0 .2,
            _pad4: 0,
            dc_step_im_head: dc_step.1 .0,
            dc_step_im_tail: dc_step.1 .1,
            dc_step_im_exp: dc_step.1 .2,
            _pad5: 0,
            reference_escaped: if reference_escaped { 1 } else { 0 },
            orbit_len,
            _pad6: [0, 0],
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "$(cat <<'EOF'
feat(gpu): add ProgressiveGpuUniforms for row-set rendering

New uniform struct supports row-set indexing and iteration chunking
for progressive GPU rendering.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Create ProgressiveGpuBuffers

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add the new buffers struct**

Add after `ProgressiveGpuUniforms` impl block:

```rust
/// GPU buffers for progressive row-set rendering.
/// Includes persistent state buffers for iteration chunking.
pub struct ProgressiveGpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,

    // Persistent state (read-write, kept on GPU between chunks)
    pub z_re: wgpu::Buffer,
    pub z_im: wgpu::Buffer,
    pub iter_count: wgpu::Buffer,
    pub escaped: wgpu::Buffer,
    pub orbit_index: wgpu::Buffer,

    // Results (read back on row-set completion)
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,

    // Staging buffers for CPU readback
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,

    pub orbit_capacity: u32,
    pub row_set_pixel_count: u32,
}

impl ProgressiveGpuBuffers {
    /// Create buffers sized for a row-set.
    /// row_set_pixel_count = (image_height / row_set_count) * image_width (rounded up)
    pub fn new(device: &wgpu::Device, orbit_len: u32, row_set_pixel_count: u32) -> Self {
        let pixel_count = row_set_pixel_count as usize;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_uniforms"),
            size: std::mem::size_of::<ProgressiveGpuUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reference_orbit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_reference_orbit"),
            size: (orbit_len as usize * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Persistent state buffers - HDRFloat z uses 3 f32s per component (head, tail, exp as f32)
        let z_re = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_re"),
            size: (pixel_count * 3 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let z_im = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_im"),
            size: (pixel_count * 3 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let iter_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_iter_count"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let escaped = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_escaped"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let orbit_index = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_orbit_index"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Result buffers
        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let glitch_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_glitch_flags"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_norm_sq"),
            size: (pixel_count * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffers
        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_glitches = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_glitches"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_z_norm_sq"),
            size: (pixel_count * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            reference_orbit,
            z_re,
            z_im,
            iter_count,
            escaped,
            orbit_index,
            results,
            glitch_flags,
            z_norm_sq,
            staging_results,
            staging_glitches,
            staging_z_norm_sq,
            orbit_capacity: orbit_len,
            row_set_pixel_count,
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "$(cat <<'EOF'
feat(gpu): add ProgressiveGpuBuffers with persistent state

Buffers for row-set rendering include z_re, z_im, iter_count, escaped,
and orbit_index for state persistence between iteration chunks.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Create Progressive WGSL Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Create the shader file**

```wgsl
// Progressive GPU Rendering Shader
// Processes row-sets with iteration chunking and persistent state.

// HDRFloat library (same as delta_iteration_hdr.wgsl)
struct HDRFloat {
    head: f32,
    tail: f32,
    exp: i32,
}

struct HDRComplex {
    re: HDRFloat,
    im: HDRFloat,
}

const HDR_ZERO: HDRFloat = HDRFloat(0.0, 0.0, 0);
const HDR_COMPLEX_ZERO: HDRComplex = HDRComplex(HDRFloat(0.0, 0.0, 0), HDRFloat(0.0, 0.0, 0));

fn hdr_exp2(n: i32) -> f32 {
    if n < -149 { return 0.0; }
    if n > 127 { return 1.0e38; }
    if n >= -126 {
        return bitcast<f32>(u32(n + 127) << 23u);
    }
    return bitcast<f32>(1u << u32(n + 149));
}

fn hdr_two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    return (a - a_virtual) + (b - b_virtual);
}

fn hdr_normalize(x: HDRFloat) -> HDRFloat {
    var head = x.head;
    var tail = x.tail;
    var exp = x.exp;

    if head == 0.0 {
        if tail == 0.0 { return HDR_ZERO; }
        head = tail;
        tail = 0.0;
    }

    let abs_head = abs(head);
    if abs_head >= 0.5 && abs_head < 1.0 {
        return HDRFloat(head, tail, exp);
    }

    let bits = bitcast<u32>(head);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    let exp_adjust = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let new_head = bitcast<f32>(new_mantissa_bits | sign);
    let scale = hdr_exp2(-exp_adjust);
    let new_tail = tail * scale;

    return HDRFloat(new_head, new_tail, exp + exp_adjust);
}

fn hdr_neg(a: HDRFloat) -> HDRFloat {
    return HDRFloat(-a.head, -a.tail, a.exp);
}

fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 || b.head == 0.0 { return HDR_ZERO; }
    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;
    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));
}

fn hdr_square(a: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return HDR_ZERO; }
    let p = a.head * a.head;
    let err = fma(a.head, a.head, -p);
    let tail = err + 2.0 * a.head * a.tail;
    return hdr_normalize(HDRFloat(p, tail, a.exp * 2));
}

fn hdr_add(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return b; }
    if b.head == 0.0 { return a; }

    let exp_diff = a.exp - b.exp;
    if exp_diff > 48 { return a; }
    if exp_diff < -48 { return b; }

    var ah: f32; var at: f32; var bh: f32; var bt: f32; var result_exp: i32;

    if exp_diff >= 0 {
        let scale = hdr_exp2(-exp_diff);
        ah = a.head; at = a.tail;
        bh = b.head * scale; bt = b.tail * scale;
        result_exp = a.exp;
    } else {
        let scale = hdr_exp2(exp_diff);
        ah = a.head * scale; at = a.tail * scale;
        bh = b.head; bt = b.tail;
        result_exp = b.exp;
    }

    let sum = ah + bh;
    let err = hdr_two_sum_err(ah, bh, sum);
    let tail = err + at + bt;

    return hdr_normalize(HDRFloat(sum, tail, result_exp));
}

fn hdr_sub(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    return hdr_add(a, hdr_neg(b));
}

fn hdr_mul_f32(a: HDRFloat, b: f32) -> HDRFloat {
    if a.head == 0.0 || b == 0.0 { return HDR_ZERO; }
    let p = a.head * b;
    let err = fma(a.head, b, -p);
    let tail = err + a.tail * b;
    return hdr_normalize(HDRFloat(p, tail, a.exp));
}

fn hdr_to_f32(x: HDRFloat) -> f32 {
    if x.head == 0.0 { return 0.0; }
    let mantissa = x.head + x.tail;
    let clamped_exp = clamp(x.exp, -126, 127);
    return mantissa * hdr_exp2(clamped_exp);
}

fn hdr_complex_square(a: HDRComplex) -> HDRComplex {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let re_im = hdr_mul(a.re, a.im);
    let two_re_im = HDRFloat(re_im.head, re_im.tail, re_im.exp + 1);
    return HDRComplex(hdr_sub(re_sq, im_sq), two_re_im);
}

fn hdr_complex_norm_sq(a: HDRComplex) -> f32 {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let sum = hdr_add(re_sq, im_sq);
    return hdr_to_f32(sum);
}

fn hdr_from_parts(head: f32, tail: f32, exp: i32) -> HDRFloat {
    return HDRFloat(head, tail, exp);
}

// ============================================================
// Progressive Iteration Shader
// ============================================================

struct Uniforms {
    image_width: u32,
    image_height: u32,
    row_set_index: u32,
    row_set_count: u32,
    row_set_pixel_count: u32,
    _pad0: u32,

    chunk_start_iter: u32,
    chunk_size: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    _pad1: u32,

    dc_origin_re_head: f32,
    dc_origin_re_tail: f32,
    dc_origin_re_exp: i32,
    _pad2: u32,
    dc_origin_im_head: f32,
    dc_origin_im_tail: f32,
    dc_origin_im_exp: i32,
    _pad3: u32,

    dc_step_re_head: f32,
    dc_step_re_tail: f32,
    dc_step_re_exp: i32,
    _pad4: u32,
    dc_step_im_head: f32,
    dc_step_im_tail: f32,
    dc_step_im_exp: i32,
    _pad5: u32,

    reference_escaped: u32,
    orbit_len: u32,
    _pad6a: u32,
    _pad6b: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;

// Persistent state buffers - HDRFloat stored as 3 consecutive f32s (head, tail, exp as f32)
@group(0) @binding(2) var<storage, read_write> z_re: array<f32>;
@group(0) @binding(3) var<storage, read_write> z_im: array<f32>;
@group(0) @binding(4) var<storage, read_write> iter_count: array<u32>;
@group(0) @binding(5) var<storage, read_write> escaped_buf: array<u32>;
@group(0) @binding(6) var<storage, read_write> orbit_index: array<u32>;

// Result buffers
@group(0) @binding(7) var<storage, read_write> results: array<u32>;
@group(0) @binding(8) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(9) var<storage, read_write> z_norm_sq: array<f32>;

fn load_hdr(buf: ptr<storage, array<f32>, read_write>, idx: u32) -> HDRFloat {
    let base = idx * 3u;
    return HDRFloat((*buf)[base], (*buf)[base + 1u], i32(bitcast<u32>((*buf)[base + 2u])));
}

fn store_hdr(buf: ptr<storage, array<f32>, read_write>, idx: u32, val: HDRFloat) {
    let base = idx * 3u;
    (*buf)[base] = val.head;
    (*buf)[base + 1u] = val.tail;
    (*buf)[base + 2u] = bitcast<f32>(u32(val.exp));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let linear_idx = id.x;
    if linear_idx >= uniforms.row_set_pixel_count {
        return;
    }

    // Check if already escaped
    if escaped_buf[linear_idx] != 0u {
        return;
    }

    // Convert linear index to image coordinates
    let row_within_set = linear_idx / uniforms.image_width;
    let col = linear_idx % uniforms.image_width;
    let global_row = row_within_set * uniforms.row_set_count + uniforms.row_set_index;

    // Construct δc for this pixel
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    let x_hdr = HDRFloat(f32(col), 0.0, 0);
    let y_hdr = HDRFloat(f32(global_row), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    let dc = HDRComplex(dc_re, dc_im);

    // Load persistent state
    var dz = HDRComplex(load_hdr(&z_re, linear_idx), load_hdr(&z_im, linear_idx));
    var n = iter_count[linear_idx];
    var m = orbit_index[linear_idx];
    var glitched = glitch_flags[linear_idx] != 0u;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;
    let chunk_end = min(uniforms.chunk_start_iter + uniforms.chunk_size, uniforms.max_iterations);

    // Safety limit
    var loop_count = 0u;
    let max_loops = uniforms.chunk_size * 4u;

    loop {
        if n >= chunk_end {
            break;
        }

        loop_count = loop_count + 1u;
        if loop_count > max_loops {
            glitched = true;
            break;
        }

        // Reference exhaustion
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let z_m = reference_orbit[m % orbit_len];
        let z_m_re = z_m.x;
        let z_m_im = z_m.y;

        let z_m_hdr_re = HDRFloat(z_m_re, 0.0, 0);
        let z_m_hdr_im = HDRFloat(z_m_im, 0.0, 0);
        let z_re_full = hdr_add(z_m_hdr_re, dz.re);
        let z_im_full = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re_full, z_im_full);

        let z_mag_sq = hdr_complex_norm_sq(z);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = hdr_complex_norm_sq(dz);

        // Escape check
        if z_mag_sq > uniforms.escape_radius_sq {
            escaped_buf[linear_idx] = 1u;
            results[linear_idx] = n;
            glitch_flags[linear_idx] = select(0u, 1u, glitched);
            z_norm_sq[linear_idx] = z_mag_sq;
            store_hdr(&z_re, linear_idx, dz.re);
            store_hdr(&z_im, linear_idx, dz.im);
            iter_count[linear_idx] = n;
            orbit_index[linear_idx] = m;
            return;
        }

        // Glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration
        let two_z_dz_re = hdr_mul_f32(hdr_sub(hdr_mul_f32(dz.re, z_m_re), hdr_mul_f32(dz.im, z_m_im)), 2.0);
        let two_z_dz_im = hdr_mul_f32(hdr_add(hdr_mul_f32(dz.re, z_m_im), hdr_mul_f32(dz.im, z_m_re)), 2.0);
        let dz_sq = hdr_complex_square(dz);

        dz = HDRComplex(
            hdr_add(hdr_add(two_z_dz_re, dz_sq.re), dc.re),
            hdr_add(hdr_add(two_z_dz_im, dz_sq.im), dc.im)
        );

        m = m + 1u;
        n = n + 1u;
    }

    // Save state for next chunk
    store_hdr(&z_re, linear_idx, dz.re);
    store_hdr(&z_im, linear_idx, dz.im);
    iter_count[linear_idx] = n;
    orbit_index[linear_idx] = m;
    glitch_flags[linear_idx] = select(0u, 1u, glitched);

    // If we reached max_iterations, write final results
    if n >= uniforms.max_iterations {
        escaped_buf[linear_idx] = 1u;  // Mark as "done" even though didn't escape
        results[linear_idx] = uniforms.max_iterations;
        z_norm_sq[linear_idx] = 0.0;
    }
}
```

**Step 2: Verify shader syntax**

Run: `cargo check -p fractalwonder-gpu`
Expected: No errors (shader is included at compile time by pipeline)

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "$(cat <<'EOF'
feat(gpu): add progressive iteration WGSL shader

Row-set based shader with persistent state buffers for z, iter_count,
escaped, and orbit_index. Supports iteration chunking for timeout prevention.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Create ProgressiveGpuPipeline

**Files:**
- Create: `fractalwonder-gpu/src/progressive_pipeline.rs`

**Step 1: Create the pipeline file**

```rust
//! Compute pipeline for progressive row-set rendering.

/// Compute pipeline for progressive GPU rendering.
pub struct ProgressiveGpuPipeline {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl ProgressiveGpuPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("progressive_iteration"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/progressive_iteration.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("progressive_layout"),
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
                // binding 2: z_re (read-write storage)
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
                // binding 3: z_im (read-write storage)
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
                // binding 4: iter_count (read-write storage)
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
                // binding 5: escaped (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 6: orbit_index (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 7: results (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 8: glitch_flags (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 9: z_norm_sq (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
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
            label: Some("progressive_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("progressive_pipeline"),
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

In `fractalwonder-gpu/src/lib.rs`, add after line 8:

```rust
mod progressive_pipeline;
```

And add to exports after line 21:

```rust
pub use progressive_pipeline::ProgressiveGpuPipeline;
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/progressive_pipeline.rs fractalwonder-gpu/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(gpu): add ProgressiveGpuPipeline

Compute pipeline with 10 buffer bindings for progressive row-set
rendering with persistent state.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Create ProgressiveGpuRenderer

**Files:**
- Create: `fractalwonder-gpu/src/progressive_renderer.rs`

**Step 1: Create the renderer file**

```rust
//! Progressive GPU renderer for row-set based rendering.

use crate::buffers::{ProgressiveGpuBuffers, ProgressiveGpuUniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::progressive_pipeline::ProgressiveGpuPipeline;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a progressive GPU row-set render.
pub struct ProgressiveRowSetResult {
    pub data: Vec<ComputeData>,
    pub row_set_index: u32,
    pub compute_time_ms: f64,
}

/// Progressive GPU renderer using row-sets and iteration chunking.
pub struct ProgressiveGpuRenderer {
    context: GpuContext,
    pipeline: ProgressiveGpuPipeline,
    buffers: Option<ProgressiveGpuBuffers>,
    cached_orbit_id: Option<u32>,
    cached_row_set_pixel_count: u32,
}

impl ProgressiveGpuRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = ProgressiveGpuPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            cached_orbit_id: None,
            cached_row_set_pixel_count: 0,
        }
    }

    /// Calculate number of pixels in a row-set.
    pub fn calculate_row_set_pixel_count(
        image_width: u32,
        image_height: u32,
        row_set_count: u32,
    ) -> u32 {
        let rows_per_set = image_height.div_ceil(row_set_count);
        rows_per_set * image_width
    }

    /// Render a single row-set with iteration chunking.
    #[allow(clippy::too_many_arguments)]
    pub async fn render_row_set(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        image_width: u32,
        image_height: u32,
        row_set_index: u32,
        row_set_count: u32,
        max_iterations: u32,
        iterations_per_dispatch: u32,
        tau_sq: f32,
        reference_escaped: bool,
    ) -> Result<ProgressiveRowSetResult, GpuError> {
        let start = Self::now();

        let row_set_pixel_count =
            Self::calculate_row_set_pixel_count(image_width, image_height, row_set_count);

        // Recreate buffers if needed
        let needs_new_buffers = self
            .buffers
            .as_ref()
            .map(|b| b.orbit_capacity)
            .unwrap_or(0)
            < orbit.len() as u32
            || self.cached_row_set_pixel_count < row_set_pixel_count;

        if needs_new_buffers {
            log::info!(
                "Creating progressive buffers for orbit len {}, row_set pixels {}",
                orbit.len(),
                row_set_pixel_count
            );
            self.buffers = Some(ProgressiveGpuBuffers::new(
                &self.context.device,
                orbit.len() as u32,
                row_set_pixel_count,
            ));
            self.cached_orbit_id = None;
            self.cached_row_set_pixel_count = row_set_pixel_count;
        }

        let buffers = self.buffers.as_ref().unwrap();

        // Upload orbit if changed
        if self.cached_orbit_id != Some(orbit_id) {
            let orbit_data: Vec<[f32; 2]> = orbit
                .iter()
                .map(|&(re, im)| [re as f32, im as f32])
                .collect();
            self.context.queue.write_buffer(
                &buffers.reference_orbit,
                0,
                bytemuck::cast_slice(&orbit_data),
            );
            self.cached_orbit_id = Some(orbit_id);
        }

        // Clear state buffers for new row-set
        self.clear_state_buffers(row_set_pixel_count);

        // Iterate in chunks
        let chunk_count = max_iterations.div_ceil(iterations_per_dispatch);
        for chunk_idx in 0..chunk_count {
            let chunk_start = chunk_idx * iterations_per_dispatch;
            let chunk_size = iterations_per_dispatch.min(max_iterations - chunk_start);

            self.dispatch_chunk(
                image_width,
                image_height,
                row_set_index,
                row_set_count,
                row_set_pixel_count,
                chunk_start,
                chunk_size,
                max_iterations,
                tau_sq,
                dc_origin,
                dc_step,
                reference_escaped,
                orbit.len() as u32,
            );

            // Wait for dispatch to complete
            #[cfg(target_arch = "wasm32")]
            self.context.device.poll(wgpu::Maintain::Poll);

            #[cfg(not(target_arch = "wasm32"))]
            self.context.device.poll(wgpu::Maintain::Wait);
        }

        // Read back results
        let (iterations, glitch_data, z_norm_sq_data) = self
            .read_results(row_set_pixel_count as usize)
            .await?;

        // Convert to ComputeData
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(glitch_data.iter())
            .zip(z_norm_sq_data.iter())
            .map(|((&iter, &glitch_flag), &z_sq)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations,
                    glitched: glitch_flag != 0,
                    final_z_norm_sq: z_sq,
                })
            })
            .collect();

        let end = Self::now();

        Ok(ProgressiveRowSetResult {
            data,
            row_set_index,
            compute_time_ms: end - start,
        })
    }

    fn clear_state_buffers(&self, pixel_count: u32) {
        let buffers = self.buffers.as_ref().unwrap();

        // Zero out all state buffers
        let zeros_u32: Vec<u32> = vec![0; pixel_count as usize];
        let zeros_f32: Vec<f32> = vec![0.0; pixel_count as usize * 3]; // HDRFloat is 3 f32s

        self.context
            .queue
            .write_buffer(&buffers.z_re, 0, bytemuck::cast_slice(&zeros_f32));
        self.context
            .queue
            .write_buffer(&buffers.z_im, 0, bytemuck::cast_slice(&zeros_f32));
        self.context
            .queue
            .write_buffer(&buffers.iter_count, 0, bytemuck::cast_slice(&zeros_u32));
        self.context
            .queue
            .write_buffer(&buffers.escaped, 0, bytemuck::cast_slice(&zeros_u32));
        self.context
            .queue
            .write_buffer(&buffers.orbit_index, 0, bytemuck::cast_slice(&zeros_u32));
        self.context
            .queue
            .write_buffer(&buffers.glitch_flags, 0, bytemuck::cast_slice(&zeros_u32));
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_chunk(
        &self,
        image_width: u32,
        image_height: u32,
        row_set_index: u32,
        row_set_count: u32,
        row_set_pixel_count: u32,
        chunk_start_iter: u32,
        chunk_size: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        reference_escaped: bool,
        orbit_len: u32,
    ) {
        let buffers = self.buffers.as_ref().unwrap();

        let uniforms = ProgressiveGpuUniforms::new(
            image_width,
            image_height,
            row_set_index,
            row_set_count,
            row_set_pixel_count,
            chunk_start_iter,
            chunk_size,
            max_iterations,
            tau_sq,
            dc_origin,
            dc_step,
            reference_escaped,
            orbit_len,
        );

        self.context
            .queue
            .write_buffer(&buffers.uniforms, 0, bytemuck::bytes_of(&uniforms));

        let bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("progressive_bind_group"),
                layout: &self.pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffers.uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers.reference_orbit.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffers.z_re.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buffers.z_im.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffers.iter_count.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: buffers.escaped.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: buffers.orbit_index.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: buffers.results.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: buffers.glitch_flags.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: buffers.z_norm_sq.as_entire_binding(),
                    },
                ],
            });

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("progressive_encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("progressive_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch with workgroup size 64
            compute_pass.dispatch_workgroups(row_set_pixel_count.div_ceil(64), 1, 1);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));
    }

    async fn read_results(
        &self,
        count: usize,
    ) -> Result<(Vec<u32>, Vec<u32>, Vec<f32>), GpuError> {
        let buffers = self.buffers.as_ref().unwrap();

        // Copy to staging buffers
        let u32_byte_size = (count * std::mem::size_of::<u32>()) as u64;
        let f32_byte_size = (count * std::mem::size_of::<f32>()) as u64;

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("progressive_copy_encoder"),
                });

        encoder.copy_buffer_to_buffer(&buffers.results, 0, &buffers.staging_results, 0, u32_byte_size);
        encoder.copy_buffer_to_buffer(&buffers.glitch_flags, 0, &buffers.staging_glitches, 0, u32_byte_size);
        encoder.copy_buffer_to_buffer(&buffers.z_norm_sq, 0, &buffers.staging_z_norm_sq, 0, f32_byte_size);

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let results_slice = buffers.staging_results.slice(..u32_byte_size);
        let glitches_slice = buffers.staging_glitches.slice(..u32_byte_size);
        let z_norm_sq_slice = buffers.staging_z_norm_sq.slice(..f32_byte_size);

        let (tx1, rx1) = futures_channel::oneshot::channel();
        let (tx2, rx2) = futures_channel::oneshot::channel();
        let (tx3, rx3) = futures_channel::oneshot::channel();

        results_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx1.send(r); });
        glitches_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx2.send(r); });
        z_norm_sq_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx3.send(r); });

        #[cfg(target_arch = "wasm32")]
        self.context.device.poll(wgpu::Maintain::Poll);

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        rx1.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;
        rx2.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;
        rx3.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        let iterations: Vec<u32> = {
            let view = results_slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };
        let glitch_data: Vec<u32> = {
            let view = glitches_slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };
        let z_norm_sq_data: Vec<f32> = {
            let view = z_norm_sq_slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };

        buffers.staging_results.unmap();
        buffers.staging_glitches.unmap();
        buffers.staging_z_norm_sq.unmap();

        Ok((iterations, glitch_data, z_norm_sq_data))
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

**Step 2: Add module and exports to lib.rs**

In `fractalwonder-gpu/src/lib.rs`, add after progressive_pipeline module:

```rust
mod progressive_renderer;
```

And add to exports:

```rust
pub use buffers::{ProgressiveGpuBuffers, ProgressiveGpuUniforms};
pub use progressive_renderer::{ProgressiveGpuRenderer, ProgressiveRowSetResult};
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/progressive_renderer.rs fractalwonder-gpu/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(gpu): add ProgressiveGpuRenderer

Main renderer for progressive row-set based GPU rendering with
iteration chunking. Handles buffer management, dispatch loop, and
result readback.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Add Basic Unit Test for Progressive Renderer

**Files:**
- Modify: `fractalwonder-gpu/src/tests.rs`

**Step 1: Add test for progressive renderer**

Add at the end of the file:

```rust
/// Test that ProgressiveGpuRenderer initializes without panic.
#[test]
fn progressive_renderer_init_does_not_panic() {
    use crate::progressive_renderer::ProgressiveGpuRenderer;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };
        let _renderer = ProgressiveGpuRenderer::new(ctx);
        println!("ProgressiveGpuRenderer initialized successfully");
    });
}

/// Test progressive renderer produces correct results for simple case.
#[test]
fn progressive_renderer_basic_render() {
    use crate::progressive_renderer::ProgressiveGpuRenderer;

    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = ProgressiveGpuRenderer::new(ctx);

        let center_re = -0.5;
        let center_im = 0.0;
        let max_iter = 256;
        let tau_sq = 1e-6_f32;
        let width = 64_u32;
        let height = 64_u32;
        let row_set_count = 4_u32;
        let iterations_per_dispatch = 100_u32;

        let orbit = create_reference_orbit(center_re, center_im, max_iter);

        // Setup dc_origin and dc_step as HDRFloat tuples
        let view_width = 3.0_f32;
        let view_height = 3.0_f32;
        let dc_origin = (
            (-view_width / 2.0, 0.0, 0),
            (-view_height / 2.0, 0.0, 0),
        );
        let dc_step = (
            (view_width / width as f32, 0.0, 0),
            (view_height / height as f32, 0.0, 0),
        );

        // Render first row-set
        let result = renderer
            .render_row_set(
                &orbit.orbit,
                1,
                dc_origin,
                dc_step,
                width,
                height,
                0, // row_set_index
                row_set_count,
                max_iter,
                iterations_per_dispatch,
                tau_sq,
                orbit.escaped_at.is_some(),
            )
            .await
            .expect("Progressive render should succeed");

        let expected_pixels = ProgressiveGpuRenderer::calculate_row_set_pixel_count(
            width, height, row_set_count
        );

        assert_eq!(
            result.data.len(),
            expected_pixels as usize,
            "Should have correct number of pixels"
        );

        // Check that we have a mix of escaped and non-escaped pixels
        let escaped_count = result
            .data
            .iter()
            .filter(|d| as_mandelbrot(d).escaped)
            .count();

        println!("Progressive render test:");
        println!("  Row set 0: {} pixels", result.data.len());
        println!("  Escaped: {}", escaped_count);
        println!("  Compute time: {:.2}ms", result.compute_time_ms);

        assert!(escaped_count > 0, "Should have some escaped pixels");
        assert!(
            escaped_count < result.data.len(),
            "Should have some non-escaped pixels"
        );
    });
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-gpu progressive`
Expected: Tests pass (or skip if no GPU)

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/tests.rs
git commit -m "$(cat <<'EOF'
test(gpu): add tests for ProgressiveGpuRenderer

Basic init and render tests to verify progressive row-set rendering
produces expected results.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Run Full Test Suite

**Files:** None (verification only)

**Step 1: Run format check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Check build**

Run: `cargo check --workspace --all-targets --all-features`
Expected: No errors

---

## Future Tasks (Not in This Plan)

The following tasks are required to fully integrate progressive rendering but are out of scope for this initial implementation:

1. **Integrate ProgressiveGpuRenderer into parallel_renderer.rs** — Replace tile-based GPU calls with row-set calls
2. **Add cancellation support** — Abort render loop on viewport change
3. **Canvas update callback** — Colorize and display each row-set as it completes
4. **Row-set to pixel mapping** — Scatter row-set results back to full image buffer
5. **Performance benchmarking** — Compare tile-based vs progressive rendering speed
