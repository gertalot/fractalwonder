use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::{RenderWorkerPool, TileResult};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Point, Rect, ToF64, Viewport};
use leptos::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[derive(Default)]
struct CachedState {
    viewport: Option<Viewport<BigFloat>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<AppData>,
}

pub(crate) struct TileRequest {
    pub tile: PixelRect,
}

pub struct ParallelCanvasRenderer {
    worker_pool: Rc<RefCell<RenderWorkerPool>>,
    colorizer: Rc<RefCell<Colorizer<AppData>>>,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
    render_id: Arc<AtomicU32>,
    progress: RwSignal<crate::rendering::RenderProgress>,
    natural_bounds: Rect<f64>,
}

impl ParallelCanvasRenderer {
    pub fn new(colorizer: Colorizer<AppData>, renderer_id: String) -> Result<Self, JsValue> {
        let canvas: Rc<RefCell<Option<HtmlCanvasElement>>> = Rc::new(RefCell::new(None));
        let canvas_clone = Rc::clone(&canvas);
        let colorizer = Rc::new(RefCell::new(colorizer));
        let colorizer_clone = Rc::clone(&colorizer);
        let cached_state = Arc::new(Mutex::new(CachedState::default()));
        let cached_state_clone = Arc::clone(&cached_state);

        // Create progress signal
        let progress = create_rw_signal(crate::rendering::RenderProgress::default());

        let on_tile_complete = move |tile_result: TileResult| {
            if let Some(canvas) = canvas_clone.borrow().as_ref() {
                let mut cache = cached_state_clone.lock().unwrap();

                store_tile_in_cache(&mut cache.data, canvas.width(), &tile_result);

                drop(cache);

                // Draw tile immediately (progressive rendering)
                let colorizer = colorizer_clone.borrow();
                if let Err(e) = draw_colorized_pixels(
                    canvas,
                    &tile_result.data,
                    tile_result.tile.width,
                    tile_result.tile.x as f64,
                    tile_result.tile.y as f64,
                    &colorizer,
                ) {
                    web_sys::console::error_1(&e);
                }
            }
        };

        let worker_pool = RenderWorkerPool::new(on_tile_complete, progress, renderer_id.clone())?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer created with {} workers",
            worker_pool.borrow().worker_count(),
        )));

        // Get natural bounds from the renderer config
        let config = fractalwonder_compute::get_config(&renderer_id)
            .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))?;
        let renderer = (config.create_renderer)();
        let natural_bounds_bigfloat = renderer.natural_bounds();
        let natural_bounds = Rect::new(
            Point::new(
                natural_bounds_bigfloat.min.x().to_f64(),
                natural_bounds_bigfloat.min.y().to_f64(),
            ),
            Point::new(
                natural_bounds_bigfloat.max.x().to_f64(),
                natural_bounds_bigfloat.max.y().to_f64(),
            ),
        );

        let render_id = Arc::new(AtomicU32::new(0));

        Ok(Self {
            worker_pool,
            colorizer,
            canvas,
            cached_state,
            render_id,
            progress,
            natural_bounds,
        })
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.borrow().worker_count()
    }

    pub fn switch_renderer(&self, renderer_id: String) {
        self.worker_pool.borrow_mut().switch_renderer(renderer_id);
    }
}

impl Clone for ParallelCanvasRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: Rc::clone(&self.colorizer),
            canvas: Rc::clone(&self.canvas),
            cached_state: Arc::clone(&self.cached_state),
            render_id: Arc::clone(&self.render_id),
            progress: self.progress,
            natural_bounds: self.natural_bounds.clone(),
        }
    }
}

impl CanvasRenderer for ParallelCanvasRenderer {
    type Scalar = f64;
    type Data = AppData;

    fn set_renderer(
        &mut self,
        _renderer: Box<
            dyn fractalwonder_compute::Renderer<Scalar = Self::Scalar, Data = Self::Data>,
        >,
    ) {
        // Not used - workers have their own AdaptiveMandelbrotRenderer
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        *self.colorizer.borrow_mut() = colorizer;
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        *self.canvas.borrow_mut() = Some(canvas.clone());

        let render_id = self.render_id.fetch_add(1, Ordering::SeqCst) + 1;
        let mut cache = self.cached_state.lock().unwrap();

        // Convert f64 viewport to BigFloat
        let viewport_bf = Viewport::new(
            Point::new(
                BigFloat::from(*viewport.center.x()),
                BigFloat::from(*viewport.center.y()),
            ),
            viewport.zoom,
        );

        // if the viewport hasn't changed and the canvas hasn't resized, then we don't
        // have to recompute anything. Just push the computed data through the coloring
        // function and into the canvas imagedata.
        if cache.viewport.as_ref() == Some(&viewport_bf)
            && cache.canvas_size == Some((width, height))
        {
            // Recolorize from cache
            if self.render_id.load(Ordering::SeqCst) == render_id {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "DRAW from cache (render_id: {})",
                    render_id
                )));
                let colorizer = self.colorizer.borrow();
                if let Err(e) =
                    draw_colorized_pixels(canvas, &cache.data, canvas.width(), 0.0, 0.0, &colorizer)
                {
                    web_sys::console::error_1(&e);
                }
            }
            drop(cache);
        } else {
            // Recompute
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "RECOMPUTE (render_id: {}, {}x{})",
                render_id, width, height
            )));

            cache.data.clear();
            cache
                .data
                .resize((width * height) as usize, AppData::default());
            cache.viewport = Some(viewport_bf.clone());
            cache.canvas_size = Some((width, height));
            drop(cache);

            // Calculate tile size based on zoom level
            let tile_size = calculate_tile_size(viewport.zoom);

            // Generate tiles using rendering strategy (tile size, center-out ordering)
            let tiles = generate_tiles(width, height, tile_size);

            self.worker_pool.borrow_mut().start_render(
                viewport_bf,
                width,
                height,
                tiles,
                render_id,
            );
        }
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        self.natural_bounds.clone()
    }

    fn cancel_render(&self) {
        self.worker_pool.borrow_mut().cancel_current_render();
    }

    fn progress(&self) -> RwSignal<crate::rendering::RenderProgress> {
        self.progress
    }
}

/// Calculate appropriate tile size based on zoom level
///
/// At extreme zoom levels, we use smaller tiles for more frequent
/// progressive rendering updates during long renders.
fn calculate_tile_size(zoom: f64) -> u32 {
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;
    const NORMAL_TILE_SIZE: u32 = 128;
    const DEEP_ZOOM_TILE_SIZE: u32 = 64;

    if zoom >= DEEP_ZOOM_THRESHOLD {
        DEEP_ZOOM_TILE_SIZE
    } else {
        NORMAL_TILE_SIZE
    }
}

/// Store tile data in the full canvas cache by mapping tile-local coordinates to canvas-global coordinates
///
/// Converts from tile's local coordinate system (0,0 to tile.width-1, tile.height-1) to
/// canvas global coordinates, then stores each pixel in the cache using 1D raster-scan indexing.
fn store_tile_in_cache(cache_data: &mut [AppData], canvas_width: u32, tile_result: &TileResult) {
    for local_y in 0..tile_result.tile.height {
        for local_x in 0..tile_result.tile.width {
            let canvas_x = tile_result.tile.x + local_x;
            let canvas_y = tile_result.tile.y + local_y;
            let cache_idx = (canvas_y * canvas_width + canvas_x) as usize;
            let tile_idx = (local_y * tile_result.tile.width + local_x) as usize;

            if cache_idx < cache_data.len() && tile_idx < tile_result.data.len() {
                cache_data[cache_idx] = tile_result.data[tile_idx].clone();
            }
        }
    }
}

/// Draw AppData pixels to canvas using the provided colorizer
/// This is the ONLY place where pixels are pushed into the canvas imagedata
///
/// The `width` parameter determines how pixels wrap into rows for ImageData.
/// The `x` and `y` parameters specify the canvas position to draw at.
fn draw_colorized_pixels(
    canvas: &HtmlCanvasElement,
    pixels: &[AppData],
    width: u32,
    x: f64,
    y: f64,
    colorizer: &Colorizer<AppData>,
) -> Result<(), JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("No 2d context"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    let colors: Vec<u8> = pixels
        .iter()
        .flat_map(|data| {
            let (r, g, b, a) = colorizer(data);
            [r, g, b, a]
        })
        .collect();

    let image_data =
        web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&colors), width)?;

    context.put_image_data(&image_data, x, y)?;

    Ok(())
}

/// Generate tiles for progressive rendering, sorted by distance from canvas center
///
/// Creates a grid of tiles covering the canvas, with each tile at most `tile_size` pixels.
/// Tiles are sorted by distance from center to render the most visible area first.
fn generate_tiles(width: u32, height: u32, tile_size: u32) -> VecDeque<TileRequest> {
    let mut tiles = Vec::new();

    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let x = x_start;
            let y = y_start;
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);

            tiles.push(TileRequest {
                tile: PixelRect::new(x, y, w, h),
            });
        }
    }

    // Sort by distance from center
    let canvas_center_x = width as f64 / 2.0;
    let canvas_center_y = height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let a_center_x = a.tile.x as f64 + a.tile.width as f64 / 2.0;
        let a_center_y = a.tile.y as f64 + a.tile.height as f64 / 2.0;
        let a_dist_sq =
            (a_center_x - canvas_center_x).powi(2) + (a_center_y - canvas_center_y).powi(2);

        let b_center_x = b.tile.x as f64 + b.tile.width as f64 / 2.0;
        let b_center_y = b.tile.y as f64 + b.tile.height as f64 / 2.0;
        let b_dist_sq =
            (b_center_x - canvas_center_x).powi(2) + (b_center_y - canvas_center_y).powi(2);

        a_dist_sq
            .partial_cmp(&b_dist_sq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_zoom_uses_128px_tiles() {
        assert_eq!(calculate_tile_size(1.0), 128);
        assert_eq!(calculate_tile_size(100.0), 128);
        assert_eq!(calculate_tile_size(1e9), 128);
        assert_eq!(calculate_tile_size(9.9e9), 128);
    }

    #[test]
    fn test_deep_zoom_uses_64px_tiles() {
        assert_eq!(calculate_tile_size(1e10), 64);
        assert_eq!(calculate_tile_size(1e11), 64);
        assert_eq!(calculate_tile_size(1e50), 64);
        assert_eq!(calculate_tile_size(1e100), 64);
    }

    #[test]
    fn test_threshold_boundary() {
        // Just below threshold
        assert_eq!(calculate_tile_size(1e10 - 1.0), 128);
        // At threshold
        assert_eq!(calculate_tile_size(1e10), 64);
        // Just above threshold
        assert_eq!(calculate_tile_size(1e10 + 1.0), 64);
    }
}
