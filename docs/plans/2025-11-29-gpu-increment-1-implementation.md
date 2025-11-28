# GPU Increment 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add GPU-accelerated f32 perturbation rendering up to ~10^7 zoom depth.

**Architecture:** New `fractalwonder-gpu` crate with wgpu compute shader. GPU renders all pixels in one dispatch, CPU handles reference orbit and glitch correction. Config-driven enablement with graceful CPU fallback.

**Tech Stack:** wgpu 23.0, bytemuck, WGSL compute shaders

**Design Document:** `docs/plans/2025-11-29-gpu-increment-1-design.md`

---

## Task 1: Create fractalwonder-gpu Crate

**Files:**
- Create: `fractalwonder-gpu/Cargo.toml`
- Create: `fractalwonder-gpu/src/lib.rs`
- Modify: `Cargo.toml` (workspace)

**Step 1: Create crate directory**

```bash
mkdir -p fractalwonder-gpu/src
```

**Step 2: Create Cargo.toml**

```toml
[package]
name = "fractalwonder-gpu"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "23.0"
bytemuck = { version = "1.14", features = ["derive"] }
log = "0.4"
thiserror = "1.0"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Window", "Performance"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
pollster = "0.4"
```

**Step 3: Create minimal lib.rs**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod device;

pub use device::{GpuContext, GpuAvailability};
```

**Step 4: Create placeholder device.rs**

Create `fractalwonder-gpu/src/device.rs`:

```rust
//! GPU device initialization and capability detection.

/// Holds the wgpu device and queue.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Result of GPU initialization attempt.
pub enum GpuAvailability {
    Available(GpuContext),
    Unavailable(String),
}
```

**Step 5: Add to workspace**

Modify root `Cargo.toml`, add to members:

```toml
members = [
    "fractalwonder-core",
    "fractalwonder-compute",
    "fractalwonder-ui",
    "fractalwonder-gpu",
]
```

**Step 6: Verify crate compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 7: Commit**

```bash
git add fractalwonder-gpu Cargo.toml
git commit -m "feat(gpu): create fractalwonder-gpu crate skeleton"
```

---

## Task 2: Implement GpuContext Initialization

**Files:**
- Modify: `fractalwonder-gpu/src/device.rs`
- Create: `fractalwonder-gpu/src/error.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create error types**

Create `fractalwonder-gpu/src/error.rs`:

```rust
//! GPU error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("No GPU adapter found")]
    NoAdapter,

    #[error("Failed to create device: {0}")]
    DeviceCreation(#[from] wgpu::RequestDeviceError),

    #[error("Buffer mapping failed: {0}")]
    BufferMap(#[from] wgpu::BufferAsyncError),

    #[error("GPU unavailable: {0}")]
    Unavailable(String),
}
```

**Step 2: Implement GpuContext::try_init**

Modify `fractalwonder-gpu/src/device.rs`:

```rust
//! GPU device initialization and capability detection.

use crate::error::GpuError;

/// Holds the wgpu device and queue.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Result of GPU initialization attempt.
pub enum GpuAvailability {
    Available(GpuContext),
    Unavailable(String),
}

impl GpuContext {
    /// Attempt to initialize GPU. Returns Unavailable on any failure.
    pub async fn try_init() -> GpuAvailability {
        match Self::init_internal().await {
            Ok(ctx) => GpuAvailability::Available(ctx),
            Err(e) => {
                log::warn!("GPU initialization failed: {e}");
                GpuAvailability::Unavailable(e.to_string())
            }
        }
    }

    async fn init_internal() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        log::info!("GPU adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("fractalwonder"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        Ok(Self { device, queue })
    }
}
```

**Step 3: Update lib.rs exports**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod device;
mod error;

pub use device::{GpuContext, GpuAvailability};
pub use error::GpuError;
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src
git commit -m "feat(gpu): implement GpuContext initialization"
```

---

## Task 3: Implement Uniforms and Buffer Types

**Files:**
- Create: `fractalwonder-gpu/src/buffers.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create buffers.rs with Uniforms struct**

Create `fractalwonder-gpu/src/buffers.rs`:

```rust
//! GPU buffer management for compute shader.

use bytemuck::{Pod, Zeroable};

/// Uniform data passed to compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
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
    pub _padding: [u32; 3],
}

impl Uniforms {
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: (f32, f32),
        dc_step: (f32, f32),
    ) -> Self {
        Self {
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
        }
    }
}

/// Manages GPU buffers for rendering.
pub struct GpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub orbit_capacity: u32,
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

        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_glitches = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_glitches"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            reference_orbit,
            results,
            glitch_flags,
            staging_results,
            staging_glitches,
            orbit_capacity: orbit_len,
            pixel_count,
        }
    }
}
```

**Step 2: Update lib.rs exports**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuContext, GpuAvailability};
pub use error::GpuError;
```

**Step 3: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src
git commit -m "feat(gpu): add Uniforms and GpuBuffers"
```

---

## Task 4: Create WGSL Compute Shader

**Files:**
- Create: `fractalwonder-gpu/src/shaders/delta_iteration.wgsl`

**Step 1: Create shaders directory**

```bash
mkdir -p fractalwonder-gpu/src/shaders
```

**Step 2: Write compute shader**

Create `fractalwonder-gpu/src/shaders/delta_iteration.wgsl`:

```wgsl
// Delta iteration compute shader for f32 perturbation rendering.

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

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders
git commit -m "feat(gpu): add WGSL delta iteration compute shader"
```

---

## Task 5: Implement Compute Pipeline

**Files:**
- Create: `fractalwonder-gpu/src/pipeline.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create pipeline.rs**

Create `fractalwonder-gpu/src/pipeline.rs`:

```rust
//! Compute pipeline for delta iteration.

/// Compute pipeline and bind group layout for delta iteration.
pub struct GpuPipeline {
    pub compute_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("delta_iteration"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/delta_iteration.wgsl").into(),
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

**Step 2: Update lib.rs exports**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pipeline;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pipeline::GpuPipeline;
```

**Step 3: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src
git commit -m "feat(gpu): implement compute pipeline"
```

---

## Task 6: Implement GpuRenderer

**Files:**
- Create: `fractalwonder-gpu/src/renderer.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create renderer.rs**

Create `fractalwonder-gpu/src/renderer.rs`:

```rust
//! High-level GPU renderer API.

use crate::buffers::{GpuBuffers, Uniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::pipeline::GpuPipeline;

/// Result of a GPU render operation.
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

/// GPU renderer for Mandelbrot perturbation.
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

    /// Render the Mandelbrot set using GPU compute shader.
    ///
    /// # Arguments
    /// * `orbit` - Reference orbit as slice of (re, im) pairs
    /// * `orbit_id` - ID to track orbit changes (skip re-upload if unchanged)
    /// * `dc_origin` - Delta-c at top-left pixel
    /// * `dc_step` - Delta-c step per pixel
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `max_iterations` - Maximum iteration count
    /// * `tau_sq` - Pauldelbrot threshold squared
    pub async fn render(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: (f32, f32),
        dc_step: (f32, f32),
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
    ) -> Result<GpuRenderResult, GpuError> {
        let start = Self::now();

        // Recreate buffers if dimensions changed
        if self.current_dimensions != Some((width, height))
            || self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0) < orbit.len() as u32
        {
            self.buffers = Some(GpuBuffers::new(
                &self.context.device,
                orbit.len() as u32,
                width,
                height,
            ));
            self.current_dimensions = Some((width, height));
            self.cached_orbit_id = None;
        }

        let buffers = self.buffers.as_ref().unwrap();

        // Upload orbit if changed
        if self.cached_orbit_id != Some(orbit_id) {
            let orbit_data: Vec<[f32; 2]> =
                orbit.iter().map(|&(re, im)| [re as f32, im as f32]).collect();
            self.context.queue.write_buffer(
                &buffers.reference_orbit,
                0,
                bytemuck::cast_slice(&orbit_data),
            );
            self.cached_orbit_id = Some(orbit_id);
        }

        // Write uniforms
        let uniforms = Uniforms::new(width, height, max_iterations, tau_sq, dc_origin, dc_step);
        self.context
            .queue
            .write_buffer(&buffers.uniforms, 0, bytemuck::bytes_of(&uniforms));

        // Create bind group
        let bind_group = self.context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("delta_iteration_bind_group"),
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
                    resource: buffers.results.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.glitch_flags.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute shader
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("delta_iteration_encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("delta_iteration_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline.compute_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
        }

        // Copy results to staging buffers
        let pixel_count = (width * height) as usize;
        let byte_size = (pixel_count * std::mem::size_of::<u32>()) as u64;

        encoder.copy_buffer_to_buffer(&buffers.results, 0, &buffers.staging_results, 0, byte_size);
        encoder.copy_buffer_to_buffer(
            &buffers.glitch_flags,
            0,
            &buffers.staging_glitches,
            0,
            byte_size,
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let iterations = self.read_buffer(&buffers.staging_results, pixel_count).await?;
        let glitch_data = self.read_buffer(&buffers.staging_glitches, pixel_count).await?;
        let glitch_flags: Vec<bool> = glitch_data.iter().map(|&v| v != 0).collect();

        let end = Self::now();

        Ok(GpuRenderResult {
            iterations,
            glitch_flags,
            compute_time_ms: end - start,
        })
    }

    async fn read_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<u32>, GpuError> {
        let slice = buffer.slice(..);

        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

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

**Step 2: Add futures-channel dependency**

Add to `fractalwonder-gpu/Cargo.toml`:

```toml
[dependencies]
futures-channel = "0.3"
```

**Step 3: Update lib.rs exports**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pipeline;
mod renderer;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add fractalwonder-gpu
git commit -m "feat(gpu): implement GpuRenderer with render method"
```

---

## Task 7: Add gpu_enabled to Config

**Files:**
- Modify: `fractalwonder-core/src/config.rs`

**Step 1: Find existing Config struct**

```bash
grep -n "struct Config" fractalwonder-core/src/config.rs
```

**Step 2: Add gpu_enabled field**

Add field to Config struct:

```rust
pub gpu_enabled: bool,
```

**Step 3: Add to Default impl**

```rust
gpu_enabled: true,
```

**Step 4: Verify compiles**

```bash
cargo check -p fractalwonder-core
```

Expected: Compiles with no errors.

**Step 5: Commit**

```bash
git add fractalwonder-core/src/config.rs
git commit -m "feat(config): add gpu_enabled setting"
```

---

## Task 8: Write Unit Test for GpuRenderer

**Files:**
- Create: `fractalwonder-gpu/src/tests.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Create tests module**

Create `fractalwonder-gpu/src/tests.rs`:

```rust
//! Tests for GPU renderer.

#[cfg(test)]
mod tests {
    use crate::{GpuAvailability, GpuContext, GpuRenderer};

    /// Test that GPU initialization doesn't panic.
    #[test]
    fn gpu_init_does_not_panic() {
        // This test verifies the initialization code path runs without panic.
        // On systems without GPU, it should return Unavailable gracefully.
        pollster::block_on(async {
            let result = GpuContext::try_init().await;
            match result {
                GpuAvailability::Available(_) => {
                    println!("GPU available");
                }
                GpuAvailability::Unavailable(reason) => {
                    println!("GPU unavailable: {reason}");
                }
            }
        });
    }

    /// Test basic render on GPU (if available).
    #[test]
    fn gpu_render_basic() {
        pollster::block_on(async {
            let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
                println!("Skipping test: no GPU available");
                return;
            };

            let mut renderer = GpuRenderer::new(ctx);

            // Simple reference orbit: z=0 -> z=c -> z=c^2+c -> ...
            // For c = (0, 0), orbit is all zeros
            let orbit = vec![(0.0_f64, 0.0_f64); 100];

            let result = renderer
                .render(
                    &orbit,
                    1,                   // orbit_id
                    (-2.0, -1.5),        // dc_origin
                    (0.01, 0.01),        // dc_step
                    100,                 // width
                    100,                 // height
                    100,                 // max_iterations
                    1e-6,                // tau_sq
                )
                .await
                .expect("Render should succeed");

            assert_eq!(result.iterations.len(), 100 * 100);
            assert_eq!(result.glitch_flags.len(), 100 * 100);
            println!("GPU render completed in {:.2}ms", result.compute_time_ms);
        });
    }
}
```

**Step 2: Add tests module to lib.rs**

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pipeline;
mod renderer;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
```

**Step 3: Run tests**

```bash
cargo test -p fractalwonder-gpu -- --nocapture
```

Expected: Tests pass (or skip gracefully if no GPU).

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src
git commit -m "test(gpu): add basic GPU renderer tests"
```

---

## Task 9: Verify Full Workspace Build

**Files:** None (verification only)

**Step 1: Run cargo check on workspace**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: No errors.

**Step 2: Run cargo clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: No warnings.

**Step 3: Run cargo fmt**

```bash
cargo fmt --all -- --check
```

Expected: No formatting issues (or run `cargo fmt --all` to fix).

**Step 4: Run all tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All tests pass.

**Step 5: Commit any fixes if needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Create crate skeleton | `fractalwonder-gpu/Cargo.toml`, `lib.rs` |
| 2 | GpuContext initialization | `device.rs`, `error.rs` |
| 3 | Uniforms and buffers | `buffers.rs` |
| 4 | WGSL compute shader | `shaders/delta_iteration.wgsl` |
| 5 | Compute pipeline | `pipeline.rs` |
| 6 | GpuRenderer API | `renderer.rs` |
| 7 | Config setting | `fractalwonder-core/src/config.rs` |
| 8 | Unit tests | `tests.rs` |
| 9 | Full workspace verification | - |

**After completion:** GPU crate is ready. Next step is UI integration (separate plan).
