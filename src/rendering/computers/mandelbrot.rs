use crate::rendering::point_compute::ImagePointComputer;
use crate::rendering::points::{Point, Rect};
use crate::rendering::renderer_info::{RendererInfo, RendererInfoData};
use crate::rendering::viewport::Viewport;

#[derive(Debug, Clone, Copy)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
}

/// Calculate maximum iterations based on zoom level
///
/// Uses a logarithmic relationship: iterations = base + k * log10(zoom)^power
/// Based on research from deep zoom Mandelbrot rendering practices.
fn calculate_max_iterations(zoom: f64) -> u32 {
    let base = 50.0;
    let k = 100.0;
    let power = 1.5;

    let iterations = base + k * zoom.log10().powf(power);

    // Clamp to reasonable range
    iterations.clamp(50.0, 10000.0) as u32
}

#[derive(Debug, Clone, Default)]
pub struct MandelbrotComputer {}

impl MandelbrotComputer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ImagePointComputer for MandelbrotComputer {
    type Coord = f64;
    type Data = MandelbrotData;

    fn natural_bounds(&self) -> Rect<f64> {
        // Standard Mandelbrot viewing window: centered at origin, spans [-2.5, 1.0] x [-1.25, 1.25]
        Rect::new(Point::new(-2.5, -1.25), Point::new(1.0, 1.25))
    }

    fn compute(&self, point: Point<f64>, viewport: &Viewport<f64>) -> MandelbrotData {
        let cx = *point.x();
        let cy = *point.y();

        let max_iterations = calculate_max_iterations(viewport.zoom);

        let mut zx = 0.0;
        let mut zy = 0.0;

        for i in 0..max_iterations {
            let zx_sq = zx * zx;
            let zy_sq = zy * zy;

            if zx_sq + zy_sq > 4.0 {
                return MandelbrotData {
                    iterations: i,
                    escaped: true,
                };
            }

            let new_zx = zx_sq - zy_sq + cx;
            let new_zy = 2.0 * zx * zy + cy;

            zx = new_zx;
            zy = new_zy;
        }

        MandelbrotData {
            iterations: max_iterations,
            escaped: false,
        }
    }
}

impl RendererInfo for MandelbrotComputer {
    type Coord = f64;

    fn info(&self, viewport: &Viewport<f64>) -> RendererInfoData {
        let max_iterations = calculate_max_iterations(viewport.zoom);
        RendererInfoData {
            name: "Mandelbrot".to_string(),
            center_display: format!("{:.6}, {:.6}", viewport.center.x(), viewport.center.y()),
            zoom_display: format!("{:.2e}", viewport.zoom),
            custom_params: vec![("Max Iterations".to_string(), max_iterations.to_string())],
            render_time_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_point_in_set() {
        let computer = MandelbrotComputer::new();
        let point = Point::new(0.0, 0.0); // Origin is in Mandelbrot set
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let result = computer.compute(point, &viewport);
        assert!(!result.escaped);
        assert_eq!(result.iterations, calculate_max_iterations(1.0));
    }

    #[test]
    fn test_mandelbrot_point_outside_set() {
        let computer = MandelbrotComputer::new();
        let point = Point::new(2.0, 2.0); // Far outside set
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let result = computer.compute(point, &viewport);
        assert!(result.escaped);
        assert!(result.iterations < calculate_max_iterations(1.0));
    }
}
