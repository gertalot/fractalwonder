//! GPU renderer for perturbation with HDRFloat deltas.
//! Uses extended-range arithmetic to avoid precision loss at moderate zoom.

use crate::buffers::{PerturbationHDRBuffers, PerturbationHDRUniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::perturbation_hdr_pipeline::PerturbationHDRPipeline;
use crate::stretch::SENTINEL_NOT_COMPUTED;
use crate::Adam7Pass;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a GPU perturbation HDR render operation.
pub struct GpuPerturbationHDRResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

impl GpuPerturbationHDRResult {
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

/// GPU renderer for Mandelbrot perturbation with HDRFloat deltas.
/// Provides extended range arithmetic to fix artifacts at moderate zoom (10^4 to 10^15).
pub struct GpuPerturbationHDRRenderer {
    context: GpuContext,
    pipeline: PerturbationHDRPipeline,
    buffers: Option<PerturbationHDRBuffers>,
    cached_orbit_id: Option<u32>,
    current_dimensions: Option<(u32, u32)>,
}

impl GpuPerturbationHDRRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = PerturbationHDRPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            cached_orbit_id: None,
            current_dimensions: None,
        }
    }

    /// Render a single Adam7 pass using HDRFloat perturbation.
    ///
    /// # Arguments
    /// * `orbit` - Reference orbit as (re, im) pairs
    /// * `orbit_id` - ID for orbit caching
    /// * `dc_origin` - Top-left δc as ((re_head, re_tail, re_exp), (im_head, im_tail, im_exp))
    /// * `dc_step` - Per-pixel δc step as ((re_head, re_tail, re_exp), (im_head, im_tail, im_exp))
    #[allow(clippy::too_many_arguments)]
    pub async fn render(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        reference_escaped: bool,
        pass: Adam7Pass,
    ) -> Result<GpuPerturbationHDRResult, GpuError> {
        let start = Self::now();

        // Recreate buffers if dimensions changed
        if self.current_dimensions != Some((width, height))
            || self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0) < orbit.len() as u32
        {
            self.buffers = Some(PerturbationHDRBuffers::new(
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

        // Write uniforms with HDRFloat dc values
        let uniforms = PerturbationHDRUniforms::new(
            width,
            height,
            max_iterations,
            tau_sq,
            dc_origin,
            dc_step,
            pass.step() as u32,
            reference_escaped,
            orbit.len() as u32,
        );

        log::debug!(
            "GPU uniforms: width={}, height={}, max_iter={}, adam7_step={}, orbit_len={}",
            width,
            height,
            max_iterations,
            pass.step(),
            orbit.len()
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

        // Dispatch compute shader
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
            .read_buffer(&buffers.staging_results, pixel_count)
            .await?;
        let glitch_data = self
            .read_buffer(&buffers.staging_glitches, pixel_count)
            .await?;
        let z_norm_sq_data = self
            .read_buffer_f32(&buffers.staging_z_norm_sq, pixel_count)
            .await?;

        // Diagnostic: Check for signs of GPU timeout/incomplete execution
        // MARKER_THREAD_STARTED (0xDEADBEEF) = shader started but loop never finished
        // Zero = shader thread never started (buffer untouched)
        const MARKER_THREAD_STARTED: u32 = 0xDEADBEEF;

        let sentinel_count = iterations.iter().filter(|&&v| v == SENTINEL_NOT_COMPUTED).count();
        let zero_count = iterations.iter().filter(|&&v| v == 0).count();
        let max_iter_count = iterations.iter().filter(|&&v| v == max_iterations).count();
        let thread_started_count = iterations.iter().filter(|&&v| v == MARKER_THREAD_STARTED).count();
        let valid_escape_count = iterations
            .iter()
            .filter(|&&v| v > 0 && v < max_iterations && v != SENTINEL_NOT_COMPUTED && v != MARKER_THREAD_STARTED)
            .count();

        log::info!(
            "GPU pass {} raw results: {} sentinel, {} zero, {} max_iter, {} valid_escape, {} STUCK (0xDEADBEEF), {} total",
            pass.step(),
            sentinel_count,
            zero_count,
            max_iter_count,
            valid_escape_count,
            thread_started_count,
            pixel_count
        );

        if thread_started_count > 0 {
            log::error!(
                "GPU TIMEOUT DETECTED: {} pixels ({:.1}%) started but never finished! \
                 Iteration loop is stuck (too many iterations or infinite rebase loop).",
                thread_started_count,
                (thread_started_count as f64 / pixel_count as f64) * 100.0
            );
        }

        if zero_count > 0 && pass.step() > 0 {
            // At pass > 0, zeros mean the shader thread never touched the buffer
            log::warn!(
                "GPU WARNING: {} pixels ({:.1}%) have zero value - shader thread may not have executed.",
                zero_count,
                (zero_count as f64 / pixel_count as f64) * 100.0
            );
        }

        // Warning: If very few sentinels on pass 1-6, something is wrong
        // Pass 1 should have ~98.4% sentinels, Pass 7 should have ~50% sentinels
        let expected_sentinel_pct = match pass.step() {
            1 | 2 => 98.4,
            3 => 93.75,
            4 => 87.5,
            5 => 75.0,
            6 => 50.0,
            7 => 0.0, // Pass 7 computes remaining 50%
            _ => 0.0,
        };
        let actual_sentinel_pct = (sentinel_count as f64 / pixel_count as f64) * 100.0;

        if pass.step() > 0 && pass.step() < 7 && actual_sentinel_pct < expected_sentinel_pct * 0.5 {
            log::error!(
                "GPU TIMEOUT SUSPECTED: Pass {} has {:.1}% sentinels, expected ~{:.1}%. \
                 GPU may have timed out or returned incomplete results.",
                pass.step(),
                actual_sentinel_pct,
                expected_sentinel_pct
            );
        }

        // Convert to ComputeData
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(glitch_data.iter())
            .zip(z_norm_sq_data.iter())
            .map(|((&iter, &glitch_flag), &z_sq)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations && iter != SENTINEL_NOT_COMPUTED,
                    glitched: glitch_flag != 0,
                    final_z_norm_sq: z_sq,
                })
            })
            .collect();

        let end = Self::now();

        Ok(GpuPerturbationHDRResult {
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

        // In WASM, we need to poll to drive the GPU work to completion.
        // The browser's WebGPU implementation handles this via requestAnimationFrame internally,
        // but we can help by yielding and polling.
        #[cfg(target_arch = "wasm32")]
        {
            // Poll until the map_async callback fires
            self.context.device.poll(wgpu::Maintain::Poll);
        }

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
