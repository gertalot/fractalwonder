pub mod canvas_renderer;
pub mod colorizers;
pub mod message_parallel_renderer;
pub mod tile_size;

pub use canvas_renderer::CanvasRenderer;
pub use colorizers::{
    mandelbrot_default_colorizer, mandelbrot_fire_colorizer, mandelbrot_opal_colorizer,
    test_image_default_colorizer, test_image_pastel_colorizer, Colorizer,
};
pub use message_parallel_renderer::MessageParallelRenderer;
pub use tile_size::calculate_tile_size;

/// Progress information for ongoing renders
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub render_id: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    pub fn new(total_tiles: u32, render_id: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            render_id,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}

impl Default for RenderProgress {
    fn default() -> Self {
        Self {
            completed_tiles: 0,
            total_tiles: 0,
            render_id: 0,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }
}

// Re-export commonly used types from core and compute for convenience in UI code
pub use fractalwonder_compute::{
    get_color_scheme, get_config, AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer,
    PrecisionCalculator, RenderConfig, Renderer, TestImageComputer, RENDER_CONFIGS,
};
pub use fractalwonder_core::{
    apply_pixel_transform_to_viewport, AppData, BigFloat, Point, Rect, ToF64, Viewport,
};

/// Get colorizer function for a specific renderer and color scheme.
/// Returns None if the renderer/scheme combination is unknown.
pub fn get_colorizer(renderer_id: &str, scheme_id: &str) -> Option<Colorizer<AppData>> {
    match (renderer_id, scheme_id) {
        ("test_image", "default") => Some(test_image_default_colorizer),
        ("test_image", "pastel") => Some(test_image_pastel_colorizer),
        ("mandelbrot", "default") => Some(mandelbrot_default_colorizer),
        ("mandelbrot", "fire") => Some(mandelbrot_fire_colorizer),
        ("mandelbrot", "opal") => Some(mandelbrot_opal_colorizer),
        _ => None,
    }
}
