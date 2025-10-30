use crate::rendering::numeric::ToF64;
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
pub struct MandelbrotComputer<T = f64> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> MandelbrotComputer<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> ImagePointComputer for MandelbrotComputer<T>
where
    T: Clone
        + From<f64>
        + ToF64
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Div<Output = T>
        + PartialOrd,
{
    type Scalar = T;
    type Data = MandelbrotData;

    fn natural_bounds(&self) -> Rect<T> {
        // Standard Mandelbrot viewing window: centered at origin, spans [-2.5, 1.0] x [-1.25, 1.25]
        Rect::new(
            Point::new(T::from(-2.5), T::from(-1.25)),
            Point::new(T::from(1.0), T::from(1.25)),
        )
    }

    fn compute(&self, point: Point<T>, viewport: &Viewport<T>) -> MandelbrotData {
        let cx = point.x().clone();
        let cy = point.y().clone();

        let max_iterations = calculate_max_iterations(viewport.zoom);

        let mut zx = T::from(0.0);
        let mut zy = T::from(0.0);

        let escape_radius_sq = T::from(4.0);
        let two = T::from(2.0);

        for i in 0..max_iterations {
            let zx_sq = zx.clone() * zx.clone();
            let zy_sq = zy.clone() * zy.clone();

            let magnitude_sq = zx_sq.clone() + zy_sq.clone();
            if magnitude_sq > escape_radius_sq {
                return MandelbrotData {
                    iterations: i,
                    escaped: true,
                };
            }

            let new_zx = zx_sq - zy_sq + cx.clone();
            let new_zy = two.clone() * zx.clone() * zy.clone() + cy.clone();

            zx = new_zx;
            zy = new_zy;
        }

        MandelbrotData {
            iterations: max_iterations,
            escaped: false,
        }
    }
}

impl<T> RendererInfo for MandelbrotComputer<T>
where
    T: Clone + From<f64> + ToF64,
{
    type Scalar = T;

    fn info(&self, viewport: &Viewport<T>) -> RendererInfoData {
        let max_iterations = calculate_max_iterations(viewport.zoom);
        RendererInfoData {
            name: "Mandelbrot (Arbitrary Precision)".to_string(),
            center_display: format!(
                "{:.6}, {:.6}",
                viewport.center.x().to_f64(),
                viewport.center.y().to_f64()
            ),
            zoom_display: format!("{:.2e}", viewport.zoom),
            custom_params: vec![("Max Iterations".to_string(), max_iterations.to_string())],
            render_time_ms: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::BigFloat;

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

    #[test]
    fn test_mandelbrot_with_bigfloat() {
        // Test that Mandelbrot works with arbitrary precision
        let computer: MandelbrotComputer<BigFloat> = MandelbrotComputer::new();
        let point = Point::new(BigFloat::from_f64(0.0), BigFloat::from_f64(0.0));
        let viewport = Viewport::new(
            Point::new(BigFloat::from_f64(0.0), BigFloat::from_f64(0.0)),
            1.0,
        );
        let result = computer.compute(point, &viewport);
        assert!(!result.escaped);
        assert_eq!(result.iterations, calculate_max_iterations(1.0));
    }

    #[test]
    fn test_mandelbrot_bigfloat_outside_set() {
        let computer: MandelbrotComputer<BigFloat> = MandelbrotComputer::new();
        let point = Point::new(BigFloat::from_f64(2.0), BigFloat::from_f64(2.0));
        let viewport = Viewport::new(
            Point::new(BigFloat::from_f64(0.0), BigFloat::from_f64(0.0)),
            1.0,
        );
        let result = computer.compute(point, &viewport);
        assert!(result.escaped);
        assert!(result.iterations < calculate_max_iterations(1.0));
    }

    #[test]
    fn test_mandelbrot_bigfloat_boundary_point() {
        // Test a point on the boundary of the Mandelbrot set
        // Point (-0.75, 0.1) is close to the main cardioid
        let computer: MandelbrotComputer<BigFloat> = MandelbrotComputer::new();
        let point = Point::new(BigFloat::from_f64(-0.75), BigFloat::from_f64(0.1));
        let viewport = Viewport::new(
            Point::new(BigFloat::from_f64(0.0), BigFloat::from_f64(0.0)),
            1.0,
        );
        let result = computer.compute(point, &viewport);

        // This point should escape
        assert!(result.escaped);
    }
}
