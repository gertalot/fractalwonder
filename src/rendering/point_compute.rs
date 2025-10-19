use crate::rendering::points::{Point, Rect};

/// Trait for computing color values at points in image space
///
/// This is the lowest-level rendering abstraction - pure computation with no loops.
/// Typically wrapped by PixelRenderer which adds the pixel iteration logic.
pub trait ImagePointComputer {
    /// Coordinate type for image space
    type Coord;

    /// Natural bounds of the image in image-space coordinates
    fn natural_bounds(&self) -> Rect<Self::Coord>;

    /// Compute RGBA color for a single point in image space
    ///
    /// # Arguments
    /// * `coord` - Point in image-space coordinates
    ///
    /// # Returns
    /// (R, G, B, A) tuple, each 0-255
    fn compute(&self, coord: Point<Self::Coord>) -> (u8, u8, u8, u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test implementation
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl ImagePointComputer for SolidColorCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Point<f64>) -> (u8, u8, u8, u8) {
            self.color
        }
    }

    #[test]
    fn test_image_point_computer_trait() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let result = computer.compute(Point::new(50.0, 50.0));
        assert_eq!(result, (255, 0, 0, 255));
    }
}
