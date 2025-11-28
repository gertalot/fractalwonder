//! GPU device initialization and capability detection.

use crate::error::GpuError;

/// Holds the wgpu device and queue.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Result of GPU initialization attempt.
pub enum GpuAvailability {
    Available(GpuContext),
    Unavailable(String),
}

impl GpuContext {
    /// Attempt to initialize GPU. Returns Unavailable on any failure.
    pub async fn try_init() -> GpuAvailability {
        match Self::init_internal().await {
            Ok(ctx) => GpuAvailability::Available(ctx),
            Err(e) => {
                log::warn!("GPU initialization failed: {e}");
                GpuAvailability::Unavailable(e.to_string())
            }
        }
    }

    async fn init_internal() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        log::info!("GPU adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("fractalwonder"),
                    required_features: wgpu::Features::empty(),
                    // Use adapter limits directly - we need compute shaders which require
                    // storage buffers (not available in WebGL2 compatibility mode)
                    required_limits: adapter.limits(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        Ok(Self { device, queue })
    }
}
