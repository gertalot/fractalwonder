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
    pub adam7_step: u32,        // 0 = compute all, 1-7 = Adam7 pass
    pub reference_escaped: u32, // 1 if reference orbit escaped (short orbit), 0 otherwise
    pub _padding: u32,
}

impl Uniforms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: (f32, f32),
        dc_step: (f32, f32),
        adam7_step: u32,
        reference_escaped: bool,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0, // 256² for smooth coloring
            tau_sq,
            dc_origin_re: dc_origin.0,
            dc_origin_im: dc_origin.1,
            dc_step_re: dc_step.0,
            dc_step_im: dc_step.1,
            adam7_step,
            reference_escaped: if reference_escaped { 1 } else { 0 },
            _padding: 0,
        }
    }
}

/// Uniform data for direct HDRFloat compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DirectHDRUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,

    // c_origin as HDRFloat: (head, tail, exp) for re and im
    pub c_origin_re_head: f32,
    pub c_origin_re_tail: f32,
    pub c_origin_re_exp: i32,
    pub _pad1: u32,
    pub c_origin_im_head: f32,
    pub c_origin_im_tail: f32,
    pub c_origin_im_exp: i32,
    pub _pad2: u32,

    // c_step as HDRFloat
    pub c_step_re_head: f32,
    pub c_step_re_tail: f32,
    pub c_step_re_exp: i32,
    pub _pad3: u32,
    pub c_step_im_head: f32,
    pub c_step_im_tail: f32,
    pub c_step_im_exp: i32,

    pub adam7_step: u32,
}

impl DirectHDRUniforms {
    #[allow(dead_code)]
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        c_origin: ((f32, f32, i32), (f32, f32, i32)), // ((re_head, re_tail, re_exp), (im_head, im_tail, im_exp))
        c_step: ((f32, f32, i32), (f32, f32, i32)),
        adam7_step: u32,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0,
            c_origin_re_head: c_origin.0 .0,
            c_origin_re_tail: c_origin.0 .1,
            c_origin_re_exp: c_origin.0 .2,
            _pad1: 0,
            c_origin_im_head: c_origin.1 .0,
            c_origin_im_tail: c_origin.1 .1,
            c_origin_im_exp: c_origin.1 .2,
            _pad2: 0,
            c_step_re_head: c_step.0 .0,
            c_step_re_tail: c_step.0 .1,
            c_step_re_exp: c_step.0 .2,
            _pad3: 0,
            c_step_im_head: c_step.1 .0,
            c_step_im_tail: c_step.1 .1,
            c_step_im_exp: c_step.1 .2,
            adam7_step,
        }
    }
}

/// Uniform data for perturbation HDRFloat compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PerturbationHDRUniforms {
    pub image_width: u32,  // Full image width (for δc calculation)
    pub image_height: u32, // Full image height
    pub max_iterations: u32,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,
    pub _pad0: u32,

    // dc_origin as HDRFloat
    pub dc_origin_re_head: f32,
    pub dc_origin_re_tail: f32,
    pub dc_origin_re_exp: i32,
    pub _pad1: u32,
    pub dc_origin_im_head: f32,
    pub dc_origin_im_tail: f32,
    pub dc_origin_im_exp: i32,
    pub _pad2: u32,

    // dc_step as HDRFloat
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
    pub _pad4: [u32; 3], // Pad struct to 120 bytes (WGSL uniform buffer alignment)
}

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
            _pad4: [0, 0, 0],
        }
    }
}

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
        }
    }
}

/// GPU buffers for progressive row-set rendering.
/// Includes persistent state buffers for iteration chunking.
pub struct ProgressiveGpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,

    // Persistent state (read-write, kept on GPU between chunks)
    pub z_re: wgpu::Buffer,
    pub z_im: wgpu::Buffer,
    pub iter_count: wgpu::Buffer,
    pub escaped: wgpu::Buffer,
    pub orbit_index: wgpu::Buffer,

    // Results (read back on row-set completion)
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,

    // Staging buffers for CPU readback
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,

    pub orbit_capacity: u32,
    pub row_set_pixel_count: u32,
}

impl ProgressiveGpuBuffers {
    /// Create buffers sized for a row-set.
    /// row_set_pixel_count = (image_height / row_set_count) * image_width (rounded up)
    pub fn new(device: &wgpu::Device, orbit_len: u32, row_set_pixel_count: u32) -> Self {
        let pixel_count = row_set_pixel_count as usize;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_uniforms"),
            size: std::mem::size_of::<ProgressiveGpuUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reference_orbit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_reference_orbit"),
            size: (orbit_len as usize * std::mem::size_of::<[f32; 2]>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Persistent state buffers - HDRFloat z uses 3 f32s per component (head, tail, exp as f32)
        let z_re = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_re"),
            size: (pixel_count * 3 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let z_im = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_im"),
            size: (pixel_count * 3 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let iter_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_iter_count"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let escaped = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_escaped"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let orbit_index = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_orbit_index"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Result buffers
        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let glitch_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_glitch_flags"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_z_norm_sq"),
            size: (pixel_count * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Staging buffers
        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_results"),
            size: (pixel_count * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_glitches = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_staging_glitches"),
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

        Self {
            uniforms,
            reference_orbit,
            z_re,
            z_im,
            iter_count,
            escaped,
            orbit_index,
            results,
            glitch_flags,
            z_norm_sq,
            staging_results,
            staging_glitches,
            staging_z_norm_sq,
            orbit_capacity: orbit_len,
            row_set_pixel_count,
        }
    }
}

/// GPU buffers for perturbation HDRFloat rendering.
/// Buffers are sized for the provided tile size.
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
    pub fn new(device: &wgpu::Device, orbit_len: u32, tile_size: u32) -> Self {
        let tile_pixels = (tile_size * tile_size) as usize;

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

/// Manages GPU buffers for rendering.
pub struct GpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,
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

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
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
            pixel_count,
        }
    }
}
