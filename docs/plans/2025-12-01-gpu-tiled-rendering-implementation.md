# GPU Tiled Progressive Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Adam7 progressive rendering with spatial tiling to prevent GPU timeout at deep zoom levels.

**Architecture:** Fixed 64×64 tiles rendered center-out. Each tile is a separate GPU dispatch with tile-sized buffers. Results accumulate into full-image CPU buffer. Quick colorize per tile for progressive display, full pipeline at end.

**Tech Stack:** Rust, wgpu, WGSL shaders, fractalwonder-gpu crate, fractalwonder-ui crate.

---

## Task 1: Update Uniforms for Tile Rendering

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:118-195` (PerturbationHDRUniforms)

**Step 1: Add tile offset fields to PerturbationHDRUniforms**

Replace `adam7_step` with tile offset fields. The struct becomes:

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PerturbationHDRUniforms {
    pub image_width: u32,       // Full image width (for δc calculation)
    pub image_height: u32,      // Full image height
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,
    pub _pad0: u32,

    // dc_origin as HDRFloat (unchanged)
    pub dc_origin_re_head: f32,
    pub dc_origin_re_tail: f32,
    pub dc_origin_re_exp: i32,
    pub _pad1: u32,
    pub dc_origin_im_head: f32,
    pub dc_origin_im_tail: f32,
    pub dc_origin_im_exp: i32,
    pub _pad2: u32,

    // dc_step as HDRFloat (unchanged)
    pub dc_step_re_head: f32,
    pub dc_step_re_tail: f32,
    pub dc_step_re_exp: i32,
    pub _pad3: u32,
    pub dc_step_im_head: f32,
    pub dc_step_im_tail: f32,
    pub dc_step_im_exp: i32,

    // Tile bounds (replaces adam7_step)
    pub tile_offset_x: u32,
    pub tile_offset_y: u32,
    pub tile_width: u32,
    pub tile_height: u32,

    pub reference_escaped: u32,
    pub orbit_len: u32,
    pub _pad4: vec2<u32>,  // Pad to 16-byte alignment
}
```

**Step 2: Update PerturbationHDRUniforms::new()**

```rust
impl PerturbationHDRUniforms {
    #[allow(dead_code, clippy::too_many_arguments)]
    pub fn new(
        image_width: u32,
        image_height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        tile_offset_x: u32,
        tile_offset_y: u32,
        tile_width: u32,
        tile_height: u32,
        reference_escaped: bool,
        orbit_len: u32,
    ) -> Self {
        Self {
            image_width,
            image_height,
            max_iterations,
            escape_radius_sq: 65536.0,
            tau_sq,
            _pad0: 0,
            dc_origin_re_head: dc_origin.0 .0,
            dc_origin_re_tail: dc_origin.0 .1,
            dc_origin_re_exp: dc_origin.0 .2,
            _pad1: 0,
            dc_origin_im_head: dc_origin.1 .0,
            dc_origin_im_tail: dc_origin.1 .1,
            dc_origin_im_exp: dc_origin.1 .2,
            _pad2: 0,
            dc_step_re_head: dc_step.0 .0,
            dc_step_re_tail: dc_step.0 .1,
            dc_step_re_exp: dc_step.0 .2,
            _pad3: 0,
            dc_step_im_head: dc_step.1 .0,
            dc_step_im_tail: dc_step.1 .1,
            dc_step_im_exp: dc_step.1 .2,
            tile_offset_x,
            tile_offset_y,
            tile_width,
            tile_height,
            reference_escaped: if reference_escaped { 1 } else { 0 },
            orbit_len,
            _pad4: [0, 0],
        }
    }
}
```

**Step 3: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: Compilation errors in files using old signature (will fix in later tasks).

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "refactor(gpu): update uniforms for tile-based rendering"
```

---

## Task 2: Update Shader for Tile Rendering

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl:198-413`

**Step 1: Update Uniforms struct in shader**

Replace the existing Uniforms struct (lines 198-227) with:

```wgsl
struct Uniforms {
    image_width: u32,
    image_height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    _pad0: u32,

    dc_origin_re_head: f32,
    dc_origin_re_tail: f32,
    dc_origin_re_exp: i32,
    _pad1: u32,
    dc_origin_im_head: f32,
    dc_origin_im_tail: f32,
    dc_origin_im_exp: i32,
    _pad2: u32,

    dc_step_re_head: f32,
    dc_step_re_tail: f32,
    dc_step_re_exp: i32,
    _pad3: u32,
    dc_step_im_head: f32,
    dc_step_im_tail: f32,
    dc_step_im_exp: i32,

    tile_offset_x: u32,
    tile_offset_y: u32,
    tile_width: u32,
    tile_height: u32,

    reference_escaped: u32,
    orbit_len: u32,
    _pad4: vec2<u32>,
}
```

**Step 2: Remove Adam7 functions**

Delete the `adam7_coords` and `adam7_step_size` functions (lines 238-262).

**Step 3: Update main() function**

Replace the main function. Key changes:
- Use `local_id` for tile-local position
- Compute `global_x/y` = tile_offset + local position
- Bounds check against tile dimensions
- Use global position for δc calculation
- Write to tile-local buffer index

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) local_id: vec3<u32>) {
    let local_x = local_id.x;
    let local_y = local_id.y;

    // Bounds check against tile size
    if local_x >= uniforms.tile_width || local_y >= uniforms.tile_height {
        return;
    }

    // Global pixel position for δc calculation
    let global_x = uniforms.tile_offset_x + local_x;
    let global_y = uniforms.tile_offset_y + local_y;

    // Tile-local buffer index
    let tile_idx = local_y * uniforms.tile_width + local_x;

    // Construct δc for this pixel using GLOBAL position
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    // δc = dc_origin + global_pixel_pos * dc_step
    let x_hdr = HDRFloat(f32(global_x), 0.0, 0);
    let y_hdr = HDRFloat(f32(global_y), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    let dc = HDRComplex(dc_re, dc_im);

    // δz starts at origin
    var dz = HDR_COMPLEX_ZERO;
    var m: u32 = 0u;
    var glitched: bool = false;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;

    var n: u32 = 0u;
    var total_loops: u32 = 0u;
    let max_total_loops = uniforms.max_iterations * 4u;

    loop {
        if n >= uniforms.max_iterations {
            break;
        }

        total_loops = total_loops + 1u;
        if total_loops > max_total_loops {
            glitched = true;
            break;
        }

        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let z_m = reference_orbit[m % orbit_len];
        let z_m_re = z_m.x;
        let z_m_im = z_m.y;

        let z_m_hdr_re = HDRFloat(z_m_re, 0.0, 0);
        let z_m_hdr_im = HDRFloat(z_m_im, 0.0, 0);
        let z_re = hdr_add(z_m_hdr_re, dz.re);
        let z_im = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re, z_im);

        let z_mag_sq = hdr_complex_norm_sq(z);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = hdr_complex_norm_sq(dz);

        // Escape check
        if z_mag_sq > uniforms.escape_radius_sq {
            results[tile_idx] = n;
            glitch_flags[tile_idx] = select(0u, 1u, glitched);
            z_norm_sq[tile_idx] = z_mag_sq;
            return;
        }

        // Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: δz' = 2·Z_m·δz + δz² + δc
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

    // Reached max iterations
    results[tile_idx] = uniforms.max_iterations;
    glitch_flags[tile_idx] = select(0u, 1u, glitched);
    z_norm_sq[tile_idx] = 0.0;
}
```

**Step 4: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: Still compilation errors (renderer not updated yet).

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl
git commit -m "refactor(gpu): update shader for tile-based rendering"
```

---

## Task 3: Update GPU Buffers for Fixed Tile Size

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:197-284` (PerturbationHDRBuffers)

**Step 1: Add tile size constant and update buffer allocation**

```rust
/// Maximum tile size for GPU rendering (64×64 = 4096 pixels).
pub const GPU_TILE_SIZE: u32 = 64;
pub const GPU_TILE_PIXELS: u32 = GPU_TILE_SIZE * GPU_TILE_SIZE;

/// GPU buffers for perturbation HDRFloat rendering.
/// Buffers are sized for a single tile (64×64), reused across tile dispatches.
pub struct PerturbationHDRBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,
    pub orbit_capacity: u32,
}

impl PerturbationHDRBuffers {
    /// Create tile-sized buffers. Orbit buffer sized for orbit_len.
    pub fn new(device: &wgpu::Device, orbit_len: u32) -> Self {
        let tile_pixels = GPU_TILE_PIXELS as usize;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_uniforms"),
            size: std::mem::size_of::<PerturbationHDRUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reference_orbit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_reference_orbit"),
            size: (orbit_len as usize * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_results"),
            size: (tile_pixels * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let glitch_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_glitch_flags"),
            size: (tile_pixels * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_staging_results"),
            size: (tile_pixels * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_glitches = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_staging_glitches"),
            size: (tile_pixels * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_z_norm_sq"),
            size: (tile_pixels * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perturbation_hdr_staging_z_norm_sq"),
            size: (tile_pixels * std::mem::size_of::<f32>()) as u64,
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
            z_norm_sq,
            staging_z_norm_sq,
            orbit_capacity: orbit_len,
        }
    }
}
```

**Step 2: Export GPU_TILE_SIZE from lib.rs**

Add to `fractalwonder-gpu/src/lib.rs`:

```rust
pub use buffers::{GpuBuffers, PerturbationHDRBuffers, PerturbationHDRUniforms, Uniforms, GPU_TILE_SIZE};
```

**Step 3: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs fractalwonder-gpu/src/lib.rs
git commit -m "refactor(gpu): use fixed tile-sized buffers"
```

---

## Task 4: Update Renderer for Tile-Based Rendering

**Files:**
- Modify: `fractalwonder-gpu/src/perturbation_hdr_renderer.rs`

**Step 1: Update render() signature and implementation**

Replace the `render()` method to accept a tile (`PixelRect`) and return tile-sized results:

```rust
use crate::buffers::{PerturbationHDRBuffers, PerturbationHDRUniforms, GPU_TILE_SIZE};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::perturbation_hdr_pipeline::PerturbationHDRPipeline;
use fractalwonder_core::{ComputeData, MandelbrotData, PixelRect};

/// Result of a GPU tile render operation.
pub struct GpuTileResult {
    pub data: Vec<ComputeData>,
    pub tile: PixelRect,
    pub compute_time_ms: f64,
}

/// GPU renderer for Mandelbrot perturbation with HDRFloat deltas.
pub struct GpuPerturbationHDRRenderer {
    context: GpuContext,
    pipeline: PerturbationHDRPipeline,
    buffers: Option<PerturbationHDRBuffers>,
    cached_orbit_id: Option<u32>,
}

impl GpuPerturbationHDRRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = PerturbationHDRPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            cached_orbit_id: None,
        }
    }

    /// Render a single tile.
    ///
    /// # Arguments
    /// * `orbit` - Reference orbit as (re, im) pairs
    /// * `orbit_id` - ID for orbit caching
    /// * `dc_origin` - Top-left δc for full image as HDRFloat tuples
    /// * `dc_step` - Per-pixel δc step as HDRFloat tuples
    /// * `image_width` - Full image width
    /// * `image_height` - Full image height
    /// * `tile` - Tile bounds in pixel coordinates
    /// * `max_iterations` - Maximum iteration count
    /// * `tau_sq` - Glitch detection threshold
    /// * `reference_escaped` - Whether reference orbit escaped
    #[allow(clippy::too_many_arguments)]
    pub async fn render_tile(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        image_width: u32,
        image_height: u32,
        tile: &PixelRect,
        max_iterations: u32,
        tau_sq: f32,
        reference_escaped: bool,
    ) -> Result<GpuTileResult, GpuError> {
        let start = Self::now();

        // Recreate buffers if orbit capacity changed
        if self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0) < orbit.len() as u32 {
            self.buffers = Some(PerturbationHDRBuffers::new(
                &self.context.device,
                orbit.len() as u32,
            ));
            self.cached_orbit_id = None;
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

        // Write uniforms with tile bounds
        let uniforms = PerturbationHDRUniforms::new(
            image_width,
            image_height,
            max_iterations,
            tau_sq,
            dc_origin,
            dc_step,
            tile.x,
            tile.y,
            tile.width,
            tile.height,
            reference_escaped,
            orbit.len() as u32,
        );

        self.context
            .queue
            .write_buffer(&buffers.uniforms, 0, bytemuck::bytes_of(&uniforms));

        // Create bind group
        let bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("perturbation_hdr_bind_group"),
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffers.z_norm_sq.as_entire_binding(),
                    },
                ],
            });

        // Dispatch compute shader for tile
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("perturbation_hdr_encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("perturbation_hdr_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch workgroups for tile size
            compute_pass.dispatch_workgroups(
                tile.width.div_ceil(8),
                tile.height.div_ceil(8),
                1,
            );
        }

        // Copy results to staging buffers (only tile pixels)
        let tile_pixels = (tile.width * tile.height) as usize;
        let u32_byte_size = (tile_pixels * std::mem::size_of::<u32>()) as u64;
        let f32_byte_size = (tile_pixels * std::mem::size_of::<f32>()) as u64;

        encoder.copy_buffer_to_buffer(&buffers.results, 0, &buffers.staging_results, 0, u32_byte_size);
        encoder.copy_buffer_to_buffer(&buffers.glitch_flags, 0, &buffers.staging_glitches, 0, u32_byte_size);
        encoder.copy_buffer_to_buffer(&buffers.z_norm_sq, 0, &buffers.staging_z_norm_sq, 0, f32_byte_size);

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let iterations = self.read_buffer(&buffers.staging_results, tile_pixels).await?;
        let glitch_data = self.read_buffer(&buffers.staging_glitches, tile_pixels).await?;
        let z_norm_sq_data = self.read_buffer_f32(&buffers.staging_z_norm_sq, tile_pixels).await?;

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

        Ok(GpuTileResult {
            data,
            tile: *tile,
            compute_time_ms: end - start,
        })
    }

    async fn read_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<u32>, GpuError> {
        let byte_size = (count * std::mem::size_of::<u32>()) as u64;
        let slice = buffer.slice(..byte_size);

        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        #[cfg(target_arch = "wasm32")]
        self.context.device.poll(wgpu::Maintain::Poll);

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

    async fn read_buffer_f32(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<f32>, GpuError> {
        let byte_size = (count * std::mem::size_of::<f32>()) as u64;
        let slice = buffer.slice(..byte_size);

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

**Step 2: Update lib.rs exports**

Modify `fractalwonder-gpu/src/lib.rs`:

```rust
pub use perturbation_hdr_renderer::{GpuPerturbationHDRRenderer, GpuTileResult};
```

Remove `Adam7Pass` and `Adam7Accumulator` exports (no longer needed for GPU path).

**Step 3: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/perturbation_hdr_renderer.rs fractalwonder-gpu/src/lib.rs
git commit -m "refactor(gpu): implement tile-based rendering"
```

---

## Task 5: Remove Adam7 from GPU Crate

**Files:**
- Modify: `fractalwonder-gpu/src/lib.rs`
- Delete or modify: `fractalwonder-gpu/src/pass.rs`
- Delete or modify: `fractalwonder-gpu/src/stretch.rs`

**Step 1: Check if Adam7 is used elsewhere**

```bash
cd /Users/gert/Code/fractals/fractalwonder
grep -r "Adam7" --include="*.rs" | grep -v "^fractalwonder-gpu"
```

If Adam7 is used in UI for CPU path, keep the types but remove GPU dependency.

**Step 2: Update lib.rs**

Remove Adam7 exports from GPU crate. Keep `SENTINEL_NOT_COMPUTED` if still useful:

```rust
pub use stretch::SENTINEL_NOT_COMPUTED;
// Remove: pub use pass::Adam7Pass;
// Remove: pub use stretch::Adam7Accumulator;
```

**Step 3: Clean up stretch.rs**

Keep `SENTINEL_NOT_COMPUTED`, remove `Adam7Accumulator` if no longer needed.

**Step 4: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

Fix any remaining references.

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/
git commit -m "refactor(gpu): remove Adam7 progressive rendering"
```

---

## Task 6: Update Parallel Renderer for Tile Loop

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

This is the largest change. Replace the Adam7 pass loop with a tile loop.

**Step 1: Update imports**

```rust
use crate::rendering::tiles::{generate_tiles, GPU_TILE_SIZE};
use fractalwonder_gpu::{GpuPerturbationHDRRenderer, GpuTileResult, GPU_TILE_SIZE};
use fractalwonder_core::PixelRect;
```

**Step 2: Add full-image accumulator**

Add a field to `ParallelRenderer`:

```rust
/// Full-image ComputeData buffer for GPU tile accumulation
gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>>,
```

Initialize in `new()`:

```rust
let gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>> = Rc::new(RefCell::new(Vec::new()));
```

**Step 3: Replace start_gpu_render()**

Replace the Adam7 pass scheduling with tile-based scheduling:

```rust
fn start_gpu_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
    let width = canvas.width();
    let height = canvas.height();

    // Increment generation to invalidate any in-progress renders
    let gen = self.render_generation.get() + 1;
    self.render_generation.set(gen);

    // Generate tiles (64×64, center-out)
    let tiles = generate_tiles(width, height, GPU_TILE_SIZE);
    let total_tiles = tiles.len() as u32;

    // Initialize progress
    self.progress.set(RenderProgress::new(total_tiles));

    // Initialize full-image result buffer
    *self.gpu_result_buffer.borrow_mut() = vec![
        ComputeData::Mandelbrot(MandelbrotData::default());
        (width * height) as usize
    ];

    self.canvas_size.set((width, height));

    // Clone references for callback chain
    let generation = Rc::clone(&self.render_generation);
    let gpu_renderer = Rc::clone(&self.gpu_renderer);
    let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
    let gpu_in_use = Rc::clone(&self.gpu_in_use);
    let canvas_element = canvas.clone();
    let xray_enabled = Rc::clone(&self.xray_enabled);
    let gpu_result_buffer = Rc::clone(&self.gpu_result_buffer);
    let options = Rc::clone(&self.options);
    let palette = Rc::clone(&self.palette);
    let colorizer = Rc::clone(&self.colorizer);
    let progress = self.progress;
    let config = self.config;
    let viewport_clone = viewport.clone();
    let tile_results = Rc::clone(&self.tile_results);

    // Set up callback for when orbit is ready
    self.worker_pool.borrow().set_orbit_complete_callback(
        move |orbit_data: OrbitCompleteData| {
            log::info!(
                "Orbit ready: {} points, starting tiled GPU render ({} tiles)",
                orbit_data.orbit.len(),
                tiles.len()
            );

            let orbit_data = Rc::new(orbit_data);
            let tiles = Rc::new(tiles);

            // Start GPU init then first tile
            let generation = Rc::clone(&generation);
            let gpu_renderer = Rc::clone(&gpu_renderer);
            // ... clone all other Rc's ...

            wasm_bindgen_futures::spawn_local(async move {
                // GPU init (same as before)
                if !gpu_init_attempted.get() {
                    gpu_init_attempted.set(true);
                    match GpuContext::try_init().await {
                        GpuAvailability::Available(ctx) => {
                            log::info!("GPU renderer initialized");
                            *gpu_renderer.borrow_mut() = Some(GpuPerturbationHDRRenderer::new(ctx));
                        }
                        GpuAvailability::Unavailable(reason) => {
                            log::warn!("GPU unavailable: {reason}");
                            return;
                        }
                    }
                }

                // Schedule first tile
                let render_start_time = performance_now();
                schedule_tile(
                    0,
                    gen,
                    width,
                    height,
                    config,
                    generation,
                    gpu_renderer,
                    gpu_in_use,
                    canvas_element,
                    xray_enabled,
                    gpu_result_buffer,
                    tile_results,
                    progress,
                    viewport_clone,
                    tiles,
                    orbit_data,
                    render_start_time,
                    options,
                    palette,
                    colorizer,
                );
            });
        },
    );

    // Compute orbit (triggers callback when ready)
    self.worker_pool
        .borrow_mut()
        .compute_orbit_for_gpu(viewport.clone(), (width, height));
}
```

**Step 4: Create schedule_tile() function**

```rust
#[allow(clippy::too_many_arguments)]
fn schedule_tile(
    tile_index: usize,
    expected_gen: u32,
    width: u32,
    height: u32,
    config: &'static FractalConfig,
    generation: Rc<Cell<u32>>,
    gpu_renderer: Rc<RefCell<Option<GpuPerturbationHDRRenderer>>>,
    gpu_in_use: Rc<Cell<bool>>,
    canvas_element: HtmlCanvasElement,
    xray_enabled: Rc<Cell<bool>>,
    gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>>,
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    progress: RwSignal<RenderProgress>,
    viewport: Viewport,
    tiles: Rc<Vec<PixelRect>>,
    orbit_data: Rc<OrbitCompleteData>,
    render_start_time: f64,
    options: Rc<RefCell<ColorOptions>>,
    palette: Rc<RefCell<Palette>>,
    colorizer: Rc<RefCell<ColorizerKind>>,
) {
    // Check generation - abort if stale
    if generation.get() != expected_gen {
        log::debug!("Render interrupted at tile {}", tile_index);
        return;
    }

    let tile = tiles[tile_index];
    let is_final = tile_index == tiles.len() - 1;

    // Clone for spawn_local
    let generation_spawn = Rc::clone(&generation);
    let gpu_renderer_spawn = Rc::clone(&gpu_renderer);
    // ... clone all ...

    wasm_bindgen_futures::spawn_local(async move {
        // Convert viewport to HDRFloat format
        let vp_width = HDRFloat::from_bigfloat(&viewport.width);
        let vp_height = HDRFloat::from_bigfloat(&viewport.height);

        let half = HDRFloat::from_f64(0.5);
        let half_width = vp_width.mul(&half);
        let half_height = vp_height.mul(&half);
        let origin_re = half_width.neg();
        let origin_im = half_height.neg();

        let step_re = HDRFloat::from_f64(vp_width.to_f64() / width as f64);
        let step_im = HDRFloat::from_f64(vp_height.to_f64() / height as f64);

        let dc_origin = (
            (origin_re.head, origin_re.tail, origin_re.exp),
            (origin_im.head, origin_im.tail, origin_im.exp),
        );
        let dc_step = (
            (step_re.head, step_re.tail, step_re.exp),
            (step_im.head, step_im.tail, step_im.exp),
        );

        let tau_sq = config.tau_sq as f32;
        let reference_escaped = orbit_data.orbit.len() < orbit_data.max_iterations as usize;

        // Mark GPU in use, take renderer
        gpu_in_use_spawn.set(true);
        let mut renderer = gpu_renderer_spawn.borrow_mut().take().unwrap();

        let tile_result = renderer
            .render_tile(
                &orbit_data.orbit,
                orbit_data.orbit_id,
                dc_origin,
                dc_step,
                width,
                height,
                &tile,
                orbit_data.max_iterations,
                tau_sq,
                reference_escaped,
            )
            .await;

        // Return renderer
        *gpu_renderer_spawn.borrow_mut() = Some(renderer);
        gpu_in_use_spawn.set(false);

        // Check generation after await
        if generation_spawn.get() != expected_gen {
            return;
        }

        match tile_result {
            Ok(result) => {
                log::debug!(
                    "Tile {}/{}: ({},{}) {}×{} in {:.1}ms",
                    tile_index + 1,
                    tiles.len(),
                    tile.x,
                    tile.y,
                    tile.width,
                    tile.height,
                    result.compute_time_ms
                );

                // Copy tile data into full-image buffer
                {
                    let mut buffer = gpu_result_buffer_spawn.borrow_mut();
                    for ty in 0..tile.height {
                        for tx in 0..tile.width {
                            let tile_idx = (ty * tile.width + tx) as usize;
                            let image_idx = ((tile.y + ty) * width + (tile.x + tx)) as usize;
                            if tile_idx < result.data.len() && image_idx < buffer.len() {
                                buffer[image_idx] = result.data[tile_idx].clone();
                            }
                        }
                    }
                }

                // Quick colorize tile and draw
                let xray = xray_enabled_spawn.get();
                let opts = options_spawn.borrow();
                let pal = palette_spawn.borrow();
                let col = colorizer_spawn.borrow();

                let tile_pixels: Vec<u8> = result
                    .data
                    .iter()
                    .flat_map(|d| colorize_with_palette(d, &opts, &pal, &col, xray))
                    .collect();

                if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                    let _ = draw_pixels_to_canvas(
                        &ctx,
                        &tile_pixels,
                        tile.width,
                        tile.x as f64,
                        tile.y as f64,
                    );
                }

                // Update progress
                let elapsed_ms = performance_now() - render_start_time;
                progress.update(|p| {
                    p.completed_steps += 1;
                    p.elapsed_ms = elapsed_ms;
                    p.is_complete = is_final;
                });

                if is_final {
                    // Run full colorizer pipeline on complete buffer
                    let full_buffer = gpu_result_buffer_spawn.borrow();
                    let reference_width = config.default_viewport(viewport.precision_bits()).width;
                    let zoom_level = reference_width.to_f64() / viewport.width.to_f64();

                    let final_pixels = col.run_pipeline(
                        &full_buffer,
                        &opts,
                        &pal,
                        width as usize,
                        height as usize,
                        zoom_level,
                    );

                    // Store for recolorize
                    tile_results_spawn.borrow_mut().clear();
                    tile_results_spawn.borrow_mut().push(TileResult {
                        tile: PixelRect::new(0, 0, width, height),
                        data: full_buffer.clone(),
                        compute_time_ms: elapsed_ms,
                    });

                    // Draw final image
                    if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                        let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
                        let _ = draw_full_frame(&ctx, &pixel_bytes, width, height);
                    }

                    log::info!("Tiled render complete: {} tiles in {:.1}ms", tiles.len(), elapsed_ms);
                } else {
                    // Schedule next tile via requestAnimationFrame
                    let next_index = tile_index + 1;
                    request_animation_frame_then(move || {
                        schedule_tile(
                            next_index,
                            expected_gen,
                            width,
                            height,
                            config,
                            generation_spawn,
                            gpu_renderer_spawn,
                            gpu_in_use_spawn,
                            canvas_element_spawn,
                            xray_enabled_spawn,
                            gpu_result_buffer_spawn,
                            tile_results_spawn,
                            progress,
                            viewport,
                            tiles,
                            orbit_data,
                            render_start_time,
                            options_spawn,
                            palette_spawn,
                            colorizer_spawn,
                        );
                    });
                }
            }
            Err(e) => {
                log::error!("GPU tile {} failed: {e}", tile_index);
                // Could fall back to CPU here
            }
        }
    });
}
```

**Step 5: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): implement tile-based GPU progressive rendering"
```

---

## Task 7: Update Tile Generation for GPU

**Files:**
- Modify: `fractalwonder-ui/src/rendering/tiles.rs`

**Step 1: Add GPU tile size constant**

```rust
/// Fixed tile size for GPU rendering (64×64).
/// Chosen to keep dispatch work bounded and avoid GPU timeout.
pub const GPU_TILE_SIZE: u32 = 64;
```

**Step 2: Export from mod.rs**

In `fractalwonder-ui/src/rendering/mod.rs`:

```rust
pub use tiles::{calculate_tile_size, generate_tiles, tile_to_viewport, GPU_TILE_SIZE};
```

**Step 3: Run tests**

```bash
cargo test --workspace -- tiles
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/tiles.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(ui): add GPU_TILE_SIZE constant"
```

---

## Task 8: Run Full Test Suite

**Step 1: Format code**

```bash
cargo fmt --all
```

**Step 2: Run Clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Fix any warnings.

**Step 3: Run cargo check**

```bash
cargo check --workspace --all-targets --all-features
```

**Step 4: Run tests**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

**Step 5: Commit fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings and test failures"
```

---

## Task 9: Manual Browser Testing

**Step 1: Start dev server**

Ensure `trunk serve` is running on http://localhost:8080.

**Step 2: Test shallow zoom**

1. Load the app
2. Verify tiles render center-out
3. Check console for tile timing logs

**Step 3: Test deep zoom**

1. Zoom to ~10^15 (where timeouts occurred before)
2. Verify tiles complete without GPU timeout
3. Check that final image matches expected (no missing tiles)

**Step 4: Test interruption**

1. Start a render
2. Pan/zoom before it completes
3. Verify old render is cancelled, new one starts

**Step 5: Document results**

Note any issues found for follow-up.

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Update uniforms for tiles | buffers.rs |
| 2 | Update shader for tiles | delta_iteration_hdr.wgsl |
| 3 | Fixed tile-sized buffers | buffers.rs, lib.rs |
| 4 | Tile-based renderer | perturbation_hdr_renderer.rs |
| 5 | Remove Adam7 from GPU | lib.rs, pass.rs, stretch.rs |
| 6 | Tile loop in parallel_renderer | parallel_renderer.rs |
| 7 | GPU tile size constant | tiles.rs, mod.rs |
| 8 | Full test suite | - |
| 9 | Manual browser testing | - |
