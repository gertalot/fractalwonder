use crate::rendering::points::{Point, Rect};
use crate::rendering::viewport::Viewport;

/// Trait for computing data values at points in image space
///
/// This is the lowest-level rendering abstraction - pure computation with no loops.
/// Typically wrapped by PixelRenderer which adds the pixel iteration logic.
pub trait ImagePointComputer {
    /// Scalar numeric type for image-space coordinates (f64, BigFloat, etc.)
    type Scalar;

    /// Data type output (NOT colors - will be colorized later)
    type Data: Clone;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Scalar>;

    /// Compute data for a single point in image space
    ///
    /// # Arguments
    /// * `coord` - Point in image-space coordinates
    /// * `viewport` - Current viewport (for zoom-dependent computations)
    ///
    /// # Returns
    /// Computation data (not RGBA - colorizer converts to colors)
    fn compute(&self, coord: Point<Self::Scalar>, viewport: &Viewport<Self::Scalar>) -> Self::Data;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test implementation
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl ImagePointComputer for SolidColorCompute {
        type Scalar = f64;
        type Data = (u8, u8, u8, u8); // For tests, Data = RGBA

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Point<f64>, _viewport: &Viewport<f64>) -> Self::Data {
            self.color
        }
    }

    #[test]
    fn test_image_point_computer_trait() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let result = computer.compute(Point::new(50.0, 50.0), &viewport);
        assert_eq!(result, (255, 0, 0, 255));
    }
}
