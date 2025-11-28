//! GPU error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("No GPU adapter found")]
    NoAdapter,

    #[error("Failed to create device: {0}")]
    DeviceCreation(#[from] wgpu::RequestDeviceError),

    #[error("Buffer mapping failed: {0}")]
    BufferMap(#[from] wgpu::BufferAsyncError),

    #[error("GPU unavailable: {0}")]
    Unavailable(String),
}
