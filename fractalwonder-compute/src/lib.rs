mod test_image;

use fractalwonder_core::Viewport;

pub use test_image::TestImageRenderer;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::Viewport;

    #[test]
    fn test_image_renderer_produces_correct_size() {
        let renderer = TestImageRenderer;
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn test_image_renderer_origin_detected() {
        let renderer = TestImageRenderer;
        // Viewport centered at origin
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 100));
        // Center pixel should be on origin
        let center_idx = 50 * 100 + 50;
        assert!(result[center_idx].is_on_origin);
    }
}
