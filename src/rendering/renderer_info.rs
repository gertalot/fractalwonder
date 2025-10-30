use crate::rendering::viewport::Viewport;

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
