pub mod canvas_renderer;
pub mod colorizers;
pub mod parallel_canvas_renderer;
pub mod presentation_config;

pub use canvas_renderer::CanvasRenderer;
pub use colorizers::{Colorizer, ColorizerInfo, RendererColorizers, COLORIZERS};
pub use parallel_canvas_renderer::ParallelCanvasRenderer;
pub use presentation_config::{
    get_colorizer, get_colorizers_for_renderer, get_default_colorizer_id, get_renderer_config,
    RendererPresentationConfig, RENDERER_CONFIGS,
};

// Re-export compute types
pub use fractalwonder_compute::{
    create_renderer, AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer,
    PrecisionCalculator, Renderer, RendererInfo, TestImageComputer,
};
pub use fractalwonder_core::{
    apply_pixel_transform_to_viewport, AppData, BigFloat, Point, Rect, ToF64, Viewport,
};

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
