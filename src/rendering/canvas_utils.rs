use crate::rendering::{renderer_trait::Renderer, viewport::Viewport, Colorizer, PixelRect};
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, ContextAttributes2d, HtmlCanvasElement, ImageData};

pub fn render_with_viewport<R>(
    canvas: &HtmlCanvasElement,
    renderer: &R,
    viewport: &Viewport<R::Scalar>,
    colorizer: Colorizer<R::Data>,
) where
    R: Renderer,
    R::Scalar: Clone,
{
    let width = canvas.width();
    let height = canvas.height();
    let pixel_rect = PixelRect::full_canvas(width, height);
    let data = renderer.render(viewport, pixel_rect, (width, height));

    // Convert data to RGBA pixels
    let pixels: Vec<u8> = data
        .iter()
        .flat_map(|d| {
            let (r, g, b, a) = colorizer(d);
            [r, g, b, a]
        })
        .collect();

    // Put pixels on canvas
    let attrs = ContextAttributes2d::new();
    attrs.set_will_read_frequently(true);

    let context = canvas
        .get_context_with_context_options("2d", &attrs)
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
    use crate::rendering::{points::Point, points::Rect};

    // Mock data for testing
    #[derive(Clone, Debug, PartialEq)]
    struct MockData {
        color: (u8, u8, u8, u8),
    }

    // Mock renderer for testing
    #[derive(Clone)]
    struct MockRenderer {
        color: (u8, u8, u8, u8),
    }

    impl Renderer for MockRenderer {
        type Scalar = f64;
        type Data = MockData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0))
        }

        fn render(
            &self,
            _viewport: &Viewport<f64>,
            pixel_rect: PixelRect,
            _canvas_size: (u32, u32),
        ) -> Vec<MockData> {
            vec![MockData { color: self.color }; (pixel_rect.width * pixel_rect.height) as usize]
        }
    }

    #[allow(dead_code)]
    fn mock_colorizer(data: &MockData) -> (u8, u8, u8, u8) {
        data.color
    }

    #[test]
    fn test_mock_renderer_produces_correct_data_count() {
        let renderer = MockRenderer {
            color: (255, 0, 0, 255),
        };
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(100, 100);
        let data = renderer.render(&viewport, pixel_rect, (100, 100));
        assert_eq!(data.len(), 100 * 100);
    }

    #[test]
    fn test_mock_renderer_fills_with_correct_data() {
        let renderer = MockRenderer {
            color: (128, 64, 32, 255),
        };
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(10, 10);
        let data = renderer.render(&viewport, pixel_rect, (10, 10));

        // Check first data point
        assert_eq!(data[0].color, (128, 64, 32, 255));

        // Check last data point
        assert_eq!(data[99].color, (128, 64, 32, 255));

        // Verify all data points are consistent
        assert!(data.iter().all(|d| d.color == (128, 64, 32, 255)));
    }
}
