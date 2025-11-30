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
    pub adam7_step: u32, // 0 = compute all, 1-7 = Adam7 pass
    pub _padding: [u32; 2],
}

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
            escape_radius_sq: 65536.0, // 256² for smooth coloring
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

/// Uniform data for direct FloatExp compute shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DirectFloatExpUniforms {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
    pub escape_radius_sq: f32,

    pub c_origin_re_m: f32,
    pub c_origin_re_e: i32,
    pub c_origin_im_m: f32,
    pub c_origin_im_e: i32,

    pub c_step_re_m: f32,
    pub c_step_re_e: i32,
    pub c_step_im_m: f32,
    pub c_step_im_e: i32,

    pub adam7_step: u32,
    pub _padding: u32,
}

impl DirectFloatExpUniforms {
    pub fn new(
        width: u32,
        height: u32,
        max_iterations: u32,
        c_origin: (f32, i32, f32, i32),  // (re_m, re_e, im_m, im_e)
        c_step: (f32, i32, f32, i32),    // (re_m, re_e, im_m, im_e)
        adam7_step: u32,
    ) -> Self {
        Self {
            width,
            height,
            max_iterations,
            escape_radius_sq: 65536.0, // 256² for smooth coloring
            c_origin_re_m: c_origin.0,
            c_origin_re_e: c_origin.1,
            c_origin_im_m: c_origin.2,
            c_origin_im_e: c_origin.3,
            c_step_re_m: c_step.0,
            c_step_re_e: c_step.1,
            c_step_im_m: c_step.2,
            c_step_im_e: c_step.3,
            adam7_step,
            _padding: 0,
        }
    }
}

/// GPU buffers for direct FloatExp rendering.
/// Simpler than perturbation buffers - no reference orbit, no glitch flags.
pub struct DirectFloatExpBuffers {
    pub uniforms: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,
    pub pixel_count: u32,
}

impl DirectFloatExpBuffers {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let pixel_count = width * height;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_uniforms"),
            size: std::mem::size_of::<DirectFloatExpUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_results = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_staging_results"),
            size: (pixel_count as usize * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("direct_floatexp_staging_z_norm_sq"),
            size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniforms,
            results,
            z_norm_sq,
            staging_results,
            staging_z_norm_sq,
            pixel_count,
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
