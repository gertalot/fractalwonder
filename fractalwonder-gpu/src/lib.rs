//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pipeline;
mod renderer;
#[cfg(test)]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
