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
        derivative_orbit: &[(f64, f64)],
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
        let needs_new_buffers = self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0)
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
        // Store as 12 f32s per point:
        // [Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp,
        //  Der_re_head, Der_re_tail, Der_im_head, Der_im_tail, Der_re_exp, Der_im_exp]
        // Uses HDRFloat representation: value = (head + tail) × 2^exp, head in [0.5, 1.0)
        if self.cached_orbit_id != Some(orbit_id) {
            let orbit_data: Vec<[f32; 12]> = orbit
                .iter()
                .zip(derivative_orbit.iter())
                .map(|(&(z_re, z_im), &(der_re, der_im))| {
                    // Convert to HDRFloat format matching CPU implementation
                    let z_re_hdr = fractalwonder_core::HDRFloat::from_f64(z_re);
                    let z_im_hdr = fractalwonder_core::HDRFloat::from_f64(z_im);
                    let der_re_hdr = fractalwonder_core::HDRFloat::from_f64(der_re);
                    let der_im_hdr = fractalwonder_core::HDRFloat::from_f64(der_im);
                    [
                        z_re_hdr.head,
                        z_re_hdr.tail,
                        z_im_hdr.head,
                        z_im_hdr.tail,
                        // Pack exponents as f32 for GPU (will be bitcast to i32)
                        f32::from_bits(z_re_hdr.exp as u32),
                        f32::from_bits(z_im_hdr.exp as u32),
                        der_re_hdr.head,
                        der_re_hdr.tail,
                        der_im_hdr.head,
                        der_im_hdr.tail,
                        f32::from_bits(der_re_hdr.exp as u32),
                        f32::from_bits(der_im_hdr.exp as u32),
                    ]
                })
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

            // Wait for dispatch to complete using sync buffer
            self.sync_after_dispatch().await?;
        }

        // Read back results
        let (iterations, glitch_data, z_norm_sq_data) =
            self.read_results(row_set_pixel_count as usize).await?;

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
                    final_z_re: 0.0,
                    final_z_im: 0.0,
                    final_derivative_re: 0.0,
                    final_derivative_im: 0.0,
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

        // Initialize results to a sentinel value (999) to detect if shader writes to it
        let sentinel_results: Vec<u32> = vec![999; pixel_count as usize];
        self.context.queue.write_buffer(
            &buffers.results,
            0,
            bytemuck::cast_slice(&sentinel_results),
        );

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

    /// Synchronize after a dispatch by copying to sync buffer and awaiting map.
    /// This ensures the compute shader has finished before proceeding.
    async fn sync_after_dispatch(&self) -> Result<(), GpuError> {
        let buffers = self.buffers.as_ref().unwrap();

        // Copy 4 bytes from results buffer to sync staging buffer
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sync_encoder"),
                });

        encoder.copy_buffer_to_buffer(
            &buffers.results,
            0,
            &buffers.sync_staging,
            0,
            std::mem::size_of::<u32>() as u64,
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Map and await - this blocks until the copy (and all preceding work) completes
        let slice = buffers.sync_staging.slice(..);
        let (tx, rx) = futures_channel::oneshot::channel();

        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        // Poll to drive the async operation
        #[cfg(target_arch = "wasm32")]
        self.context.device.poll(wgpu::Maintain::Poll);

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| GpuError::Unavailable("Sync channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        buffers.sync_staging.unmap();
        Ok(())
    }

    async fn read_results(&self, count: usize) -> Result<(Vec<u32>, Vec<u32>, Vec<f32>), GpuError> {
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

        encoder.copy_buffer_to_buffer(
            &buffers.results,
            0,
            &buffers.staging_results,
            0,
            u32_byte_size,
        );
        encoder.copy_buffer_to_buffer(
            &buffers.glitch_flags,
            0,
            &buffers.staging_glitches,
            0,
            u32_byte_size,
        );
        encoder.copy_buffer_to_buffer(
            &buffers.z_norm_sq,
            0,
            &buffers.staging_z_norm_sq,
            0,
            f32_byte_size,
        );

        self.context.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let results_slice = buffers.staging_results.slice(..u32_byte_size);
        let glitches_slice = buffers.staging_glitches.slice(..u32_byte_size);
        let z_norm_sq_slice = buffers.staging_z_norm_sq.slice(..f32_byte_size);

        let (tx1, rx1) = futures_channel::oneshot::channel();
        let (tx2, rx2) = futures_channel::oneshot::channel();
        let (tx3, rx3) = futures_channel::oneshot::channel();

        results_slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx1.send(r);
        });
        glitches_slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx2.send(r);
        });
        z_norm_sq_slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx3.send(r);
        });

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
