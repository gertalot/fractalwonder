pub mod adaptive_mandelbrot_renderer;
pub mod app_data_renderer;
pub mod computers;
pub mod messages;
pub mod pixel_renderer;
pub mod point_compute;
pub mod precision;
pub mod render_config;
pub mod renderer_info;
pub mod renderer_trait;

#[cfg(target_arch = "wasm32")]
pub mod worker;

pub use adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
pub use app_data_renderer::AppDataRenderer;
pub use computers::{MandelbrotComputer, TestImageComputer};
pub use messages::{MainToWorker, WorkerToMain};
pub use pixel_renderer::PixelRenderer;
pub use point_compute::ImagePointComputer;
pub use precision::PrecisionCalculator;
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
pub use renderer_info::{RendererInfo, RendererInfoData};
pub use renderer_trait::Renderer;

#[cfg(target_arch = "wasm32")]
pub use worker::init_message_worker;

// Re-export core types for convenience
pub use fractalwonder_core::*;
