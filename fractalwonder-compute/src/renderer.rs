use fractalwonder_core::{PixelRect, Rect, Viewport};

// ============================================================================
// Core Renderer Trait
// ============================================================================

/// Core trait for rendering pixel data given viewport and pixel-space dimensions
///
/// Implementations can be composed (e.g., TiledRenderer wrapping PixelRenderer)
pub trait Renderer: dyn_clone::DynClone {
    /// Scalar numeric type for image-space coordinates (f64, BigFloat, etc.)
    type Scalar;

    /// Data type output (NOT colors - will be colorized later)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Scalar>;

    /// Render data for pixels in a given viewport and pixel rectangle
    ///
    /// # Arguments
    /// * `viewport` - What image coordinates the full canvas shows
    /// * `pixel_rect` - Which portion of canvas to render (for tiling)
    /// * `canvas_size` - Full canvas dimensions (width, height)
    ///
    /// # Returns
    /// Data for the specified pixel_rect (length = width * height)
    fn render(
        &self,
        viewport: &Viewport<Self::Scalar>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data>;
}

dyn_clone::clone_trait_object!(<S, D> Renderer<Scalar = S, Data = D>);

// ============================================================================
// Renderer Info
// ============================================================================

/// Optional trait for renderers to expose displayable information to the UI.
/// Combines viewport state with renderer-specific parameters.
pub trait RendererInfo {
    type Scalar;

    /// Returns current display information including viewport and custom parameters.
    /// Performance metrics (render_time_ms) are filled by InteractiveCanvas.
    fn info(&self, viewport: &Viewport<Self::Scalar>) -> RendererInfoData;
}

/// Display information for UI overlay
#[derive(Clone, Debug)]
pub struct RendererInfoData {
    /// Renderer name (e.g., "Test Image", "Mandelbrot Fractal", "Map View")
    pub name: String,

    /// Viewport center point, formatted for display by renderer
    pub center_display: String,

    /// Zoom level, formatted for display by renderer
    pub zoom_display: String,

    /// Custom renderer parameters (e.g., "Iterations: 1000", "Color: rainbow")
    /// Each tuple is (parameter_name, display_value)
    pub custom_params: Vec<(String, String)>,

    /// Performance metrics (filled by InteractiveCanvas after render)
    pub render_time_ms: Option<f64>,
}

// ============================================================================
// Renderer Factory
// ============================================================================

use crate::{AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer, TestImageComputer};
use fractalwonder_core::{AppData, BigFloat};

/// Create a renderer by ID for use by workers
pub fn create_renderer(
    renderer_id: &str,
) -> Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>> {
    match renderer_id {
        "mandelbrot" => Some(Box::new(AdaptiveMandelbrotRenderer::new(1e10))),
        "test_image" => {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
            Some(Box::new(app_renderer))
        }
        _ => None,
    }
}
