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
