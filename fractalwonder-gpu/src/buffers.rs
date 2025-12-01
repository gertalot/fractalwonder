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
    pub _pad4: [u32; 2], // Pad to 16-byte alignment
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
            _pad4: [0, 0],
        }
    }
}

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
