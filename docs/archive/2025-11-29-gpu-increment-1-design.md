# GPU Increment 1: Infrastructure & Basic f32 Perturbation

> Design document for GPU-accelerated perturbation rendering up to ~10^7 zoom depth.

**Date:** 2025-11-29
**Status:** Draft
**Depends on:** Perturbation theory increments 1-4 (complete)

---

## Overview

Increment 1 establishes the wgpu pipeline and implements basic f32 delta iteration on GPU. This provides 50-200x speedup for shallow zoom renders while maintaining mathematical correctness validated against the CPU implementation.

**Scope:**
- GPU device initialization and capability detection
- Compute shader for f32 delta iteration with rebasing
- Glitch detection (Pauldelbrot criterion) on GPU
- Integration with existing worker pool (CPU handles reference orbit, glitch correction)
- Graceful fallback to CPU on any GPU failure

**Out of scope (future increments):**
- FloatExp extended precision (Increment 3)
- BLA acceleration on GPU (Increment 5)
- Progressive multi-pass rendering (Increment 2)

---

## Module Structure

New crate `fractalwonder-gpu` alongside existing crates:

```
fractalwonder-gpu/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API: GpuRenderer, GpuRenderResult
│   ├── device.rs           # Device/queue init, GpuContext, GpuAvailability
│   ├── buffers.rs          # Buffer management, Uniforms struct
│   ├── pipeline.rs         # Compute pipeline, bind group layout
│   └── shaders/
│       └── delta_iteration.wgsl
```

**Dependencies (Cargo.toml):**

```toml
[package]
name = "fractalwonder-gpu"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "23.0"
bytemuck = { version = "1.14", features = ["derive"] }
log = "0.4"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
```

**Rationale:** Separate crate isolates GPU-specific dependencies and compilation targets. The UI crate conditionally depends on it.

---

## Device Initialization

```rust
// device.rs

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub limits: wgpu::Limits,
}

pub enum GpuAvailability {
    Available(GpuContext),
    Unavailable(String),
}

impl GpuContext {
    pub async fn try_init() -> GpuAvailability {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let Some(adapter) = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await else {
            return GpuAvailability::Unavailable("No GPU adapter found".into());
        };

        let Ok((device, queue)) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("fractalwonder"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: Default::default(),
            },
            None,
        ).await else {
            return GpuAvailability::Unavailable("Failed to create device".into());
        };

        GpuAvailability::Available(GpuContext {
            limits: device.limits(),
            device,
            queue,
        })
    }
}
```

**Key decisions:**
- `Backends::all()` selects WebGPU on WASM, Vulkan/Metal/DX12 on native
- `downlevel_webgl2_defaults()` ensures maximum compatibility
- Any failure returns `Unavailable` with reason (no panics)

---

## Buffer Management

```rust
// buffers.rs

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,
    pub dc_origin_re: f32,
    pub dc_origin_im: f32,
    pub dc_step_re: f32,
    pub dc_step_im: f32,
    pub _padding: [u32; 3],  // Align to 16 bytes
}

pub struct GpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub orbit_length: u32,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub pixel_count: u32,
}

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, orbit_len: u32, width: u32, height: u32) -> Self {
        let pixel_count = width * height;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reference_orbit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference_orbit"),
            size: (orbit_len as usize * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let glitch_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glitch_flags"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            reference_orbit,
            orbit_length: orbit_len,
            results,
            glitch_flags,
            pixel_count,
        }
    }
}
```

**Key decision: delta-c computed on GPU.** Instead of uploading per-pixel values, upload `origin + step` and compute in shader:

```
dc = origin + step * pixel_index
```

This reduces upload bandwidth from O(pixels) to O(1).

**Memory budget (4K frame):**
- Results: 8.3M × 4 bytes = 33 MB
- Glitch flags: 8.3M × 4 bytes = 33 MB
- Reference orbit (1M iter): 1M × 8 bytes = 8 MB
- Total: ~74 MB (within 128 MB WebGPU minimum)

---

## Compute Shader

```wgsl
// shaders/delta_iteration.wgsl

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    dc_origin_re: f32,
    dc_origin_im: f32,
    dc_step_re: f32,
    dc_step_im: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Compute delta-c for this pixel
    let dc = vec2<f32>(
        uniforms.dc_origin_re + f32(gid.x) * uniforms.dc_step_re,
        uniforms.dc_origin_im + f32(gid.y) * uniforms.dc_step_im
    );

    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let Z = reference_orbit[m];
        let z = Z + dz;

        let z_sq = dot(z, z);
        let Z_sq = dot(Z, Z);
        let dz_sq = dot(dz, dz);

        // Escape check
        if (z_sq > uniforms.escape_radius_sq) {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            return;
        }

        // Pauldelbrot glitch detection: |z|^2 < tau^2 * |Z|^2
        if (Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq) {
            glitched = true;
        }

        // Rebase check: |z|^2 < |dz|^2
        if (z_sq < dz_sq) {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: dz' = 2*Z*dz + dz^2 + dc
        // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
        let two_Z_dz_re = 2.0 * (Z.x * dz.x - Z.y * dz.y);
        let two_Z_dz_im = 2.0 * (Z.x * dz.y + Z.y * dz.x);
        let dz_sq_re = dz.x * dz.x - dz.y * dz.y;
        let dz_sq_im = 2.0 * dz.x * dz.y;

        dz = vec2<f32>(
            two_Z_dz_re + dz_sq_re + dc.x,
            two_Z_dz_im + dz_sq_im + dc.y
        );

        m = m + 1u;
        if (m >= orbit_len) {
            m = 0u;
        }
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
}
```

**Workgroup size 8×8 = 64:** Standard GPU-efficient size within WebGPU limits.

**Dispatch:** `ceil(width/8) × ceil(height/8)` workgroups.

**Algorithm matches CPU exactly:** Same delta iteration formula, rebase condition, Pauldelbrot criterion.

---

## Pipeline Setup

```rust
// pipeline.rs

pub struct GpuPipeline {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("delta_iteration"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/delta_iteration.wgsl").into()
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("delta_iteration_layout"),
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("delta_iteration_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("delta_iteration_pipeline"),
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

---

## Public API

```rust
// lib.rs

pub use device::{GpuContext, GpuAvailability};
pub use buffers::{GpuBuffers, Uniforms};
pub use pipeline::GpuPipeline;

#[derive(Debug)]
pub enum GpuRenderError {
    BufferMapFailed(wgpu::BufferAsyncError),
    DeviceLost(String),
    OutOfMemory,
    ValidationError(String),
}

pub struct GpuRenderResult {
    pub iterations: Vec<u32>,
    pub glitch_flags: Vec<bool>,
    pub compute_time_ms: f64,
}

impl GpuRenderResult {
    pub fn has_glitches(&self) -> bool {
        self.glitch_flags.iter().any(|&g| g)
    }

    pub fn glitched_pixel_count(&self) -> usize {
        self.glitch_flags.iter().filter(|&&g| g).count()
    }
}

pub struct GpuRenderer {
    context: GpuContext,
    pipeline: GpuPipeline,
    buffers: Option<GpuBuffers>,
    cached_orbit_id: Option<u32>,
    current_dimensions: Option<(u32, u32)>,
}

impl GpuRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = GpuPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            cached_orbit_id: None,
            current_dimensions: None,
        }
    }

    pub async fn render(
        &mut self,
        orbit: &ReferenceOrbit,
        orbit_id: u32,
        dc_origin: (f32, f32),
        dc_step: (f32, f32),
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
    ) -> Result<GpuRenderResult, GpuRenderError> {
        let start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        // Recreate buffers if dimensions changed
        if self.current_dimensions != Some((width, height)) {
            self.buffers = Some(GpuBuffers::new(
                &self.context.device,
                orbit.orbit.len() as u32,
                width,
                height,
            ));
            self.current_dimensions = Some((width, height));
            self.cached_orbit_id = None; // Force orbit re-upload
        }

        let buffers = self.buffers.as_ref().unwrap();

        // Upload orbit if changed
        if self.cached_orbit_id != Some(orbit_id) {
            let orbit_data: Vec<[f32; 2]> = orbit.orbit
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

        // Write uniforms
        let uniforms = Uniforms {
            width,
            height,
            max_iterations,
            escape_radius_sq: 4.0,
            tau_sq,
            dc_origin_re: dc_origin.0,
            dc_origin_im: dc_origin.1,
            dc_step_re: dc_step.0,
            dc_step_im: dc_step.1,
            _padding: [0; 3],
        };
        self.context.queue.write_buffer(
            &buffers.uniforms,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // Create bind group
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("delta_iteration_bind_group"),
            layout: &self.pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.reference_orbit.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buffers.results.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.glitch_flags.as_entire_binding() },
            ],
        });

        // Dispatch compute shader
        let mut encoder = self.context.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline.compute_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(
                (width + 7) / 8,
                (height + 7) / 8,
                1,
            );
        }

        // Copy results to staging buffers for readback
        let pixel_count = (width * height) as usize;
        let staging_results = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_glitches = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_glitches"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &buffers.results, 0,
            &staging_results, 0,
            (pixel_count * std::mem::size_of::<u32>()) as u64,
        );
        encoder.copy_buffer_to_buffer(
            &buffers.glitch_flags, 0,
            &staging_glitches, 0,
            (pixel_count * std::mem::size_of::<u32>()) as u64,
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Map and read results
        let (tx, rx) = futures::channel::oneshot::channel();
        staging_results.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.context.device.poll(wgpu::Maintain::Wait);
        rx.await.unwrap().map_err(GpuRenderError::BufferMapFailed)?;

        let iterations: Vec<u32> = {
            let data = staging_results.slice(..).get_mapped_range();
            bytemuck::cast_slice(&data).to_vec()
        };
        staging_results.unmap();

        // Map and read glitch flags
        let (tx, rx) = futures::channel::oneshot::channel();
        staging_glitches.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.context.device.poll(wgpu::Maintain::Wait);
        rx.await.unwrap().map_err(GpuRenderError::BufferMapFailed)?;

        let glitch_flags: Vec<bool> = {
            let data = staging_glitches.slice(..).get_mapped_range();
            bytemuck::cast_slice::<_, u32>(&data)
                .iter()
                .map(|&v| v != 0)
                .collect()
        };
        staging_glitches.unmap();

        let end = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        Ok(GpuRenderResult {
            iterations,
            glitch_flags,
            compute_time_ms: end - start,
        })
    }
}
```

---

## Integration with UI

**Config change** (fractalwonder-core/src/config.rs):

```rust
pub struct Config {
    // ... existing fields
    pub gpu_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... existing defaults
            gpu_enabled: true,
        }
    }
}
```

**ParallelRenderer integration** (fractalwonder-ui/src/rendering/parallel_renderer.rs):

```rust
pub struct ParallelRenderer {
    worker_pool: WorkerPool,
    gpu_renderer: Option<GpuRenderer>,
}

impl ParallelRenderer {
    pub async fn new(config: &Config) -> Self {
        let gpu_renderer = if config.gpu_enabled {
            match GpuContext::try_init().await {
                GpuAvailability::Available(ctx) => {
                    log::info!("GPU rendering enabled");
                    Some(GpuRenderer::new(ctx))
                }
                GpuAvailability::Unavailable(reason) => {
                    log::warn!("GPU unavailable: {reason}, using CPU fallback");
                    None
                }
            }
        } else {
            log::info!("GPU rendering disabled by config");
            None
        };

        Self {
            worker_pool: WorkerPool::new(4),
            gpu_renderer,
        }
    }

    pub async fn render(&mut self, viewport: &Viewport, canvas_size: (u32, u32)) {
        // 1. Compute reference orbit on CPU (always)
        let orbit = self.worker_pool.compute_reference_orbit(viewport).await;

        // 2. Try GPU path
        if let Some(gpu) = &mut self.gpu_renderer {
            match gpu.render(/* params */).await {
                Ok(result) => {
                    // 3. Re-render glitched pixels on CPU
                    if result.has_glitches() {
                        let glitched_tiles = self.extract_glitched_regions(&result);
                        self.worker_pool.render_tiles(glitched_tiles, &orbit).await;
                    }

                    self.colorize_and_draw(&result);
                    return;
                }
                Err(e) => {
                    log::error!("GPU render failed: {e:?}, falling back to CPU");
                }
            }
        }

        // 4. Fallback: full CPU rendering
        self.worker_pool.render_all_tiles(viewport, canvas_size, &orbit).await;
    }
}
```

**Key flow:**
1. Reference orbit computed on CPU worker (BigFloat precision)
2. GPU renders all pixels in one dispatch
3. Glitched pixels re-rendered on CPU workers
4. Any GPU error falls back to full CPU rendering

---

## Test Strategy

Tests validate mathematical correctness against CPU as ground truth.

### Test 1: Bit-exact match at low iterations

For n < 100 iterations, GPU and CPU must produce identical counts.

```rust
#[test]
fn iteration_counts_match_cpu_low_iter() {
    let coords = generate_random_coords(1000, 1e0..1e7);
    for (c_ref, dc) in coords {
        let cpu = compute_pixel_perturbation(&orbit, dc, 100);
        let gpu = gpu_render_single_pixel(&orbit, dc, 100);
        assert_eq!(cpu.iterations, gpu.iterations);
    }
}
```

### Test 2: Statistical agreement at high iterations

Average iteration difference < 0.1% (GPU may differ ±1 due to f32 vs f64).

```rust
#[test]
fn statistical_agreement_high_iter() {
    let cpu = cpu_render_frame(viewport, 10000);
    let gpu = gpu_render_frame(viewport, 10000);
    let diff = average_iteration_diff(&cpu, &gpu);
    assert!(diff < 0.001);
}
```

### Test 3: Rebase triggers at same iteration

```rust
#[test]
fn rebase_triggers_match() {
    let cpu_rebases = trace_cpu_rebases(coord);
    let gpu_rebases = trace_gpu_rebases(coord);
    assert_eq!(cpu_rebases, gpu_rebases);
}
```

### Test 4: No false escapes

GPU-only escapes indicate precision loss.

```rust
#[test]
fn no_false_escapes() {
    let cpu = cpu_render_frame(viewport, max_iter);
    let gpu = gpu_render_frame(viewport, max_iter);
    for (c, g) in cpu.iter().zip(gpu.iter()) {
        assert!(!(g.escaped && !c.escaped), "False escape");
    }
}
```

### Test 5: Glitch masks match

```rust
#[test]
fn glitch_masks_match() {
    let cpu_glitches = cpu_glitch_detection(viewport);
    let gpu_glitches = gpu_render_frame(viewport).glitch_flags;
    assert_eq!(cpu_glitches, gpu_glitches);
}
```

### Test 6: Performance (10x minimum speedup)

```rust
#[test]
fn gpu_10x_faster() {
    let cpu_ms = benchmark(|| cpu_render(1000, 1000, 1000));
    let gpu_ms = benchmark(|| gpu_render(1000, 1000, 1000));
    assert!(cpu_ms / gpu_ms >= 10.0);
}
```

---

## Error Handling

**Philosophy:** GPU failure is not catastrophic. Log and fall back to CPU.

```rust
#[derive(Debug)]
pub enum GpuRenderError {
    BufferMapFailed(wgpu::BufferAsyncError),
    DeviceLost(String),
    OutOfMemory,
    ValidationError(String),
}
```

All wgpu operations use `?` or explicit error handling. No panics in the GPU crate.

---

## Acceptance Criteria

- [ ] GPU iteration counts match CPU within tolerance at 10^5 zoom
- [ ] Glitch detection produces identical pixel masks on CPU and GPU
- [ ] Documented speedup of at least 10x for 1M pixel image
- [ ] All existing CPU tests still pass
- [ ] GPU path disabled via `Config::gpu_enabled = false`
- [ ] Graceful fallback on any GPU error

---

## Summary

| Component | Decision |
|-----------|----------|
| Module | New `fractalwonder-gpu` crate |
| Dependencies | wgpu 23.0, bytemuck |
| Shader | f32 delta iteration, 8×8 workgroups |
| δc values | Computed on GPU from origin+step |
| Integration | Config-driven, alongside worker pool |
| Fallback | Any GPU error → CPU path |
| Tests | CPU as ground truth, 6 validation tests |
