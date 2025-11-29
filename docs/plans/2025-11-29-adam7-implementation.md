# Adam7 Progressive Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace resolution-based passes with Adam7 interlacing for meaningful visual progress during GPU rendering.

**Architecture:** GPU shader early-exits for non-matching pixels based on Adam7 step. CPU accumulates results across 7 passes, filling gaps from neighbors for display. ComputeData cache preserved for instant recolorization.

**Tech Stack:** Rust, wgpu, WGSL shaders, Leptos

**Sentinel Value:** `0xFFFFFFFF` (u32::MAX) marks uncomputed pixels.

---

## Task 1: Replace Pass with Adam7Pass

**Files:**
- Modify: `fractalwonder-gpu/src/pass.rs` (replace entire file)

**Step 1: Delete old Pass enum and write Adam7Pass**

Replace the entire contents of `pass.rs` with:

```rust
// fractalwonder-gpu/src/pass.rs

/// Adam7 progressive rendering pass (1-7).
///
/// Replaces the old resolution-based Pass system. Each pass computes a subset
/// of pixels at full resolution, with each pass doubling the pixel count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adam7Pass(u8);

impl Adam7Pass {
    /// Create a new Adam7Pass. Panics if step is not 1-7.
    pub fn new(step: u8) -> Self {
        assert!((1..=7).contains(&step), "Adam7 step must be 1-7, got {step}");
        Self(step)
    }

    /// Returns all 7 passes in order.
    pub fn all() -> [Adam7Pass; 7] {
        [1, 2, 3, 4, 5, 6, 7].map(Adam7Pass)
    }

    /// Returns the step number (1-7).
    pub fn step(&self) -> u8 {
        self.0
    }

    /// Returns true if this is the final pass (step 7).
    pub fn is_final(&self) -> bool {
        self.0 == 7
    }

    /// Cumulative pixel percentage after this pass completes.
    pub fn cumulative_percent(&self) -> f32 {
        match self.0 {
            1 => 1.5625,
            2 => 3.125,
            3 => 6.25,
            4 => 12.5,
            5 => 25.0,
            6 => 50.0,
            7 => 100.0,
            _ => 0.0,
        }
    }

    /// Pixels computed in this pass as a fraction (for progress display).
    pub fn pass_fraction(&self) -> f32 {
        match self.0 {
            1 => 1.0 / 64.0,
            2 => 1.0 / 64.0,
            3 => 2.0 / 64.0,
            4 => 4.0 / 64.0,
            5 => 8.0 / 64.0,
            6 => 16.0 / 64.0,
            7 => 32.0 / 64.0,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_passes() {
        let passes = Adam7Pass::all();
        assert_eq!(passes.len(), 7);
        assert_eq!(passes[0].step(), 1);
        assert_eq!(passes[6].step(), 7);
    }

    #[test]
    fn test_is_final() {
        assert!(!Adam7Pass::new(1).is_final());
        assert!(!Adam7Pass::new(6).is_final());
        assert!(Adam7Pass::new(7).is_final());
    }

    #[test]
    fn test_cumulative_percent() {
        assert!((Adam7Pass::new(1).cumulative_percent() - 1.5625).abs() < 0.001);
        assert!((Adam7Pass::new(7).cumulative_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_fractions_sum_to_one() {
        let total: f32 = Adam7Pass::all().iter().map(|p| p.pass_fraction()).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    #[should_panic(expected = "Adam7 step must be 1-7")]
    fn test_invalid_step_zero() {
        Adam7Pass::new(0);
    }

    #[test]
    #[should_panic(expected = "Adam7 step must be 1-7")]
    fn test_invalid_step_eight() {
        Adam7Pass::new(8);
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p fractalwonder-gpu pass::tests --all-features -- --nocapture
```

Expected: All 6 tests pass.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/pass.rs
git commit -m "refactor(gpu): replace Pass enum with Adam7Pass"
```

---

## Task 2: Add adam7_step to Uniforms

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Update Uniforms struct**

In `buffers.rs`, modify the `Uniforms` struct. Change:

```rust
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
```

To:

```rust
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
    pub adam7_step: u32,  // 0 = compute all, 1-7 = Adam7 pass
    pub _padding: [u32; 2],
}
```

**Step 2: Update Uniforms::new()**

Add `adam7_step` parameter. Change:

```rust
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
```

To:

```rust
impl Uniforms {
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: (f32, f32),
        dc_step: (f32, f32),
        adam7_step: u32,
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
            adam7_step,
            _padding: [0; 2],
        }
    }
}
```

**Step 3: Verify struct size is unchanged (48 bytes)**

The struct should remain 48 bytes (12 u32s). The padding went from 3 to 2 elements because we added adam7_step.

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles (may have errors in renderer.rs due to changed signature - that's expected).

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add adam7_step to Uniforms"
```

---

## Task 3: Update WGSL Shader with Adam7 Logic

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/delta_iteration.wgsl`

**Step 1: Update shader**

Replace the entire contents of `delta_iteration.wgsl` with:

```wgsl
// Delta iteration compute shader for f32 perturbation rendering.
// Supports Adam7 progressive rendering via adam7_step uniform.

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
    adam7_step: u32,  // 0 = compute all, 1-7 = Adam7 pass
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;

// Adam7 interlacing matrix (8x8 pattern, values 1-7)
fn get_adam7_pass(x: u32, y: u32) -> u32 {
    // Row-major 8x8 matrix indexed by [y % 8][x % 8]
    let row = y % 8u;
    let col = x % 8u;

    // Pattern encodes which pass (1-7) each pixel belongs to
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

// Sentinel value for uncomputed pixels
const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Adam7 early exit: skip pixels not in current pass
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        // Write sentinel to indicate "not computed this pass"
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

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
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            return;
        }

        // Pauldelbrot glitch detection: |z|^2 < tau^2 * |Z|^2
        if Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq {
            glitched = true;
        }

        // Rebase check: |z|^2 < |dz|^2
        if z_sq < dz_sq {
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
        if m >= orbit_len {
            m = 0u;
        }
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
}
```

**Step 2: Verify shader compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles (renderer.rs may have errors - expected).

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/delta_iteration.wgsl
git commit -m "feat(gpu): add Adam7 interlacing to compute shader"
```

---

## Task 4: Replace stretch.rs with Adam7 Accumulator

**Files:**
- Modify: `fractalwonder-gpu/src/stretch.rs` (replace entire file)

**Step 1: Replace stretch.rs**

Replace the entire contents with:

```rust
// fractalwonder-gpu/src/stretch.rs

use fractalwonder_core::{ComputeData, MandelbrotData};

/// Sentinel value indicating a pixel was not computed in the current Adam7 pass.
pub const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFF;

/// Accumulator for Adam7 progressive rendering.
///
/// Collects results across multiple Adam7 passes, merging each pass's computed
/// pixels into a full-resolution buffer.
pub struct Adam7Accumulator {
    data: Vec<Option<ComputeData>>,
    width: u32,
    height: u32,
}

impl Adam7Accumulator {
    /// Create a new accumulator for the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![None; (width * height) as usize],
            width,
            height,
        }
    }

    /// Merge GPU results into accumulator.
    ///
    /// Only updates pixels where GPU returned valid data (not sentinel).
    pub fn merge(&mut self, gpu_result: &[ComputeData]) {
        for (i, computed) in gpu_result.iter().enumerate() {
            if let ComputeData::Mandelbrot(m) = computed {
                if m.iterations != SENTINEL_NOT_COMPUTED {
                    self.data[i] = Some(computed.clone());
                }
            }
        }
    }

    /// Export to Vec<ComputeData> for colorization.
    ///
    /// Uncomputed pixels (None) are filled from left neighbor, or top neighbor
    /// if at left edge. First pixel defaults to black if uncomputed.
    pub fn to_display_buffer(&self) -> Vec<ComputeData> {
        let mut result = Vec::with_capacity(self.data.len());
        let width = self.width as usize;

        for (i, pixel) in self.data.iter().enumerate() {
            match pixel {
                Some(d) => result.push(d.clone()),
                None => {
                    // Try left neighbor first, then top neighbor
                    let fallback = if i % width > 0 {
                        result.get(i - 1).cloned()
                    } else if i >= width {
                        result.get(i - width).cloned()
                    } else {
                        None
                    };

                    result.push(fallback.unwrap_or_else(Self::black_pixel));
                }
            }
        }

        result
    }

    /// Export final complete buffer for caching.
    ///
    /// After pass 7, all pixels should be computed. Panics if any are missing.
    pub fn to_final_buffer(&self) -> Vec<ComputeData> {
        self.data
            .iter()
            .map(|opt| opt.clone().expect("All pixels should be computed after pass 7"))
            .collect()
    }

    /// Check if all pixels have been computed.
    pub fn is_complete(&self) -> bool {
        self.data.iter().all(|opt| opt.is_some())
    }

    /// Default black pixel for uncomputed areas.
    fn black_pixel() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 0,
            max_iterations: 1,
            escaped: false,
            glitched: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 1000,
            escaped: iterations < 1000,
            glitched: false,
        })
    }

    fn make_sentinel() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: SENTINEL_NOT_COMPUTED,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
        })
    }

    fn get_iterations(data: &ComputeData) -> u32 {
        match data {
            ComputeData::Mandelbrot(m) => m.iterations,
            _ => panic!("Expected Mandelbrot"),
        }
    }

    #[test]
    fn test_new_accumulator() {
        let acc = Adam7Accumulator::new(10, 10);
        assert_eq!(acc.data.len(), 100);
        assert!(acc.data.iter().all(|x| x.is_none()));
    }

    #[test]
    fn test_merge_skips_sentinel() {
        let mut acc = Adam7Accumulator::new(2, 2);

        // GPU returns: computed, sentinel, sentinel, computed
        let gpu_result = vec![
            make_data(100),
            make_sentinel(),
            make_sentinel(),
            make_data(200),
        ];

        acc.merge(&gpu_result);

        assert!(acc.data[0].is_some());
        assert!(acc.data[1].is_none());
        assert!(acc.data[2].is_none());
        assert!(acc.data[3].is_some());
    }

    #[test]
    fn test_to_display_buffer_fills_gaps() {
        let mut acc = Adam7Accumulator::new(4, 1);

        // Only first and last computed
        acc.data[0] = Some(make_data(100));
        acc.data[3] = Some(make_data(200));

        let display = acc.to_display_buffer();

        // Gaps filled from left neighbor
        assert_eq!(get_iterations(&display[0]), 100);
        assert_eq!(get_iterations(&display[1]), 100); // from left
        assert_eq!(get_iterations(&display[2]), 100); // from left
        assert_eq!(get_iterations(&display[3]), 200);
    }

    #[test]
    fn test_to_display_buffer_uses_top_at_edge() {
        let mut acc = Adam7Accumulator::new(2, 2);

        // Row 0: [100, 200]
        // Row 1: [None, 300]
        acc.data[0] = Some(make_data(100));
        acc.data[1] = Some(make_data(200));
        acc.data[3] = Some(make_data(300));

        let display = acc.to_display_buffer();

        // acc.data[2] (row 1, col 0) should copy from top (acc.data[0])
        assert_eq!(get_iterations(&display[2]), 100);
    }

    #[test]
    fn test_is_complete() {
        let mut acc = Adam7Accumulator::new(2, 2);
        assert!(!acc.is_complete());

        acc.data[0] = Some(make_data(1));
        acc.data[1] = Some(make_data(2));
        acc.data[2] = Some(make_data(3));
        assert!(!acc.is_complete());

        acc.data[3] = Some(make_data(4));
        assert!(acc.is_complete());
    }

    #[test]
    fn test_to_final_buffer() {
        let mut acc = Adam7Accumulator::new(2, 2);
        acc.data[0] = Some(make_data(1));
        acc.data[1] = Some(make_data(2));
        acc.data[2] = Some(make_data(3));
        acc.data[3] = Some(make_data(4));

        let final_buf = acc.to_final_buffer();
        assert_eq!(final_buf.len(), 4);
        assert_eq!(get_iterations(&final_buf[0]), 1);
        assert_eq!(get_iterations(&final_buf[3]), 4);
    }

    #[test]
    #[should_panic(expected = "All pixels should be computed")]
    fn test_to_final_buffer_panics_if_incomplete() {
        let acc = Adam7Accumulator::new(2, 2);
        acc.to_final_buffer();
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p fractalwonder-gpu stretch::tests --all-features -- --nocapture
```

Expected: All 7 tests pass.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/stretch.rs
git commit -m "refactor(gpu): replace stretch with Adam7Accumulator"
```

---

## Task 5: Update GpuRenderer

**Files:**
- Modify: `fractalwonder-gpu/src/renderer.rs`

**Step 1: Simplify render method to accept Adam7Pass**

Replace the entire file with:

```rust
//! High-level GPU renderer API.

use crate::buffers::{GpuBuffers, Uniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::pipeline::GpuPipeline;
use crate::stretch::SENTINEL_NOT_COMPUTED;
use crate::Adam7Pass;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a GPU render operation.
pub struct GpuRenderResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

impl GpuRenderResult {
    pub fn has_glitches(&self) -> bool {
        self.data.iter().any(|d| match d {
            ComputeData::Mandelbrot(m) => m.glitched && m.iterations != SENTINEL_NOT_COMPUTED,
            _ => false,
        })
    }

    pub fn glitched_pixel_count(&self) -> usize {
        self.data
            .iter()
            .filter(|d| match d {
                ComputeData::Mandelbrot(m) => m.glitched && m.iterations != SENTINEL_NOT_COMPUTED,
                _ => false,
            })
            .count()
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

    /// Render a single Adam7 pass.
    ///
    /// Returns ComputeData for all pixels, but only pixels matching the Adam7
    /// pass will have valid data. Non-matching pixels have iterations = SENTINEL_NOT_COMPUTED.
    #[allow(clippy::too_many_arguments)]
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
        pass: Adam7Pass,
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

        // Write uniforms with Adam7 step
        let uniforms = Uniforms::new(
            width,
            height,
            max_iterations,
            tau_sq,
            dc_origin,
            dc_step,
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
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("delta_iteration_encoder"),
                });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("delta_iteration_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
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
        let iterations = self
            .read_buffer(&buffers.staging_results, pixel_count)
            .await?;
        let glitch_data = self
            .read_buffer(&buffers.staging_glitches, pixel_count)
            .await?;

        // Convert to ComputeData
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(glitch_data.iter())
            .map(|(&iter, &glitch_flag)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations && iter != SENTINEL_NOT_COMPUTED,
                    glitched: glitch_flag != 0,
                })
            })
            .collect();

        let end = Self::now();

        Ok(GpuRenderResult {
            data,
            compute_time_ms: end - start,
        })
    }

    async fn read_buffer(
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

**Step 2: Verify it compiles**

```bash
cargo check -p fractalwonder-gpu
```

Expected: Compiles successfully.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/renderer.rs
git commit -m "refactor(gpu): simplify GpuRenderer for Adam7 passes"
```

---

## Task 6: Update lib.rs Exports

**Files:**
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Update exports**

Replace:

```rust
pub use pass::Pass;
pub use stretch::stretch_compute_data;
```

With:

```rust
pub use pass::Adam7Pass;
pub use stretch::{Adam7Accumulator, SENTINEL_NOT_COMPUTED};
```

Full file should be:

```rust
//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pass;
mod pipeline;
mod renderer;
mod stretch;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pass::Adam7Pass;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
pub use stretch::{Adam7Accumulator, SENTINEL_NOT_COMPUTED};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 2: Run all GPU crate tests**

```bash
cargo test -p fractalwonder-gpu --all-features -- --nocapture
```

Expected: All tests pass.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/lib.rs
git commit -m "refactor(gpu): update exports for Adam7"
```

---

## Task 7: Update parallel_renderer.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

This is the largest change. The file needs significant updates to:
1. Import `Adam7Pass` and `Adam7Accumulator`
2. Replace the pass scheduling logic
3. Use the accumulator for progressive display

**Step 1: Update imports**

At the top of the file, change:

```rust
use fractalwonder_gpu::{stretch_compute_data, GpuAvailability, GpuContext, GpuRenderer, Pass};
```

To:

```rust
use fractalwonder_gpu::{Adam7Accumulator, Adam7Pass, GpuAvailability, GpuContext, GpuRenderer};
```

**Step 2: Add accumulator to ParallelRenderer struct**

Add a new field after `canvas_size`:

```rust
/// Adam7 accumulator for progressive rendering
adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>>,
```

**Step 3: Initialize accumulator in new()**

After `let render_generation: Rc<Cell<u32>> = Rc::new(Cell::new(0));`, add:

```rust
let adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>> = Rc::new(RefCell::new(None));
```

And add to the `Self` construction:

```rust
adam7_accumulator,
```

**Step 4: Update progress initialization in start_gpu_render()**

Change:

```rust
let total_passes = Pass::all().len() as u32;
self.progress.set(RenderProgress::new(total_passes));
```

To:

```rust
let total_passes = Adam7Pass::all().len() as u32;
self.progress.set(RenderProgress::new(total_passes));

// Initialize accumulator for this render
*self.adam7_accumulator.borrow_mut() = Some(Adam7Accumulator::new(width, height));
```

**Step 5: Clone accumulator for callback**

After `let tiles = generate_tiles(width, height, calculate_tile_size(1.0));`, add:

```rust
let adam7_accumulator = Rc::clone(&self.adam7_accumulator);
```

And clone it again inside the orbit complete callback setup (similar to other clones).

**Step 6: Replace schedule_gpu_pass function**

This function needs to be completely rewritten. Replace the entire `schedule_gpu_pass` function with:

```rust
/// Schedule an Adam7 pass with proper browser repaint between passes.
#[allow(clippy::too_many_arguments)]
fn schedule_adam7_pass(
    pass: Adam7Pass,
    pass_index: usize,
    expected_gen: u32,
    width: u32,
    height: u32,
    config: &'static FractalConfig,
    generation: Rc<Cell<u32>>,
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,
    canvas_element: HtmlCanvasElement,
    xray_enabled: Rc<Cell<bool>>,
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    viewport: Viewport,
    tiles: Vec<PixelRect>,
    orbit_data: Rc<OrbitCompleteData>,
    render_start_time: f64,
    gpu_in_use: Rc<Cell<bool>>,
    adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>>,
) {
    log::info!("Scheduling Adam7 pass {}", pass.step());

    // Check generation - abort if stale
    if generation.get() != expected_gen {
        log::debug!("Render interrupted at Adam7 pass {}", pass.step());
        return;
    }

    // Clone for spawn_local
    let generation_spawn = Rc::clone(&generation);
    let gpu_renderer_spawn = Rc::clone(&gpu_renderer);
    let gpu_in_use_spawn = Rc::clone(&gpu_in_use);
    let canvas_element_spawn = canvas_element.clone();
    let xray_enabled_spawn = Rc::clone(&xray_enabled);
    let tile_results_spawn = Rc::clone(&tile_results);
    let worker_pool_spawn = Rc::clone(&worker_pool);
    let viewport_spawn = viewport.clone();
    let tiles_spawn = tiles.clone();
    let orbit_data_spawn = Rc::clone(&orbit_data);
    let adam7_accumulator_spawn = Rc::clone(&adam7_accumulator);

    wasm_bindgen_futures::spawn_local(async move {
        let vp_width = viewport_spawn.width.to_f64() as f32;
        let vp_height = viewport_spawn.height.to_f64() as f32;
        let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
        let dc_step = (vp_width / width as f32, vp_height / height as f32);
        let tau_sq = config.tau_sq as f32;

        // Mark GPU as in use
        gpu_in_use_spawn.set(true);

        // Take renderer temporarily
        let mut renderer = gpu_renderer_spawn.borrow_mut().take().unwrap();
        let pass_result = renderer
            .render(
                &orbit_data_spawn.orbit,
                orbit_data_spawn.orbit_id,
                dc_origin,
                dc_step,
                width,
                height,
                orbit_data_spawn.max_iterations,
                tau_sq,
                pass,
            )
            .await;

        // Put renderer back
        *gpu_renderer_spawn.borrow_mut() = Some(renderer);
        gpu_in_use_spawn.set(false);

        match pass_result {
            Ok(result) => {
                log::info!(
                    "Adam7 pass {}: {:.1}ms",
                    pass.step(),
                    result.compute_time_ms
                );

                // Merge into accumulator
                if let Some(ref mut acc) = *adam7_accumulator_spawn.borrow_mut() {
                    acc.merge(&result.data);

                    // Get display buffer (with gaps filled)
                    let display_data = if pass.is_final() {
                        acc.to_final_buffer()
                    } else {
                        acc.to_display_buffer()
                    };

                    // Store for recolorize (update with latest)
                    tile_results_spawn.borrow_mut().clear();
                    tile_results_spawn.borrow_mut().push(TileResult {
                        tile: PixelRect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        data: display_data.clone(),
                        compute_time_ms: result.compute_time_ms,
                    });

                    // Colorize and draw
                    let xray = xray_enabled_spawn.get();
                    let pixels: Vec<u8> = display_data.iter().flat_map(|d| colorize(d, xray)).collect();

                    if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                        match draw_full_frame(&ctx, &pixels, width, height) {
                            Ok(()) => log::info!(
                                "Drew Adam7 pass {} to canvas",
                                pass.step()
                            ),
                            Err(e) => log::error!("Draw failed for Adam7 pass {}: {:?}", pass.step(), e),
                        }
                    }
                }

                // Update progress
                let elapsed_ms = performance_now() - render_start_time;
                progress.update(|p| {
                    p.completed_steps += 1;
                    p.elapsed_ms = elapsed_ms;
                    p.is_complete = pass.is_final();
                });

                if !pass.is_final() {
                    // Schedule next pass via double rAF
                    let passes = Adam7Pass::all();
                    let next_index = pass_index + 1;
                    if next_index < passes.len() {
                        request_animation_frame_then(move || {
                            request_animation_frame_then(move || {
                                schedule_adam7_pass(
                                    passes[next_index],
                                    next_index,
                                    expected_gen,
                                    width,
                                    height,
                                    config,
                                    generation_spawn,
                                    gpu_renderer_spawn,
                                    canvas_element_spawn,
                                    xray_enabled_spawn,
                                    tile_results_spawn,
                                    worker_pool_spawn,
                                    progress,
                                    viewport_spawn,
                                    tiles_spawn,
                                    orbit_data_spawn,
                                    render_start_time,
                                    gpu_in_use_spawn,
                                    adam7_accumulator_spawn,
                                );
                            });
                        });
                    }
                }
            }
            Err(e) => {
                log::warn!("GPU Adam7 pass {} failed: {e}, falling back to CPU", pass.step());
                worker_pool_spawn.borrow_mut().start_perturbation_render(
                    viewport_spawn,
                    (width, height),
                    tiles_spawn,
                );
            }
        }
    });
}
```

**Step 7: Update call sites to use schedule_adam7_pass**

In the orbit complete callback, change all calls from `schedule_gpu_pass` to `schedule_adam7_pass`, and:

1. Change `Pass::all()[0]` to `Adam7Pass::all()[0]`
2. Add `adam7_accumulator` parameter to all calls

**Step 8: Verify it compiles**

```bash
cargo check -p fractalwonder-ui
```

Expected: Compiles (may have warnings about unused code).

**Step 9: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(ui): implement Adam7 progressive rendering"
```

---

## Task 8: Full Build and Test

**Step 1: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: No errors or warnings.

**Step 2: Run all tests**

```bash
cargo test --workspace --all-features -- --nocapture
```

Expected: All tests pass.

**Step 3: Build for WASM**

```bash
trunk build
```

Expected: Build succeeds.

**Step 4: Manual test in browser**

1. Run `trunk serve`
2. Open http://localhost:8080
3. Navigate to a deep zoom location
4. Observe 7 progressive passes rendering instead of 4
5. Verify visual feedback is more useful than before

**Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address review feedback for Adam7 rendering"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Replace Pass with Adam7Pass | `pass.rs` |
| 2 | Add adam7_step to Uniforms | `buffers.rs` |
| 3 | Update shader with Adam7 logic | `delta_iteration.wgsl` |
| 4 | Replace stretch with Adam7Accumulator | `stretch.rs` |
| 5 | Update GpuRenderer | `renderer.rs` |
| 6 | Update lib.rs exports | `lib.rs` |
| 7 | Update parallel_renderer | `parallel_renderer.rs` |
| 8 | Full build and test | - |

Total: 8 tasks, each with incremental commits.
