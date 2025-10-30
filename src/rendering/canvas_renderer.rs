use crate::rendering::{
    points::Rect, renderer_trait::Renderer, viewport::Viewport, AppData, Colorizer,
};
use web_sys::HtmlCanvasElement;

/// Canvas renderer trait - takes a Renderer and Colorizer to render RGBA pixels on a canvas
///
/// Implementations handle the strategy for putting computed data onto canvas pixels:
/// - TilingCanvasRenderer: progressive tiled rendering with caching
/// - Future: SimpleCanvasRenderer, OffscreenCanvasRenderer, etc.
pub trait CanvasRenderer {
    type Scalar;
    type Data: Clone;

    /// Swap the renderer at runtime (invalidates cache)
    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>);

    /// Swap the colorizer at runtime (preserves cache if implementation supports it)
    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>);

    /// Main rendering entry point - renders viewport to canvas
    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement);

    /// Get natural bounds from the underlying renderer
    fn natural_bounds(&self) -> Rect<Self::Scalar>;

    /// Cancel any in-progress render
    fn cancel_render(&self);
}
