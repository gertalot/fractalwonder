use crate::config::FractalConfig;
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::canvas_utils::{draw_pixels_to_canvas, get_2d_context, performance_now, yield_to_browser};
use crate::rendering::RenderProgress;
use fractalwonder_compute::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{calculate_max_iterations, BigFloat, ComputeData, PixelRect, Viewport};
use leptos::*;
use std::cell::Cell;
use std::rc::Rc;
use web_sys::HtmlCanvasElement;

/// Async progressive renderer that yields to browser between tiles.
///
/// Renders tiles one-by-one, drawing each to canvas immediately and yielding
/// via requestAnimationFrame to keep UI responsive during long renders.
#[derive(Clone)]
pub struct AsyncProgressiveRenderer {
    config: &'static FractalConfig,
    progress: RwSignal<RenderProgress>,
    render_id: Rc<Cell<u32>>,
}

impl AsyncProgressiveRenderer {
    /// Create a new renderer for the given fractal config.
    pub fn new(config: &'static FractalConfig) -> Self {
        Self {
            config,
            progress: create_rw_signal(RenderProgress::default()),
            render_id: Rc::new(Cell::new(0)),
        }
    }

    /// Get progress signal for UI binding.
    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }

    /// Cancel any in-progress render.
    pub fn cancel(&self) {
        // Increment render_id - the async loop checks this before each tile
        self.render_id.set(self.render_id.get().wrapping_add(1));
    }

    /// Start rendering viewport to canvas.
    ///
    /// Returns immediately - rendering happens asynchronously.
    /// Previous render is automatically cancelled.
    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Cancel any existing render and start new one
        let render_id = self.render_id.get().wrapping_add(1);
        self.render_id.set(render_id);

        // Calculate tile size (smaller at deep zoom for progress feedback)
        let reference_width = self.config.default_viewport(viewport.precision_bits()).width;
        let zoom = reference_width.to_f64() / viewport.width.to_f64();
        let tile_size = calculate_tile_size(zoom);

        // Generate tiles in center-out order
        let tiles = generate_tiles(width, height, tile_size);
        let total_tiles = tiles.len() as u32;

        // Reset progress
        self.progress.set(RenderProgress::new(total_tiles));

        // Clone what we need for async block
        let render_id_cell = self.render_id.clone();
        let progress = self.progress;
        let config_id = self.config.id;
        let vp = viewport.clone();
        let ctx = match get_2d_context(canvas) {
            Ok(ctx) => ctx,
            Err(_) => return,
        };
        let canvas_size = (width, height);
        let max_iters = calculate_max_iterations(&viewport.width, &reference_width);

        spawn_local(async move {
            let start_time = performance_now();

            for (i, tile) in tiles.iter().enumerate() {
                // Check cancellation before each tile
                if render_id_cell.get() != render_id {
                    return;
                }

                // Compute tile
                let tile_viewport = tile_to_viewport(tile, &vp, canvas_size);
                let tile_size = (tile.width, tile.height);

                let computed_data: Vec<ComputeData> = match config_id {
                    "test_image" => {
                        let renderer = TestImageRenderer;
                        renderer.render(&tile_viewport, tile_size)
                            .into_iter()
                            .map(ComputeData::TestImage)
                            .collect()
                    }
                    "mandelbrot" => {
                        let renderer = MandelbrotRenderer::new(max_iters);
                        renderer.render(&tile_viewport, tile_size)
                            .into_iter()
                            .map(ComputeData::Mandelbrot)
                            .collect()
                    }
                    _ => continue,
                };

                // Colorize
                let pixels: Vec<u8> = computed_data
                    .iter()
                    .flat_map(colorize)
                    .collect();

                // Draw to canvas
                let _ = draw_pixels_to_canvas(&ctx, &pixels, tile.width, tile.x as f64, tile.y as f64);

                // Update progress
                progress.update(|p| {
                    p.completed_tiles = (i + 1) as u32;
                    p.elapsed_ms = performance_now() - start_time;
                });

                // Yield to browser
                yield_to_browser().await;
            }

            // Mark complete
            progress.update(|p| {
                p.is_complete = true;
                p.elapsed_ms = performance_now() - start_time;
            });
        });
    }
}

/// Convert a pixel-space tile to its corresponding fractal-space viewport.
fn tile_to_viewport(tile: &PixelRect, viewport: &Viewport, canvas_size: (u32, u32)) -> Viewport {
    let (canvas_width, canvas_height) = canvas_size;
    let precision = viewport.precision_bits();

    // Calculate fractal-space dimensions per pixel
    let pixel_width = viewport.width.div(&BigFloat::with_precision(canvas_width as f64, precision));
    let pixel_height = viewport.height.div(&BigFloat::with_precision(canvas_height as f64, precision));

    // Calculate tile center in fractal space
    // Tile pixel center relative to canvas center
    let canvas_center_x = canvas_width as f64 / 2.0;
    let canvas_center_y = canvas_height as f64 / 2.0;
    let tile_center_x = tile.x as f64 + tile.width as f64 / 2.0;
    let tile_center_y = tile.y as f64 + tile.height as f64 / 2.0;

    let offset_x = tile_center_x - canvas_center_x;
    let offset_y = tile_center_y - canvas_center_y;

    // Convert pixel offset to fractal offset
    let offset_x_bf = pixel_width.mul(&BigFloat::with_precision(offset_x, precision));
    let offset_y_bf = pixel_height.mul(&BigFloat::with_precision(offset_y, precision));

    // Note: In fractal space, Y increases upward, but in pixel space Y increases downward
    // So we negate the Y offset
    let center_x = viewport.center.0.add(&offset_x_bf);
    let center_y = viewport.center.1.sub(&offset_y_bf);

    // Tile dimensions in fractal space
    let tile_width = pixel_width.mul(&BigFloat::with_precision(tile.width as f64, precision));
    let tile_height = pixel_height.mul(&BigFloat::with_precision(tile.height as f64, precision));

    Viewport::with_bigfloat(center_x, center_y, tile_width, tile_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_to_viewport_center_tile() {
        // Viewport centered at origin
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        // Center tile of a 200x200 canvas with 100x100 tiles
        let tile = PixelRect::new(50, 50, 100, 100);
        let canvas_size = (200, 200);

        let tile_vp = tile_to_viewport(&tile, &vp, canvas_size);

        // Center should be at origin (0, 0)
        assert!((tile_vp.center.0.to_f64() - 0.0).abs() < 0.001);
        assert!((tile_vp.center.1.to_f64() - 0.0).abs() < 0.001);

        // Width/height should be 2.0 (half of viewport since tile is half of canvas)
        assert!((tile_vp.width.to_f64() - 2.0).abs() < 0.001);
        assert!((tile_vp.height.to_f64() - 2.0).abs() < 0.001);
    }

    #[test]
    fn tile_to_viewport_top_left_tile() {
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        // Top-left tile
        let tile = PixelRect::new(0, 0, 100, 100);
        let canvas_size = (200, 200);

        let tile_vp = tile_to_viewport(&tile, &vp, canvas_size);

        // Center should be at (-1, 1) - left and up from origin
        // Pixel center is at (50, 50), canvas center at (100, 100)
        // Offset: (-50, -50) pixels = (-1, +1) in fractal space (Y inverted)
        assert!((tile_vp.center.0.to_f64() - (-1.0)).abs() < 0.001);
        assert!((tile_vp.center.1.to_f64() - 1.0).abs() < 0.001);
    }
}
