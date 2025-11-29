//! High-level GPU renderer API.

use crate::buffers::{GpuBuffers, Uniforms};
use crate::device::GpuContext;
use crate::error::GpuError;
use crate::pipeline::GpuPipeline;
use crate::Pass;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Result of a GPU render operation.
pub struct GpuRenderResult {
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

impl GpuRenderResult {
    pub fn has_glitches(&self) -> bool {
        self.data.iter().any(|d| match d {
            ComputeData::Mandelbrot(m) => m.glitched,
            _ => false,
        })
    }

    pub fn glitched_pixel_count(&self) -> usize {
        self.data
            .iter()
            .filter(|d| match d {
                ComputeData::Mandelbrot(m) => m.glitched,
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

        // Write uniforms
        let uniforms = Uniforms::new(width, height, max_iterations, tau_sq, dc_origin, dc_step);
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
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("delta_iteration_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline.compute_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
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

        // Convert to ComputeData (same format as CPU)
        let data: Vec<ComputeData> = iterations
            .iter()
            .zip(glitch_data.iter())
            .map(|(&iter, &glitch_flag)| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: iter,
                    max_iterations,
                    escaped: iter < max_iterations,
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

    /// Render a single pass at reduced resolution/iterations.
    ///
    /// Returns ComputeData for `pass.dimensions()` pixels, NOT full canvas size.
    /// Caller is responsible for stretching to full size.
    #[allow(clippy::too_many_arguments)]
    pub async fn render_pass(
        &mut self,
        orbit: &[(f64, f64)],
        orbit_id: u32,
        dc_origin: (f32, f32),
        viewport_width: f32,
        viewport_height: f32,
        canvas_width: u32,
        canvas_height: u32,
        max_iterations: u32,
        tau_sq: f32,
        pass: Pass,
    ) -> Result<GpuRenderResult, GpuError> {
        let (pass_w, pass_h) = pass.dimensions(canvas_width, canvas_height);
        let pass_max_iter = pass.scale_iterations(max_iterations);
        let pass_tau_sq = if pass.is_final() { tau_sq } else { 0.0 };

        // Compute dc_step for this pass resolution
        let dc_step = (
            viewport_width / pass_w as f32,
            viewport_height / pass_h as f32,
        );

        self.render(
            orbit,
            orbit_id,
            dc_origin,
            dc_step,
            pass_w,
            pass_h,
            pass_max_iter,
            pass_tau_sq,
        )
        .await
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
