use crate::rendering::{points::Rect, renderer_trait::Renderer, viewport::Viewport, PixelRect};

/// Renderer that splits rendering into tiles, delegating to inner renderer
///
/// This is a composable wrapper that adds tiling to any Renderer implementation.
/// Useful for parallelization, progress tracking, or memory management.
#[derive(Clone)]
pub struct TiledRenderer<R: Renderer> {
    inner: R,
    tile_size: u32,
}

impl<R: Renderer> TiledRenderer<R> {
    pub fn new(inner: R, tile_size: u32) -> Self {
        Self { inner, tile_size }
    }
}

impl<R> Renderer for TiledRenderer<R>
where
    R: Renderer,
    R::Coord: Clone,
{
    type Coord = R::Coord;
    type Data = R::Data;

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.inner.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<Self::Data> {
        let mut output = Vec::with_capacity((pixel_rect.width * pixel_rect.height) as usize);

        // Iterate over tiles within pixel_rect
        let mut tile_y = 0;
        while tile_y < pixel_rect.height {
            let mut tile_x = 0;
            while tile_x < pixel_rect.width {
                let tile_width = self.tile_size.min(pixel_rect.width - tile_x);
                let tile_height = self.tile_size.min(pixel_rect.height - tile_y);

                // Create tile rect in absolute canvas coordinates
                let tile_rect = PixelRect::new(
                    pixel_rect.x + tile_x,
                    pixel_rect.y + tile_y,
                    tile_width,
                    tile_height,
                );

                // Render this tile
                let tile_data = self.inner.render(viewport, tile_rect, canvas_size);

                // Append tile data to output
                output.extend(tile_data);

                tile_x += self.tile_size;
            }
            tile_y += self.tile_size;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::{
        pixel_renderer::PixelRenderer, point_compute::ImagePointComputer, points::Point,
    };

    #[derive(Clone, Debug, PartialEq)]
    struct ColorData {
        color: (u8, u8, u8, u8),
    }

    #[derive(Clone)]
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl ImagePointComputer for SolidColorCompute {
        type Coord = f64;
        type Data = ColorData;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Point<f64>, _viewport: &Viewport<f64>) -> ColorData {
            ColorData { color: self.color }
        }
    }

    #[test]
    fn test_tiled_renderer_produces_same_output() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let direct_renderer = PixelRenderer::new(computer.clone());
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 16);

        let viewport = Viewport::new(Point::new(50.0, 50.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(32, 32);

        let direct_data = direct_renderer.render(&viewport, pixel_rect, (32, 32));
        let tiled_data = tiled_renderer.render(&viewport, pixel_rect, (32, 32));

        assert_eq!(direct_data, tiled_data);
    }

    #[test]
    fn test_tiled_renderer_with_non_multiple_size() {
        // Test that tiling works when canvas size is not a multiple of tile_size
        let computer = SolidColorCompute {
            color: (0, 255, 0, 255),
        };
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 10);

        let viewport = Viewport::new(Point::new(50.0, 50.0), 1.0);
        let pixel_rect = PixelRect::full_canvas(27, 27); // Not divisible by 10

        let data = tiled_renderer.render(&viewport, pixel_rect, (27, 27));

        assert_eq!(data.len(), 27 * 27);
        // All data should have green color
        assert_eq!(data[0].color, (0, 255, 0, 255));
        assert!(data.iter().all(|d| d.color == (0, 255, 0, 255)));
    }
}
