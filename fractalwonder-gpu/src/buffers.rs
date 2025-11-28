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
    pub _padding: [u32; 3],
}

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

/// Manages GPU buffers for rendering.
pub struct GpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
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

        Self {
            uniforms,
            reference_orbit,
            results,
            glitch_flags,
            staging_results,
            staging_glitches,
            orbit_capacity: orbit_len,
            pixel_count,
        }
    }
}
