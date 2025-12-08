//! GPU renderer for perturbation with HDRFloat deltas.
//! Uses extended-range arithmetic to avoid precision loss at moderate zoom.

use crate::buffers::{PerturbationHDRBuffers, PerturbationHDRUniforms};
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
    cached_tile_size: u32,
}

impl GpuPerturbationHDRRenderer {
    pub fn new(context: GpuContext) -> Self {
        let pipeline = PerturbationHDRPipeline::new(&context.device);
        Self {
            context,
            pipeline,
            buffers: None,
            cached_orbit_id: None,
            cached_tile_size: 0,
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
    /// * `tile_size` - Base tile size for buffer allocation
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
        tile_size: u32,
        max_iterations: u32,
        tau_sq: f32,
        reference_escaped: bool,
    ) -> Result<GpuTileResult, GpuError> {
        let start = Self::now();
        let t0 = start;

        // Recreate buffers if orbit capacity or tile size changed
        let needs_new_buffers = self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0)
            < orbit.len() as u32
            || self.cached_tile_size < tile_size;
        if needs_new_buffers {
            log::info!(
                "Creating buffers for orbit len {}, tile size {}",
                orbit.len(),
                tile_size
            );
            self.buffers = Some(PerturbationHDRBuffers::new(
                &self.context.device,
                orbit.len() as u32,
                tile_size,
            ));
            self.cached_orbit_id = None;
            self.cached_tile_size = tile_size;
        }
        let t1 = Self::now();

        let buffers = self.buffers.as_ref().unwrap();

        // Upload orbit if changed
        // Uses full HDRFloat representation matching CPU: value = (head + tail) × 2^exp
        if self.cached_orbit_id != Some(orbit_id) {
            log::info!("Uploading orbit {} ({} points)", orbit_id, orbit.len());
            let orbit_data: Vec<[f32; 6]> = orbit
                .iter()
                .map(|&(re, im)| {
                    // Convert to HDRFloat format matching CPU implementation
                    let re_hdr = fractalwonder_core::HDRFloat::from_f64(re);
                    let im_hdr = fractalwonder_core::HDRFloat::from_f64(im);
                    [
                        re_hdr.head,
                        re_hdr.tail,
                        im_hdr.head,
                        im_hdr.tail,
                        f32::from_bits(re_hdr.exp as u32),
                        f32::from_bits(im_hdr.exp as u32),
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
        let t2 = Self::now();
        if t2 - t0 > 10.0 {
            log::info!("Setup: buffers={:.1}ms, orbit={:.1}ms", t1 - t0, t2 - t1);
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
            compute_pass.dispatch_workgroups(tile.width.div_ceil(8), tile.height.div_ceil(8), 1);
        }

        // Copy results to staging buffers (only tile pixels)
        let tile_pixels = (tile.width * tile.height) as usize;
        let u32_byte_size = (tile_pixels * std::mem::size_of::<u32>()) as u64;
        let f32_byte_size = (tile_pixels * std::mem::size_of::<f32>()) as u64;

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

        let t3 = Self::now();
        self.context.queue.submit(std::iter::once(encoder.finish()));
        let t4 = Self::now();

        // Read back all results in a single batch
        let (iterations, glitch_data, z_norm_sq_data) = self
            .read_all_buffers(
                &buffers.staging_results,
                &buffers.staging_glitches,
                &buffers.staging_z_norm_sq,
                tile_pixels,
            )
            .await?;
        let t5 = Self::now();

        log::info!(
            "Tile timing: setup={:.1}ms, submit={:.1}ms, readback={:.1}ms, total={:.1}ms",
            t3 - t2,
            t4 - t3,
            t5 - t4,
            t5 - t0
        );

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

    /// Read all 3 staging buffers in a single batch operation.
    /// Maps all buffers, polls once, then reads all data.
    async fn read_all_buffers(
        &self,
        results_buf: &wgpu::Buffer,
        glitches_buf: &wgpu::Buffer,
        z_norm_sq_buf: &wgpu::Buffer,
        count: usize,
    ) -> Result<(Vec<u32>, Vec<u32>, Vec<f32>), GpuError> {
        let u32_byte_size = (count * std::mem::size_of::<u32>()) as u64;
        let f32_byte_size = (count * std::mem::size_of::<f32>()) as u64;

        let results_slice = results_buf.slice(..u32_byte_size);
        let glitches_slice = glitches_buf.slice(..u32_byte_size);
        let z_norm_sq_slice = z_norm_sq_buf.slice(..f32_byte_size);

        // Start all 3 mappings
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

        // Single poll for all 3 buffers
        #[cfg(target_arch = "wasm32")]
        self.context.device.poll(wgpu::Maintain::Poll);

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        // Wait for all 3 to complete
        rx1.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;
        rx2.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;
        rx3.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        // Read all data
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

        // Unmap all buffers
        results_buf.unmap();
        glitches_buf.unmap();
        z_norm_sq_buf.unmap();

        Ok((iterations, glitch_data, z_norm_sq_data))
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    async fn read_buffer_f32(
        &self,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f32>, GpuError> {
        let byte_size = (count * std::mem::size_of::<f32>()) as u64;
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
