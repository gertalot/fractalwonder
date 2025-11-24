use fractalwonder_core::Viewport;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}
