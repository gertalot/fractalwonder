use crate::{AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer, Renderer, TestImageComputer};
use fractalwonder_core::{AppData, BigFloat};

/// Create a renderer by ID for use by workers
pub fn create_renderer(
    renderer_id: &str,
) -> Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>> {
    match renderer_id {
        "mandelbrot" => Some(Box::new(AdaptiveMandelbrotRenderer::new(1e10))),
        "test_image" => {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
            Some(Box::new(app_renderer))
        }
        _ => None,
    }
}
