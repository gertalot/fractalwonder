use crate::renderer_trait::Renderer;
use fractalwonder_core::{Rect, Viewport, PixelRect, AppData};

/// Wrapper that converts a Renderer<Data=D> to Renderer<Data=AppData>
///
/// This enables using specific renderers (like PixelRenderer<TestImageComputer>)
/// in contexts that expect AppData.
pub struct AppDataRenderer<R, F>
where
    R: Renderer,
    F: Fn(&R::Data) -> AppData + Clone,
{
    renderer: R,
    wrap_fn: F,
}

impl<R, F> AppDataRenderer<R, F>
where
    R: Renderer,
    F: Fn(&R::Data) -> AppData + Clone,
{
    pub fn new(renderer: R, wrap_fn: F) -> Self {
        Self { renderer, wrap_fn }
    }
}

impl<R, F> Clone for AppDataRenderer<R, F>
where
    R: Renderer + Clone,
    F: Fn(&R::Data) -> AppData + Clone,
{
    fn clone(&self) -> Self {
        Self {
            renderer: self.renderer.clone(),
            wrap_fn: self.wrap_fn.clone(),
        }
    }
}

impl<R, F> Renderer for AppDataRenderer<R, F>
where
    R: Renderer + Clone,
    R::Scalar: Clone,
    F: Fn(&R::Data) -> AppData + Clone,
{
    type Scalar = R::Scalar;
    type Data = AppData;

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        self.renderer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Scalar>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<AppData> {
        let data = self.renderer.render(viewport, pixel_rect, canvas_size);
        data.iter().map(&self.wrap_fn).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_compute::ImagePointComputer;
    use crate::pixel_renderer::PixelRenderer;
    use fractalwonder_core::{TestImageData, Point};

    #[derive(Clone)]
    struct DummyComputer;

    impl ImagePointComputer for DummyComputer {
        type Scalar = f64;
        type Data = TestImageData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0))
        }

        fn compute(&self, _coord: Point<f64>, _viewport: &Viewport<f64>) -> TestImageData {
            TestImageData::new(true, 5.0)
        }
    }

    #[test]
    fn test_app_data_renderer_wraps_data() {
        let pixel_renderer = PixelRenderer::new(DummyComputer);
        let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));

        let viewport = Viewport::new(Point::new(5.0, 5.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(2, 2);
        let data = app_renderer.render(&viewport, pixel_rect, (2, 2));

        assert_eq!(data.len(), 4);
        // All wrapped in AppData::TestImageData
        matches!(data[0], AppData::TestImageData(_));
    }
}
