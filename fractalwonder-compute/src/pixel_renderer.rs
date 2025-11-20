use crate::point_compute::ImagePointComputer;
use crate::renderer::{Renderer, RendererInfo, RendererInfoData};
use fractalwonder_core::{calculate_visible_bounds, pixel_to_image, PixelRect, Rect, Viewport};

/// Renderer that wraps an ImagePointComputer, adding pixel iteration logic
///
/// This is a composable wrapper that converts ImagePointComputer (single point)
/// into a full Renderer (pixel rectangle).
#[derive(Clone)]
pub struct PixelRenderer<C: ImagePointComputer> {
    computer: C,
}

impl<C: ImagePointComputer> PixelRenderer<C> {
    pub fn new(computer: C) -> Self {
        Self { computer }
    }
}

impl<C> Renderer for PixelRenderer<C>
where
    C: ImagePointComputer + Clone,
    C::Scalar: Clone
        + std::ops::Sub<Output = C::Scalar>
        + std::ops::Add<Output = C::Scalar>
        + std::ops::Mul<f64, Output = C::Scalar>
        + std::ops::Div<f64, Output = C::Scalar>,
{
    type Scalar = C::Scalar;
    type Data = C::Data; // Pass through Data from computer

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        self.computer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Scalar>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data> {
        let mut data = Vec::with_capacity((pixel_rect.width * pixel_rect.height) as usize);

        // Calculate visible bounds from viewport once
        let natural_bounds = self.computer.natural_bounds();
        let visible_bounds =
            calculate_visible_bounds(viewport, &natural_bounds, canvas_size.0, canvas_size.1);

        for local_y in 0..pixel_rect.height {
            for local_x in 0..pixel_rect.width {
                // Convert local pixel coords to absolute canvas coords
                let abs_x = pixel_rect.x + local_x;
                let abs_y = pixel_rect.y + local_y;

                // Map pixel to image coordinates
                let image_coord = pixel_to_image(
                    abs_x as f64,
                    abs_y as f64,
                    &visible_bounds,
                    canvas_size.0,
                    canvas_size.1,
                );

                // Compute data (not color!)
                let point_data = self.computer.compute(image_coord, viewport);
                data.push(point_data);
            }
        }

        data
    }
}

impl<C> RendererInfo for PixelRenderer<C>
where
    C: ImagePointComputer + RendererInfo<Scalar = <C as ImagePointComputer>::Scalar>,
{
    type Scalar = <C as ImagePointComputer>::Scalar;

    fn info(&self, viewport: &Viewport<Self::Scalar>) -> RendererInfoData {
        self.computer.info(viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::Point;

    #[derive(Clone)]
    struct TestCompute;

    impl ImagePointComputer for TestCompute {
        type Scalar = f64;
        type Data = (u8, u8, u8, u8); // For test, Data = RGBA

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(-10.0, -10.0), Point::new(10.0, 10.0))
        }

        fn compute(&self, coord: Point<f64>, _viewport: &Viewport<f64>) -> Self::Data {
            // Red if x > 0, blue otherwise
            if *coord.x() > 0.0 {
                (255, 0, 0, 255)
            } else {
                (0, 0, 255, 255)
            }
        }
    }

    #[test]
    fn test_pixel_renderer_full_canvas() {
        let renderer = PixelRenderer::new(TestCompute);
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(10, 10);
        let data = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(data.len(), 10 * 10);

        // First pixel (top-left, x < 0) should be blue
        assert_eq!(data[0], (0, 0, 255, 255));
    }

    #[test]
    fn test_pixel_renderer_partial_rect() {
        let renderer = PixelRenderer::new(TestCompute);
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        // Render just a 5x5 tile starting at (2, 2)
        let pixel_rect = PixelRect::new(2, 2, 5, 5);
        let data = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(data.len(), 5 * 5);
    }
}
