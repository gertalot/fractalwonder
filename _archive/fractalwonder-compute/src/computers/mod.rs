pub mod mandelbrot;
pub mod test_image;

pub use mandelbrot::MandelbrotComputer;
pub use test_image::TestImageComputer;
// Re-export MandelbrotData from core for convenience
pub use fractalwonder_core::MandelbrotData;
