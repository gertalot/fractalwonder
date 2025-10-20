use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
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
            center_display: format!(
                "x: {:.2}, y: {:.2}",
                viewport.center.x(),
                viewport.center.y()
            ),
            zoom_display: format!("{:.2}x", viewport.zoom),
            custom_params: vec![
                (
                    "Checkerboard size".to_string(),
                    format!("{:.1}", self.checkerboard_size),
                ),
                (
                    "Circle radius step".to_string(),
                    format!("{:.1}", self.circle_radius_step),
                ),
            ],
            render_time_ms: None, // Filled by InteractiveCanvas
        }
    }
}

#[component]
pub fn TestImageView() -> impl IntoView {
    let renderer = PixelRenderer::new(TestImageRenderer::new());
    let canvas_with_info = InteractiveCanvas(renderer);

    // UI visibility
    let ui_visibility = use_ui_visibility();

    // Clone reset callback for use in closure
    let reset_fn = canvas_with_info.reset_viewport;
    let on_home_click = move || {
        (reset_fn)();
    };

    // Fullscreen callback
    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="w-full h-full">
            {canvas_with_info.view}
        </div>
        <UI
            info=canvas_with_info.info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
        />
    }
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

        // Point at (-2.5, -2.5) in square (-1, -1), sum=-2 (even) -> white
        let color1 = renderer.compute_point_color(-2.5, -2.5);
        // Point at (2.5, 2.5) in square (0, 0), sum=0 (even) -> white
        let color2 = renderer.compute_point_color(2.5, 2.5);
        // Point at (2.5, -2.5) in square (0, -1), sum=-1 (odd) -> grey
        let color3 = renderer.compute_point_color(2.5, -2.5);

        assert_eq!(color1, color2); // Both white (even sum)
        assert_ne!(color1, color3); // color1 white, color3 grey
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
