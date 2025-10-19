use crate::components::InteractiveCanvas;
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
    PixelRenderer,
};
use leptos::*;

#[derive(Clone)]
pub struct TestImageRenderer {
    checkerboard_size: f64,
    circle_radius_step: f64,
    circle_line_thickness: f64,
}

impl TestImageRenderer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 5.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.1,
        }
    }

    fn compute_point_color(&self, x: f64, y: f64) -> (u8, u8, u8, u8) {
        // Draw bright green vertical line through the center (x=0)
        if x.abs() < self.circle_line_thickness {
            return (0, 255, 0, 255); // Bright green
        }

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

impl ImagePointComputer for TestImageRenderer {
    type Coord = f64;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0))
    }

    fn compute(&self, coord: Point<f64>) -> (u8, u8, u8, u8) {
        self.compute_point_color(*coord.x(), *coord.y())
    }
}

impl RendererInfo for TestImageRenderer {
    type Coord = f64;

    fn info(&self, viewport: &Viewport<f64>) -> RendererInfoData {
        RendererInfoData {
            name: "Test Image".to_string(),
            center_display: format!("x: {:.2}, y: {:.2}", viewport.center.x(), viewport.center.y()),
            zoom_display: format!("{:.2}x", viewport.zoom),
            custom_params: vec![
                ("Checkerboard size".to_string(), format!("{:.1}", self.checkerboard_size)),
                ("Circle radius step".to_string(), format!("{:.1}", self.circle_radius_step)),
                ("Circle line thickness".to_string(), format!("{:.2}", self.circle_line_thickness)),
            ],
            render_time_ms: None, // Filled by InteractiveCanvas
        }
    }
}

#[component]
pub fn TestImageView() -> impl IntoView {
    let renderer = PixelRenderer::new(TestImageRenderer::new());
    view! { <InteractiveCanvas renderer=renderer /> }
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
    fn test_checkerboard_pattern_at_origin() {
        let renderer = TestImageRenderer::new();

        // Point at (-5, -5) should be in one square
        let color1 = renderer.compute_point_color(-5.0, -5.0);
        // Point at (5, 5) should be in same color (both negative square indices sum to even)
        let color2 = renderer.compute_point_color(5.0, 5.0);
        // Point at (5, -5) should be opposite color
        let color3 = renderer.compute_point_color(5.0, -5.0);

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_circle_at_radius_10() {
        let renderer = TestImageRenderer::new();

        // Point exactly on circle (radius 10)
        let color_on = renderer.compute_point_color(10.0, 0.0);
        assert_eq!(color_on, (255, 0, 0, 255)); // Red

        // Point between circles
        let color_off = renderer.compute_point_color(15.0, 0.0);
        assert_ne!(color_off, (255, 0, 0, 255)); // Not red
    }

    #[test]
    fn test_origin_is_corner_of_four_squares() {
        let renderer = TestImageRenderer::new();

        // (0,0) is corner, so nearby points in different quadrants have different colors
        let q1 = renderer.compute_point_color(1.0, 1.0);
        let q2 = renderer.compute_point_color(-1.0, 1.0);
        let q3 = renderer.compute_point_color(-1.0, -1.0);
        let q4 = renderer.compute_point_color(1.0, -1.0);

        // Opposite quadrants should have same color
        assert_eq!(q1, q3);
        assert_eq!(q2, q4);
        assert_ne!(q1, q2);
    }
}
