pub mod adaptive_mandelbrot_renderer;
pub mod app_data_renderer;
pub mod computers;
pub mod pixel_renderer;
pub mod point_compute;
pub mod precision;
pub mod render_config;
pub mod renderer_info;
pub mod renderer_trait;

pub use adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;
pub use app_data_renderer::AppDataRenderer;
pub use computers::{MandelbrotComputer, TestImageComputer};
pub use pixel_renderer::PixelRenderer;
pub use point_compute::ImagePointComputer;
pub use precision::PrecisionCalculator;
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
pub use renderer_info::{RendererInfo, RendererInfoData};
pub use renderer_trait::Renderer;

// Re-export core types for convenience
pub use fractalwonder_core::*;
