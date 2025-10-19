use crate::rendering::{coords::Rect, renderer_trait::Renderer, viewport::Viewport, PixelRect};

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

    fn natural_bounds(&self) -> Rect<Self::Coord> {
        self.inner.natural_bounds()
    }

    fn render(
        &self,
        viewport: &Viewport<Self::Coord>,
        pixel_rect: PixelRect,
        canvas_size: (u32, u32),
    ) -> Vec<u8> {
        let mut output = vec![0u8; (pixel_rect.width * pixel_rect.height * 4) as usize];

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
                let tile_pixels = self.inner.render(viewport, tile_rect, canvas_size);

                // Copy tile pixels into output buffer
                for y in 0..tile_height {
                    for x in 0..tile_width {
                        let tile_idx = ((y * tile_width + x) * 4) as usize;
                        let output_x = tile_x + x;
                        let output_y = tile_y + y;
                        let output_idx = ((output_y * pixel_rect.width + output_x) * 4) as usize;

                        output[output_idx..output_idx + 4]
                            .copy_from_slice(&tile_pixels[tile_idx..tile_idx + 4]);
                    }
                }

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
        coords::Coord, pixel_renderer::PixelRenderer, point_compute::ImagePointComputer,
    };

    #[derive(Clone)]
    struct SolidColorCompute {
        color: (u8, u8, u8, u8),
    }

    impl ImagePointComputer for SolidColorCompute {
        type Coord = f64;

        fn natural_bounds(&self) -> Rect<f64> {
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0))
        }

        fn compute(&self, _coord: Coord<f64>) -> (u8, u8, u8, u8) {
            self.color
        }
    }

    #[test]
    fn test_tiled_renderer_produces_same_output() {
        let computer = SolidColorCompute {
            color: (255, 0, 0, 255),
        };
        let direct_renderer = PixelRenderer::new(computer.clone());
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 16);

        let viewport = Viewport::new(
            Coord::new(50.0, 50.0),
            1.0,
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0)),
        );
        let pixel_rect = PixelRect::full_canvas(32, 32);

        let direct_pixels = direct_renderer.render(&viewport, pixel_rect, (32, 32));
        let tiled_pixels = tiled_renderer.render(&viewport, pixel_rect, (32, 32));

        assert_eq!(direct_pixels, tiled_pixels);
    }

    #[test]
    fn test_tiled_renderer_with_non_multiple_size() {
        // Test that tiling works when canvas size is not a multiple of tile_size
        let computer = SolidColorCompute {
            color: (0, 255, 0, 255),
        };
        let tiled_renderer = TiledRenderer::new(PixelRenderer::new(computer), 10);

        let viewport = Viewport::new(
            Coord::new(50.0, 50.0),
            1.0,
            Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 100.0)),
        );
        let pixel_rect = PixelRect::full_canvas(27, 27); // Not divisible by 10

        let pixels = tiled_renderer.render(&viewport, pixel_rect, (27, 27));

        assert_eq!(pixels.len(), 27 * 27 * 4);
        // All pixels should be green
        assert_eq!(&pixels[0..4], &[0, 255, 0, 255]);
    }
}
