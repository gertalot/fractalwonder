//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod perturbation_hdr_pipeline;
mod perturbation_hdr_renderer;
mod progressive_pipeline;
mod progressive_renderer;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use buffers::{
    PerturbationHDRBuffers, PerturbationHDRUniforms, ProgressiveGpuBuffers, ProgressiveGpuUniforms,
};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use perturbation_hdr_pipeline::PerturbationHDRPipeline;
pub use perturbation_hdr_renderer::{GpuPerturbationHDRRenderer, GpuTileResult};
pub use progressive_pipeline::ProgressiveGpuPipeline;
pub use progressive_renderer::{ProgressiveGpuRenderer, ProgressiveRowSetResult};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
