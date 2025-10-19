use crate::rendering::{
    calculate_visible_bounds,
    point_compute::ImagePointComputer,
    points::Rect,
    renderer_info::{RendererInfo, RendererInfoData},
    renderer_trait::Renderer,
    transforms::pixel_to_image,
    viewport::Viewport,
    PixelRect,
};

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
    C: ImagePointComputer,
    C::Coord: Clone
        + std::ops::Sub<Output = C::Coord>
        + std::ops::Add<Output = C::Coord>
        + std::ops::Mul<f64, Output = C::Coord>
        + std::ops::Div<f64, Output = C::Coord>,
{
    type Coord = C::Coord;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.computer.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; (pixel_rect.width * pixel_rect.height * 4) as usize];

        // Calculate visible bounds from viewport once
        let visible_bounds = calculate_visible_bounds(viewport, canvas_size.0, canvas_size.1);

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

                // Compute color
                let (r, g, b, a) = self.computer.compute(image_coord);

                // Write to output
                let idx = ((local_y * pixel_rect.width + local_x) * 4) as usize;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = a;
            }
        }

        pixels
    }
}

impl<C> RendererInfo for PixelRenderer<C>
where
    C: ImagePointComputer + RendererInfo<Coord = <C as ImagePointComputer>::Coord>,
{
    type Coord = <C as ImagePointComputer>::Coord;

    fn info(&self, viewport: &Viewport<Self::Coord>) -> RendererInfoData {
        self.computer.info(viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::points::Point;

    struct TestCompute;

    impl ImagePointComputer for TestCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(-10.0, -10.0), Point::new(10.0, 10.0))
        }

        fn compute(&self, coord: Point<f64>) -> (u8, u8, u8, u8) {
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
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0, renderer.natural_bounds());
        let pixel_rect = PixelRect::full_canvas(10, 10);
        let pixels = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(pixels.len(), 10 * 10 * 4);

        // First pixel (top-left, x < 0) should be blue
        assert_eq!(&pixels[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_pixel_renderer_partial_rect() {
        let renderer = PixelRenderer::new(TestCompute);
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0, renderer.natural_bounds());
        // Render just a 5x5 tile starting at (2, 2)
        let pixel_rect = PixelRect::new(2, 2, 5, 5);
        let pixels = renderer.render(&viewport, pixel_rect, (10, 10));

        assert_eq!(pixels.len(), 5 * 5 * 4);
    }
}
