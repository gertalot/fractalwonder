pub mod canvas_renderer;
pub mod canvas_utils;
pub mod colorizers;
pub mod tiling_canvas_renderer;

pub use canvas_renderer::CanvasRenderer;
pub use canvas_utils::render_with_viewport;
pub use colorizers::{
    mandelbrot_default_colorizer, mandelbrot_fire_colorizer, mandelbrot_opal_colorizer,
    test_image_default_colorizer, test_image_pastel_colorizer, Colorizer,
};
pub use tiling_canvas_renderer::TilingCanvasRenderer;

// Re-export commonly used types from core and compute for convenience in UI code
pub use fractalwonder_compute::{
    get_color_scheme, get_config, AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer,
    PrecisionCalculator, Renderer, RenderConfig, TestImageComputer, RENDER_CONFIGS,
};
pub use fractalwonder_core::{
    apply_pixel_transform_to_viewport, AppData, BigFloat, Point, Rect, ToF64, Viewport,
};

/// Get colorizer function for a specific renderer and color scheme
pub fn get_colorizer(renderer_id: &str, scheme_id: &str) -> Colorizer<AppData> {
    match (renderer_id, scheme_id) {
        ("test_image", "default") => test_image_default_colorizer,
        ("test_image", "pastel") => test_image_pastel_colorizer,
        ("mandelbrot", "default") => mandelbrot_default_colorizer,
        ("mandelbrot", "fire") => mandelbrot_fire_colorizer,
        ("mandelbrot", "opal") => mandelbrot_opal_colorizer,
        _ => panic!("Unknown renderer/scheme combination: {}/{}", renderer_id, scheme_id),
    }
}
