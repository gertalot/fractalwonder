//! GPU-accelerated Mandelbrot rendering using wgpu.

mod buffers;
mod device;
mod error;
mod pass;
mod pipeline;
mod renderer;
mod stretch;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use buffers::{GpuBuffers, Uniforms};
pub use device::{GpuAvailability, GpuContext};
pub use error::GpuError;
pub use pass::Adam7Pass;
pub use pipeline::GpuPipeline;
pub use renderer::{GpuRenderResult, GpuRenderer};
pub use stretch::{Adam7Accumulator, SENTINEL_NOT_COMPUTED};

// Re-export ComputeData for convenience
pub use fractalwonder_core::{ComputeData, MandelbrotData};
