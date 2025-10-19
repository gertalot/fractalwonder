use crate::rendering::{calculate_visible_bounds, renderer_trait::CanvasRenderer, Viewport};
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

/// Renders a scene to a canvas using a renderer and viewport
///
/// This is a generic utility that works with any `CanvasRenderer` implementation.
/// It handles the full rendering pipeline:
/// 1. Gets the 2D context from the canvas
/// 2. Calculates visible bounds from the viewport
/// 3. Calls the renderer to generate pixel data
/// 4. Puts the pixel data onto the canvas
///
/// # Type Parameters
///
/// * `R` - Any type implementing the `CanvasRenderer` trait
///
/// # Arguments
///
/// * `canvas` - The HTML canvas element to render to
/// * `renderer` - The renderer that generates pixel data
/// * `viewport` - The viewport defining what portion of the scene to render
pub fn render_with_viewport<R>(
    canvas: &web_sys::HtmlCanvasElement,
    renderer: &R,
    viewport: &Viewport<R::Coord>,
) where
    R: CanvasRenderer,
    R::Coord: Clone
        + std::ops::Sub<Output = R::Coord>
        + std::ops::Add<Output = R::Coord>
        + std::ops::Div<f64, Output = R::Coord>
        + std::ops::Mul<f64, Output = R::Coord>,
{
    let context = canvas
        .get_context("2d")
        .expect("should get 2d context")
        .expect("context should not be null")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("should cast to CanvasRenderingContext2d");

    let width = canvas.width();
    let height = canvas.height();

    // Calculate visible bounds from viewport
    let visible_bounds = calculate_visible_bounds(viewport, width, height);

    // Render the pattern
    let pixel_data = renderer.render(&visible_bounds, width, height);

    // Put pixels on canvas
    let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&pixel_data),
        width,
        height,
    )
    .expect("should create ImageData");

    context
        .put_image_data(&image_data, 0.0, 0.0)
        .expect("should put image data");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{coords::Coord, coords::Rect};

    // Mock renderer for testing
    struct MockRenderer {
        color: (u8, u8, u8, u8),
    }

    impl CanvasRenderer for MockRenderer {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0))
        }

        fn render(&self, _target_rect: &Rect<f64>, width: u32, height: u32) -> Vec<u8> {
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            for i in 0..(width * height) as usize {
                pixels[i * 4] = self.color.0;
                pixels[i * 4 + 1] = self.color.1;
                pixels[i * 4 + 2] = self.color.2;
                pixels[i * 4 + 3] = self.color.3;
            }
            pixels
        }
    }

    #[test]
    fn test_mock_renderer_produces_correct_pixel_count() {
        let renderer = MockRenderer {
            color: (255, 0, 0, 255),
        };
        let bounds = Rect::new(Coord::new(-10.0, -10.0), Coord::new(10.0, 10.0));
        let pixels = renderer.render(&bounds, 100, 100);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_mock_renderer_fills_with_color() {
        let renderer = MockRenderer {
            color: (128, 64, 32, 255),
        };
        let bounds = Rect::new(Coord::new(-10.0, -10.0), Coord::new(10.0, 10.0));
        let pixels = renderer.render(&bounds, 10, 10);

        // Check first pixel
        assert_eq!(pixels[0], 128);
        assert_eq!(pixels[1], 64);
        assert_eq!(pixels[2], 32);
        assert_eq!(pixels[3], 255);

        // Check last pixel
        let last_idx = (10 * 10 - 1) * 4;
        assert_eq!(pixels[last_idx], 128);
        assert_eq!(pixels[last_idx + 1], 64);
        assert_eq!(pixels[last_idx + 2], 32);
        assert_eq!(pixels[last_idx + 3], 255);
    }
}
