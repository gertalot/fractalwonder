use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
    TestImageData,
};

#[derive(Clone)]
pub struct TestImageComputer {
    checkerboard_size: f64,
    circle_radius_step: f64,
    circle_line_thickness: f64,
}

impl Default for TestImageComputer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestImageComputer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 5.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.1,
        }
    }

    fn compute_point_data(&self, x: f64, y: f64) -> TestImageData {
        // Calculate circle distance
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        // On circle if within line thickness and not at origin
        let circle_distance =
            if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
                ring_distance
            } else {
                ring_distance + 1.0 // Definitely not on circle
            };

        // Also treat vertical green line as a circle for now
        if x.abs() < self.circle_line_thickness {
            return TestImageData::new(false, 0.0); // Mark as on circle
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor() as i32;
        let square_y = (y / self.checkerboard_size).floor() as i32;
        let is_light = (square_x + square_y) % 2 == 0;

        TestImageData::new(is_light, circle_distance)
    }
}

impl ImagePointComputer for TestImageComputer {
    type Scalar = f64;
    type Data = TestImageData;

    fn natural_bounds(&self) -> Rect<f64> {
        Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0))
    }

    fn compute(&self, coord: Point<f64>, _viewport: &Viewport<f64>) -> TestImageData {
        self.compute_point_data(*coord.x(), *coord.y())
    }
}

impl RendererInfo for TestImageComputer {
    type Scalar = f64;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_natural_bounds() {
        let computer = TestImageComputer::new();
        let bounds = computer.natural_bounds();
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_checkerboard_pattern_at_origin() {
        let computer = TestImageComputer::new();

        // Point at (-2.5, -2.5) in square (-1, -1), sum=-2 (even) -> light
        let data1 = computer.compute_point_data(-2.5, -2.5);
        // Point at (2.5, 2.5) in square (0, 0), sum=0 (even) -> light
        let data2 = computer.compute_point_data(2.5, 2.5);
        // Point at (2.5, -2.5) in square (0, -1), sum=-1 (odd) -> dark
        let data3 = computer.compute_point_data(2.5, -2.5);

        assert_eq!(data1.checkerboard, data2.checkerboard); // Both light
        assert_ne!(data1.checkerboard, data3.checkerboard); // data1 light, data3 dark
    }

    #[test]
    fn test_circle_at_radius_10() {
        let computer = TestImageComputer::new();

        // Point exactly on circle (radius 10)
        let data_on = computer.compute_point_data(10.0, 0.0);
        assert!(data_on.circle_distance < 0.1); // On circle

        // Point between circles
        let data_off = computer.compute_point_data(15.0, 0.0);
        assert!(data_off.circle_distance > 0.1); // Not on circle
    }

    #[test]
    fn test_origin_is_corner_of_four_squares() {
        let computer = TestImageComputer::new();

        // (0,0) is corner, so nearby points in different quadrants have different checkerboard
        let q1 = computer.compute_point_data(1.0, 1.0);
        let q2 = computer.compute_point_data(-1.0, 1.0);
        let q3 = computer.compute_point_data(-1.0, -1.0);
        let q4 = computer.compute_point_data(1.0, -1.0);

        // Opposite quadrants should have same checkerboard
        assert_eq!(q1.checkerboard, q3.checkerboard);
        assert_eq!(q2.checkerboard, q4.checkerboard);
        assert_ne!(q1.checkerboard, q2.checkerboard);
    }
}
