//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod direct_pipeline;
mod direct_renderer;
mod error;
mod pass;
mod perturbation_floatexp_pipeline;
mod pipeline;
mod renderer;
mod stretch;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use buffers::{DirectFloatExpBuffers, DirectFloatExpUniforms, GpuBuffers, PerturbationFloatExpUniforms, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use direct_pipeline::DirectFloatExpPipeline;
pub use direct_renderer::{DirectFloatExpRenderer, DirectFloatExpResult};
pub use error::GpuError;
pub use pass::Adam7Pass;
pub use perturbation_floatexp_pipeline::PerturbationFloatExpPipeline;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
pub use stretch::{Adam7Accumulator, SENTINEL_NOT_COMPUTED};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
