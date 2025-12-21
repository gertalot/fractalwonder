//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod progressive_pipeline;
mod progressive_renderer;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use buffers::{ProgressiveGpuBuffers, ProgressiveGpuUniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use progressive_pipeline::ProgressiveGpuPipeline;
pub use progressive_renderer::{ProgressiveGpuRenderer, ProgressiveRowSetResult};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
