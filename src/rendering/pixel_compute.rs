use crate::rendering::coords::{Coord, Rect};

/// Trait for computing individual pixel colors from image coordinates
///
/// This is the lowest-level rendering abstraction - pure computation with no loops.
/// Typically wrapped by PixelRenderer which adds the pixel iteration logic.
pub trait PixelCompute {
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
    fn compute(&self, coord: Coord<Self::Coord>) -> (u8, u8, u8, u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test implementation
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl PixelCompute for SolidColorCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Coord<f64>) -> (u8, u8, u8, u8) {
            self.color
        }
    }

    #[test]
    fn test_pixel_compute_trait() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let result = computer.compute(Coord::new(50.0, 50.0));
        assert_eq!(result, (255, 0, 0, 255));
    }
}
