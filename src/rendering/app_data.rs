use crate::rendering::computers::mandelbrot::MandelbrotData;

/// Unified data type for all renderer implementations
///
/// Each renderer wraps its specific data type in this enum to enable
/// runtime polymorphism via trait objects.
#[derive(Clone, Debug)]
pub enum AppData {
    TestImageData(TestImageData),
    MandelbrotData(MandelbrotData),
}

impl Default for AppData {
    fn default() -> Self {
        // Default to black pixel (0 iterations, not escaped)
        AppData::MandelbrotData(MandelbrotData {
            iterations: 0,
            escaped: false,
        })
    }
}

/// Data computed by TestImageRenderer
#[derive(Clone, Copy, Debug)]
pub struct TestImageData {
    pub checkerboard: bool,
    pub circle_distance: f64,
}

impl TestImageData {
    pub fn new(checkerboard: bool, circle_distance: f64) -> Self {
        Self {
            checkerboard,
            circle_distance,
        }
    }
}
