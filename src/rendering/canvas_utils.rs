use crate::rendering::{renderer_trait::Renderer, viewport::Viewport, PixelRect};
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub fn render_with_viewport<R>(
    canvas: &HtmlCanvasElement,
    renderer: &R,
    viewport: &Viewport<R::Coord>,
) where
    R: Renderer,
    R::Coord: Clone,
{
    let width = canvas.width();
    let height = canvas.height();
    let pixel_rect = PixelRect::full_canvas(width, height);
    let pixels = renderer.render(viewport, pixel_rect, (width, height));

    // Put pixels on canvas
    let context = canvas
        .get_context("2d")
        .expect("Failed to get context")
        .expect("Context is None")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("Failed to cast to 2D context");

    let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&pixels), width, height)
        .expect("Failed to create ImageData");

    context
        .put_image_data(&image_data, 0.0, 0.0)
        .expect("Failed to put image data");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{coords::Coord, coords::Rect};

    // Mock renderer for testing
    struct MockRenderer {
        color: (u8, u8, u8, u8),
    }

    impl Renderer for MockRenderer {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0))
        }

        fn render(
            &self,
            _viewport: &Viewport<f64>,
            pixel_rect: PixelRect,
            _canvas_size: (u32, u32),
        ) -> Vec<u8> {
            let mut pixels = vec![0u8; (pixel_rect.width * pixel_rect.height * 4) as usize];
            for i in 0..(pixel_rect.width * pixel_rect.height) as usize {
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
        let viewport = Viewport::new(Coord::new(0.0, 0.0), 1.0, renderer.natural_bounds());
        let pixel_rect = PixelRect::full_canvas(100, 100);
        let pixels = renderer.render(&viewport, pixel_rect, (100, 100));
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_mock_renderer_fills_with_color() {
        let renderer = MockRenderer {
            color: (128, 64, 32, 255),
        };
        let viewport = Viewport::new(Coord::new(0.0, 0.0), 1.0, renderer.natural_bounds());
        let pixel_rect = PixelRect::full_canvas(10, 10);
        let pixels = renderer.render(&viewport, pixel_rect, (10, 10));

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
