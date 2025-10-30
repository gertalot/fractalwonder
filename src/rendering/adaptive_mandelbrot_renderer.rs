use crate::rendering::{
    computers::MandelbrotComputer, renderer_info::{RendererInfo, RendererInfoData}, AppData,
    AppDataRenderer, BigFloat, PixelRect, PixelRenderer, Point, PrecisionCalculator, Rect,
    Renderer, ToF64, Viewport,
};

/// Adaptive Mandelbrot renderer that switches between f64 and BigFloat based on zoom level.
///
/// At low zoom levels (< threshold), uses fast f64 arithmetic.
/// At high zoom levels (>= threshold), uses arbitrary precision BigFloat.
#[derive(Clone)]
pub struct AdaptiveMandelbrotRenderer {
    f64_renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>>,
    bigfloat_renderer: Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>,
    zoom_threshold: f64,
}

impl AdaptiveMandelbrotRenderer {
    /// Create a new adaptive renderer with the specified zoom threshold.
    ///
    /// # Arguments
    /// * `zoom_threshold` - Zoom level at which to switch from f64 to BigFloat (e.g., 1e10)
    pub fn new(zoom_threshold: f64) -> Self {
        // Create f64 renderer
        let computer_f64 = MandelbrotComputer::<f64>::new();
        let pixel_renderer_f64 = PixelRenderer::new(computer_f64);
        let app_renderer_f64 =
            AppDataRenderer::new(pixel_renderer_f64, |d| AppData::MandelbrotData(*d));

        // Create BigFloat renderer
        let computer_bigfloat = MandelbrotComputer::<BigFloat>::new();
        let pixel_renderer_bigfloat = PixelRenderer::new(computer_bigfloat);
        let app_renderer_bigfloat =
            AppDataRenderer::new(pixel_renderer_bigfloat, |d| AppData::MandelbrotData(*d));

        Self {
            f64_renderer: Box::new(app_renderer_f64),
            bigfloat_renderer: Box::new(app_renderer_bigfloat),
            zoom_threshold,
        }
    }

    /// Convert BigFloat viewport to f64 viewport
    fn convert_viewport_to_f64(&self, viewport: &Viewport<BigFloat>) -> Viewport<f64> {
        Viewport::new(
            Point::new(viewport.center.x().to_f64(), viewport.center.y().to_f64()),
            viewport.zoom,
        )
    }
}

impl Renderer for AdaptiveMandelbrotRenderer {
    type Scalar = BigFloat;
    type Data = AppData;

    fn natural_bounds(&self) -> Rect<BigFloat> {
        self.bigfloat_renderer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<BigFloat>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<AppData> {
        if viewport.zoom < self.zoom_threshold {
            // Use fast f64 renderer at low zoom
            let viewport_f64 = self.convert_viewport_to_f64(viewport);
            self.f64_renderer
                .render(&viewport_f64, pixel_rect, canvas_size)
        } else {
            // Use BigFloat renderer at high zoom
            self.bigfloat_renderer
                .render(viewport, pixel_rect, canvas_size)
        }
    }
}

impl RendererInfo for AdaptiveMandelbrotRenderer {
    type Scalar = BigFloat;

    fn info(&self, viewport: &Viewport<BigFloat>) -> RendererInfoData {
        if viewport.zoom < self.zoom_threshold {
            // Use f64 renderer info
            let viewport_f64 = self.convert_viewport_to_f64(viewport);
            let mut info_data = MandelbrotComputer::<f64>::new().info(&viewport_f64);
            info_data.custom_params.push((
                "Precision".to_string(),
                "f64 (fast)".to_string(),
            ));
            info_data
        } else {
            // Use BigFloat renderer info
            let mut info_data = MandelbrotComputer::<BigFloat>::new().info(viewport);
            let precision_bits = PrecisionCalculator::calculate_precision_bits(viewport.zoom);
            info_data.custom_params.push((
                "Precision".to_string(),
                format!("{} bits", precision_bits),
            ));
            info_data
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uses_f64_at_low_zoom() {
        let renderer = AdaptiveMandelbrotRenderer::new(1e10);
        let viewport = Viewport::new(
            Point::new(BigFloat::with_precision(0.0, 128), BigFloat::with_precision(0.0, 128)),
            1.0, // Low zoom
        );

        // Should use f64 renderer (we can't directly test which is used, but we can verify it works)
        let data = renderer.render(&viewport, PixelRect::new(0, 0, 10, 10), (10, 10));
        assert_eq!(data.len(), 100);
    }

    #[test]
    fn test_uses_bigfloat_at_high_zoom() {
        let renderer = AdaptiveMandelbrotRenderer::new(1e10);
        let precision_bits = PrecisionCalculator::calculate_precision_bits(1e15);
        let viewport = Viewport::new(
            Point::new(
                BigFloat::with_precision(-0.5, precision_bits),
                BigFloat::with_precision(0.0, precision_bits),
            ),
            1e15, // High zoom
        );

        // Should use BigFloat renderer
        let data = renderer.render(&viewport, PixelRect::new(0, 0, 10, 10), (10, 10));
        assert_eq!(data.len(), 100);
    }

    #[test]
    fn test_threshold_boundary() {
        let threshold = 1e10;
        let renderer = AdaptiveMandelbrotRenderer::new(threshold);

        // Just below threshold - should use f64
        let viewport_low = Viewport::new(
            Point::new(
                BigFloat::with_precision(0.0, 128),
                BigFloat::with_precision(0.0, 128),
            ),
            threshold * 0.9,
        );
        let data_low = renderer.render(&viewport_low, PixelRect::new(0, 0, 5, 5), (5, 5));
        assert_eq!(data_low.len(), 25);

        // At threshold - should use BigFloat
        let precision_bits = PrecisionCalculator::calculate_precision_bits(threshold);
        let viewport_high = Viewport::new(
            Point::new(
                BigFloat::with_precision(0.0, precision_bits),
                BigFloat::with_precision(0.0, precision_bits),
            ),
            threshold,
        );
        let data_high = renderer.render(&viewport_high, PixelRect::new(0, 0, 5, 5), (5, 5));
        assert_eq!(data_high.len(), 25);
    }

    #[test]
    fn test_natural_bounds() {
        let renderer = AdaptiveMandelbrotRenderer::new(1e10);
        let bounds = renderer.natural_bounds();

        // Should return BigFloat bounds
        assert!(bounds.min.x().to_f64() < -2.0);
        assert!(bounds.max.x().to_f64() > 0.5);
    }
}
