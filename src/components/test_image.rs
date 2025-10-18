use crate::components::canvas::Canvas;
use crate::rendering::{
    coords::{ImageCoord, ImageRect},
    renderer_trait::CanvasRenderer,
    viewport::Viewport,
};
use leptos::*;

pub struct TestImageRenderer {
    checkerboard_size: f64,
    circle_radius_step: f64,
    circle_line_thickness: f64,
}

impl TestImageRenderer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 10.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.5,
        }
    }

    fn compute_pixel_color(&self, x: f64, y: f64) -> (u8, u8, u8, u8) {
        // Check if on circle first (circles drawn on top)
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
            return (255, 0, 0, 255); // Red circle
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor() as i32;
        let square_y = (y / self.checkerboard_size).floor() as i32;
        let is_light = (square_x + square_y) % 2 == 0;

        if is_light {
            (255, 255, 255, 255) // White
        } else {
            (204, 204, 204, 255) // Light grey
        }
    }
}

impl CanvasRenderer for TestImageRenderer {
    type Coord = f64;

    fn natural_bounds(&self) -> ImageRect<f64> {
        ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0))
    }

    fn render(&self, target_rect: &ImageRect<f64>, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for py in 0..height {
            for px in 0..width {
                // Map pixel to image coordinates
                let img_x = target_rect.min.x()
                    + (px as f64 / width as f64) * (target_rect.max.x() - target_rect.min.x());
                let img_y = target_rect.min.y()
                    + (py as f64 / height as f64) * (target_rect.max.y() - target_rect.min.y());

                let color = self.compute_pixel_color(img_x, img_y);

                let idx = ((py * width + px) * 4) as usize;
                pixels[idx] = color.0; // R
                pixels[idx + 1] = color.1; // G
                pixels[idx + 2] = color.2; // B
                pixels[idx + 3] = color.3; // A
            }
        }

        pixels
    }
}

#[component]
pub fn TestImageView() -> impl IntoView {
    let renderer = TestImageRenderer::new();

    // Initialize viewport - center at (0,0), zoom 1.0 shows full natural bounds
    let (viewport, _set_viewport) = create_signal(Viewport {
        center: ImageCoord::new(0.0, 0.0),
        zoom: 1.0,
        natural_bounds: renderer.natural_bounds(),
    });

    view! { <Canvas renderer=renderer viewport=viewport /> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_natural_bounds() {
        let renderer = TestImageRenderer::new();
        let bounds = renderer.natural_bounds();
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_renderer_produces_correct_pixel_count() {
        let renderer = TestImageRenderer::new();
        let bounds = ImageRect::new(ImageCoord::new(-10.0, -10.0), ImageCoord::new(10.0, 10.0));
        let pixels = renderer.render(&bounds, 100, 100);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_checkerboard_pattern_at_origin() {
        let renderer = TestImageRenderer::new();

        // Point at (-5, -5) should be in one square
        let color1 = renderer.compute_pixel_color(-5.0, -5.0);
        // Point at (5, 5) should be in same color (both negative square indices sum to even)
        let color2 = renderer.compute_pixel_color(5.0, 5.0);
        // Point at (5, -5) should be opposite color
        let color3 = renderer.compute_pixel_color(5.0, -5.0);

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_circle_at_radius_10() {
        let renderer = TestImageRenderer::new();

        // Point exactly on circle (radius 10)
        let color_on = renderer.compute_pixel_color(10.0, 0.0);
        assert_eq!(color_on, (255, 0, 0, 255)); // Red

        // Point between circles
        let color_off = renderer.compute_pixel_color(15.0, 0.0);
        assert_ne!(color_off, (255, 0, 0, 255)); // Not red
    }

    #[test]
    fn test_origin_is_corner_of_four_squares() {
        let renderer = TestImageRenderer::new();

        // (0,0) is corner, so nearby points in different quadrants have different colors
        let q1 = renderer.compute_pixel_color(1.0, 1.0);
        let q2 = renderer.compute_pixel_color(-1.0, 1.0);
        let q3 = renderer.compute_pixel_color(-1.0, -1.0);
        let q4 = renderer.compute_pixel_color(1.0, -1.0);

        // Opposite quadrants should have same color
        assert_eq!(q1, q3);
        assert_eq!(q2, q4);
        assert_ne!(q1, q2);
    }
}
