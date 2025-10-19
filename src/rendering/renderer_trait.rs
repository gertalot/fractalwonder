use crate::rendering::{points::Rect, viewport::Viewport, PixelRect};

/// Core trait for rendering pixel data given viewport and pixel-space dimensions
///
/// Implementations can be composed (e.g., TiledRenderer wrapping PixelRenderer)
pub trait Renderer {
    /// Coordinate type for image space (f64, rug::Float, etc.)
    type Coord;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Render pixels for a given viewport and pixel rectangle
    ///
    /// # Arguments
    /// * `viewport` - What image coordinates the full canvas shows
    /// * `pixel_rect` - Which portion of canvas to render (for tiling)
    /// * `canvas_size` - Full canvas dimensions (width, height)
    ///
    /// # Returns
    /// RGBA pixel data for the specified pixel_rect (length = width * height * 4)
    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8>;
}
