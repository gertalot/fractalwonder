use crate::rendering::{CanvasRenderer, Colorizer};
use fractalwonder_compute::Renderer;
use fractalwonder_core::{PixelRect, Rect, Viewport};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Rendering state for async tile processing
struct RenderState<S, D: Clone> {
    viewport: Viewport<S>,
    canvas_size: (u32, u32),
    remaining_tiles: Vec<PixelRect>,
    computed_data: Vec<D>,
    render_id: u32,
    total_tiles: usize,
    start_time: f64,
}

/// Cached state between renders
struct CachedState<S, D: Clone> {
    viewport: Option<Viewport<S>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<D>,
    render_id: AtomicU32,
}

impl<S, D: Clone> Default for CachedState<S, D> {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
            render_id: AtomicU32::new(0),
        }
    }
}

/// Async progressive canvas renderer - yields between tiles
pub struct AsyncProgressiveRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
    current_render: Rc<RefCell<Option<RenderState<S, D>>>>,
}

impl<S, D: Clone> Clone for AsyncProgressiveRenderer<S, D> {
    fn clone(&self) -> Self {
        Self {
            renderer: dyn_clone::clone_box(&*self.renderer),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state),
            current_render: Rc::clone(&self.current_render),
        }
    }
}

impl<S: Clone + PartialEq, D: Clone + Default + 'static> AsyncProgressiveRenderer<S, D> {
    pub fn new(
        renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
        colorizer: Colorizer<D>,
        tile_size: u32,
    ) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
            current_render: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
        self.renderer = renderer;
        self.clear_cache();
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
        self.colorizer = colorizer;
        // Cache preserved - no recomputation needed
    }

    pub fn natural_bounds(&self) -> Rect<S> {
        self.renderer.natural_bounds()
    }

    pub fn cancel_render(&self) {
        // Cancel in-progress async render
        let cache = self.cached_state.lock().unwrap();
        cache.render_id.fetch_add(1, Ordering::SeqCst);
        drop(cache);

        // Clear current render state
        *self.current_render.borrow_mut() = None;
    }

    fn clear_cache(&mut self) {
        let mut cache = self.cached_state.lock().unwrap();
        cache.viewport = None;
        cache.canvas_size = None;
        cache.data.clear();
    }

    fn colorize_and_display_tile(&self, data: &[D], rect: PixelRect, canvas: &HtmlCanvasElement) {
        use wasm_bindgen::Clamped;
        use web_sys::{CanvasRenderingContext2d, ImageData};

        // Verify data length
        let expected_pixels = (rect.width * rect.height) as usize;
        if data.len() != expected_pixels {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Tile data mismatch: {} pixels, expected {}",
                data.len(),
                expected_pixels
            )));
            return;
        }

        // Convert data to RGBA pixels
        let pixels: Vec<u8> = data
            .iter()
            .flat_map(|d| {
                let (r, g, b, a) = (self.colorizer)(d);
                [r, g, b, a]
            })
            .collect();

        // Get 2D context
        let context = canvas
            .get_context("2d")
            .expect("Failed to get 2d context")
            .expect("2d context is None")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("Failed to cast to 2D context");

        // Create ImageData
        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&pixels), rect.width, rect.height)
                .expect("Failed to create ImageData");

        // Put on canvas at tile position
        context
            .put_image_data(&image_data, rect.x as f64, rect.y as f64)
            .expect("Failed to put image data");
    }

    fn render_next_tile_async(&self, canvas: HtmlCanvasElement)
    where
        S: Clone + 'static,
    {
        // Clone Rc for closure
        let current_render = Rc::clone(&self.current_render);
        let cached_state = Arc::clone(&self.cached_state);
        let renderer = dyn_clone::clone_box(&*self.renderer);
        let self_clone = self.clone();

        // Get current render state
        let mut render_state = current_render.borrow_mut();
        let state = match render_state.as_mut() {
            Some(s) => s,
            None => {
                // No active render
                return;
            }
        };

        // Check if cancelled
        let cache = cached_state.lock().unwrap();
        let current_render_id = cache.render_id.load(Ordering::SeqCst);
        drop(cache);

        if current_render_id != state.render_id {
            // Render cancelled
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Render {} cancelled",
                state.render_id
            )));
            *render_state = None;
            return;
        }

        // Get next tile
        let tile_rect = match state.remaining_tiles.pop() {
            Some(tile) => tile,
            None => {
                // All tiles complete - finalize render
                #[cfg(target_arch = "wasm32")]
                {
                    let elapsed = web_sys::window()
                        .and_then(|w| w.performance())
                        .map(|p| p.now() - state.start_time)
                        .unwrap_or(0.0);

                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                        "Render {} complete - {} tiles in {:.2}ms ({:.2}ms/tile avg)",
                        state.render_id,
                        state.total_tiles,
                        elapsed,
                        elapsed / state.total_tiles as f64
                    )));
                }

                // Update cache
                let mut cache = cached_state.lock().unwrap();
                cache.viewport = Some(state.viewport.clone());
                cache.canvas_size = Some(state.canvas_size);
                cache.data = state.computed_data.clone();

                *render_state = None;
                return;
            }
        };

        let viewport = state.viewport.clone();
        let canvas_size = state.canvas_size;

        // Drop mutable borrow before calling renderer
        drop(render_state);

        // Compute tile (synchronous computation, but async scheduling)
        let tile_data = renderer.render(&viewport, tile_rect, canvas_size);

        // Store in cache
        let mut render_state = current_render.borrow_mut();
        if let Some(state) = render_state.as_mut() {
            // Store tile data in raster order
            let width = state.canvas_size.0;
            let mut tile_data_idx = 0;
            for local_y in 0..tile_rect.height {
                let canvas_y = tile_rect.y + local_y;
                for local_x in 0..tile_rect.width {
                    let canvas_x = tile_rect.x + local_x;
                    let cache_idx = (canvas_y * width + canvas_x) as usize;
                    state.computed_data[cache_idx] = tile_data[tile_data_idx].clone();
                    tile_data_idx += 1;
                }
            }
        }
        drop(render_state);

        // Display tile immediately
        self_clone.colorize_and_display_tile(&tile_data, tile_rect, &canvas);

        // Schedule next tile via requestAnimationFrame
        let window = web_sys::window().expect("no global window");

        let closure = Closure::once(move || {
            self_clone.render_next_tile_async(canvas);
        });

        window
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .expect("requestAnimationFrame failed");

        closure.forget(); // Keep closure alive
    }

    pub fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement)
    where
        S: Clone + 'static,
    {
        let width = canvas.width();
        let height = canvas.height();
        let cache = self.cached_state.lock().unwrap();

        // Increment render ID to cancel any in-progress renders
        let current_render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

        // Decision: compute vs recolorize
        if cache.viewport.as_ref() == Some(viewport) && cache.canvas_size == Some((width, height)) {
            // Same viewport/size → recolorize from cache (synchronous)
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "RECOLORIZE from cache (render_id: {})",
                current_render_id
            )));

            let expected_pixels = (width * height) as usize;
            if cache.data.len() == expected_pixels {
                let data = cache.data.clone();
                drop(cache); // Release lock before rendering
                let full_rect = PixelRect::full_canvas(width, height);
                self.colorize_and_display_tile(&data, full_rect, canvas);
            } else {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "Cache size mismatch: {} pixels, expected {}",
                    cache.data.len(),
                    expected_pixels
                )));
            }
        } else {
            // Viewport/size changed → async recompute
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "ASYNC RECOMPUTE (render_id: {})",
                current_render_id
            )));

            drop(cache); // Release lock
            self.start_async_render(viewport.clone(), canvas.clone(), current_render_id);
        }
    }

    fn start_async_render(&self, viewport: Viewport<S>, canvas: HtmlCanvasElement, render_id: u32)
    where
        S: Clone + 'static,
    {
        let width = canvas.width();
        let height = canvas.height();

        // Compute all tiles up front
        let tiles = compute_tiles(width, height, self.tile_size);
        let total_tiles = tiles.len();

        // Capture start time for performance tracking
        #[cfg(target_arch = "wasm32")]
        let start_time = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        #[cfg(not(target_arch = "wasm32"))]
        let start_time = 0.0;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Starting async render: {} tiles ({}x{} canvas, {} tile_size)",
            total_tiles, width, height, self.tile_size
        )));

        // Initialize render state
        let render_state = RenderState {
            viewport,
            canvas_size: (width, height),
            remaining_tiles: tiles,
            computed_data: vec![D::default(); (width * height) as usize],
            render_id,
            total_tiles,
            start_time,
        };

        *self.current_render.borrow_mut() = Some(render_state);

        // Kick off first tile
        self.render_next_tile_async(canvas);
    }
}

/// Compute tiles for given canvas dimensions and tile size
fn compute_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let x = x_start;
            let y = y_start;
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);

            tiles.push(PixelRect::new(x, y, w, h));
        }
    }

    tiles
}

impl<S: Clone + PartialEq + 'static, D: Clone + Default + 'static> CanvasRenderer
    for AsyncProgressiveRenderer<S, D>
{
    type Scalar = S;
    type Data = D;

    fn set_renderer(
        &mut self,
        renderer: Box<dyn Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    ) {
        self.set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        self.set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        self.render(viewport, canvas);
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        self.natural_bounds()
    }

    fn cancel_render(&self) {
        self.cancel_render();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_compute::{AppDataRenderer, MandelbrotComputer, PixelRenderer};
    use fractalwonder_core::AppData;

    fn test_colorizer(_data: &AppData) -> (u8, u8, u8, u8) {
        (255, 0, 0, 255) // Red
    }

    #[test]
    fn test_async_renderer_creation() {
        // Create a simple f64-based renderer wrapped to produce AppData
        let computer = MandelbrotComputer::<f64>::default();
        let pixel_renderer = PixelRenderer::new(computer);
        let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
        let renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>> = Box::new(app_renderer);

        let async_renderer = AsyncProgressiveRenderer::new(renderer, test_colorizer, 256);

        assert_eq!(async_renderer.tile_size, 256);
    }

    #[test]
    fn test_cancel_render() {
        // Create a simple f64-based renderer wrapped to produce AppData
        let computer = MandelbrotComputer::<f64>::default();
        let pixel_renderer = PixelRenderer::new(computer);
        let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
        let renderer: Box<dyn Renderer<Scalar = f64, Data = AppData>> = Box::new(app_renderer);

        let async_renderer = AsyncProgressiveRenderer::new(renderer, test_colorizer, 256);

        // Cancel should increment render_id
        let cache = async_renderer.cached_state.lock().unwrap();
        let initial_id = cache.render_id.load(Ordering::SeqCst);
        drop(cache);

        async_renderer.cancel_render();

        let cache = async_renderer.cached_state.lock().unwrap();
        let new_id = cache.render_id.load(Ordering::SeqCst);
        assert_eq!(new_id, initial_id + 1);
    }

    #[test]
    fn test_compute_tiles() {
        // 512x512 canvas with 256 tile size → 4 tiles
        let tiles = compute_tiles(512, 512, 256);
        assert_eq!(tiles.len(), 4);

        // Verify tile positions
        assert_eq!(tiles[0], PixelRect::new(0, 0, 256, 256));
        assert_eq!(tiles[1], PixelRect::new(256, 0, 256, 256));
        assert_eq!(tiles[2], PixelRect::new(0, 256, 256, 256));
        assert_eq!(tiles[3], PixelRect::new(256, 256, 256, 256));
    }

    #[test]
    fn test_compute_tiles_partial() {
        // 300x200 with 256 tile size → edge tiles are smaller
        let tiles = compute_tiles(300, 200, 256);
        assert_eq!(tiles.len(), 2);

        assert_eq!(tiles[0], PixelRect::new(0, 0, 256, 200));
        assert_eq!(tiles[1], PixelRect::new(256, 0, 44, 200)); // Width = 300 - 256 = 44
    }
}
