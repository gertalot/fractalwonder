//! GPU buffer management for compute shader.

use bytemuck::{Pod, Zeroable};

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

    // BLA configuration
    pub bla_enabled: u32,
    pub bla_num_levels: u32,
    pub _pad7: [u32; 2], // Padding for 16-byte alignment of bla_level_offsets
    pub bla_level_offsets: [u32; 32],
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
        bla_enabled: bool,
        bla_num_levels: u32,
        bla_level_offsets: &[usize],
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
            bla_enabled: if bla_enabled { 1 } else { 0 },
            bla_num_levels,
            _pad7: [0, 0],
            bla_level_offsets: {
                let mut offsets = [0u32; 32];
                for (i, &offset) in bla_level_offsets.iter().take(32).enumerate() {
                    offsets[i] = offset as u32;
                }
                offsets
            },
        }
    }
}

/// GPU buffers for progressive row-set rendering.
/// Includes persistent state buffers for iteration chunking.
/// Buffer consolidation: Uses 10 storage buffers to fit within WebGPU browser limits.
/// escaped+glitch → flags_buf (bit 0 = escaped, bit 1 = glitch)
pub struct ProgressiveGpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,

    // Persistent state (read-write, kept on GPU between chunks)
    // z_state: combined z_re + z_im (6 f32s per pixel)
    pub z_state: wgpu::Buffer,
    // drho_state: combined drho_re + drho_im (6 f32s per pixel)
    pub drho_state: wgpu::Buffer,
    pub iter_count: wgpu::Buffer,
    // flags_buf: bit 0 = escaped, bit 1 = glitch (packed to stay within 10 storage buffer limit)
    pub flags_buf: wgpu::Buffer,
    pub orbit_index: wgpu::Buffer,

    // Results (read back on row-set completion)
    pub results: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,
    // final_values: combined z_re, z_im, der_re, der_im (4 f32s per pixel)
    pub final_values: wgpu::Buffer,

    // BLA acceleration data (read-only)
    pub bla_data: wgpu::Buffer,
    pub bla_entry_count: u32,

    // Staging buffers for CPU readback
    pub staging_results: wgpu::Buffer,
    pub staging_flags: wgpu::Buffer, // For reading back flags_buf (glitch = bit 1)
    pub staging_z_norm_sq: wgpu::Buffer,
    pub staging_final_values: wgpu::Buffer,

    // Sync buffer for WASM chunk synchronization (4 bytes)
    pub sync_staging: wgpu::Buffer,

    pub orbit_capacity: u32,
    pub row_set_pixel_count: u32,
}

impl ProgressiveGpuBuffers {
    /// Create buffers sized for a row-set.
    /// row_set_pixel_count = (image_height / row_set_count) * image_width (rounded up)
    pub fn new(
        device: &wgpu::Device,
        orbit_len: u32,
        row_set_pixel_count: u32,
        bla_entry_count: u32,
    ) -> Self {
        let pixel_count = row_set_pixel_count as usize;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_uniforms"),
            size: std::mem::size_of::<ProgressiveGpuUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Orbit stored as 12 f32s per point:
        // [Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp,
        //  Der_re_head, Der_re_tail, Der_im_head, Der_im_tail, Der_re_exp, Der_im_exp]
        // This uses full HDRFloat representation matching the CPU implementation
        let reference_orbit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_reference_orbit"),
            size: (orbit_len as usize * std::mem::size_of::<[f32; 12]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // z_state: combined z_re + z_im (6 f32s per pixel: z_re head/tail/exp, z_im head/tail/exp)
        let z_state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_state"),
            size: (pixel_count * 6 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // drho_state: combined drho_re + drho_im (6 f32s per pixel)
        let drho_state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_drho_state"),
            size: (pixel_count * 6 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let iter_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_iter_count"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // flags_buf: bit 0 = escaped, bit 1 = glitch (packed to stay within 10 storage buffer limit)
        let flags_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_flags_buf"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let orbit_index = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_orbit_index"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Result buffers - need COPY_DST for clear_state_buffers, COPY_SRC for read_results
        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_norm_sq"),
            size: (pixel_count * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // final_values: combined z_re, z_im, der_re, der_im (4 f32s per pixel)
        let final_values = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_final_values"),
            size: (pixel_count * 4 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Staging buffers
        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_flags"),
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

        // staging_final_values: 4 f32s per pixel (z_re, z_im, der_re, der_im)
        let staging_final_values = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_final_values"),
            size: (pixel_count * 4 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Tiny sync buffer for WASM chunk synchronization
        let sync_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_sync_staging"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // BLA data: 16 f32s per entry (64 bytes)
        let bla_data = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_bla_data"),
            size: (bla_entry_count as usize * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            reference_orbit,
            z_state,
            drho_state,
            iter_count,
            flags_buf,
            orbit_index,
            results,
            z_norm_sq,
            final_values,
            staging_results,
            staging_flags,
            staging_z_norm_sq,
            staging_final_values,
            sync_staging,
            bla_data,
            bla_entry_count,
            orbit_capacity: orbit_len,
            row_set_pixel_count,
        }
    }
}
