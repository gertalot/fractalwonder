use crate::rendering::{points::Rect, viewport::Viewport, PixelRect};

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
