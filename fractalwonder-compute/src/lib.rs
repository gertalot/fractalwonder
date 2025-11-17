pub mod adaptive_mandelbrot_renderer;
pub mod app_data_renderer;
pub mod computers;
pub mod pixel_renderer;
pub mod point_compute;
pub mod precision;
pub mod render_config;
pub mod renderer_info;
pub mod renderer_trait;
pub mod shared_buffer;
pub mod worker_messages;

#[cfg(target_arch = "wasm32")]
pub mod atomics;

#[cfg(target_arch = "wasm32")]
pub mod worker;

pub use adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
pub use app_data_renderer::AppDataRenderer;
pub use computers::{MandelbrotComputer, TestImageComputer};
pub use pixel_renderer::PixelRenderer;
pub use point_compute::ImagePointComputer;
pub use precision::PrecisionCalculator;
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
pub use renderer_info::{RendererInfo, RendererInfoData};
pub use renderer_trait::Renderer;
pub use shared_buffer::SharedBufferLayout;
pub use worker_messages::{WorkerRequest, WorkerResponse};

#[cfg(target_arch = "wasm32")]
pub use atomics::{atomic_fetch_add_u32, atomic_load_u32, atomic_store_u32};

#[cfg(target_arch = "wasm32")]
pub use worker::{init_worker, process_render_request, handle_message};

// Re-export core types for convenience
pub use fractalwonder_core::*;
