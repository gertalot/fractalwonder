//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod constants;
mod device;
mod error;
mod pass;
mod perturbation_hdr_pipeline;
mod perturbation_hdr_renderer;
mod pipeline;
mod progressive_pipeline;
mod progressive_renderer;
mod renderer;
mod stretch;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use buffers::{
    GpuBuffers, PerturbationHDRBuffers, PerturbationHDRUniforms, ProgressiveGpuBuffers, ProgressiveGpuUniforms,
    Uniforms,
};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use perturbation_hdr_pipeline::PerturbationHDRPipeline;
pub use perturbation_hdr_renderer::{GpuPerturbationHDRRenderer, GpuTileResult};
pub use pipeline::GpuPipeline;
pub use progressive_pipeline::ProgressiveGpuPipeline;
pub use progressive_renderer::{ProgressiveGpuRenderer, ProgressiveRowSetResult};
pub use renderer::{GpuPerturbationRenderer, GpuRenderResult};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
