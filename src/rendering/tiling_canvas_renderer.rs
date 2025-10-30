use crate::rendering::{
    canvas_renderer::CanvasRenderer, points::Rect, renderer_trait::Renderer, viewport::Viewport,
    AppData, Colorizer, PixelRect,
};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Cached rendering state
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

/// Canvas renderer with tiling, progressive rendering, and caching
pub struct TilingCanvasRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
}

impl<S: Clone + PartialEq, D: Clone + Default> TilingCanvasRenderer<S, D> {
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
        }
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
        self.renderer = renderer;
        self.clear_cache();
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
        self.colorizer = colorizer;
        // Cache preserved!
    }

    fn clear_cache(&mut self) {
        let mut cache = self.cached_state.lock().unwrap();
        cache.viewport = None;
        cache.canvas_size = None;
        cache.data.clear();
    }

    pub fn natural_bounds(&self) -> Rect<S> {
        self.renderer.natural_bounds()
    }

    /// Cancel any in-progress render
    pub fn cancel_render(&self) {
        let cache = self.cached_state.lock().unwrap();
        cache.render_id.fetch_add(1, Ordering::SeqCst);
    }

    /// Main render entry point
    pub fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();
        let mut cache = self.cached_state.lock().unwrap();

        // Increment render ID to cancel any in-progress renders
        let current_render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

        // Decision: compute vs recolorize
        if cache.viewport.as_ref() == Some(viewport) && cache.canvas_size == Some((width, height)) {
            // Same viewport/size → recolorize from cache
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "RECOLORIZE from cache (render_id: {}, cached pixels: {})",
                current_render_id,
                cache.data.len()
            )));
            drop(cache); // Release lock before rendering
            self.recolorize_from_cache(current_render_id, canvas);
        } else {
            // Viewport/size changed → recompute
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "RECOMPUTE (render_id: {}, viewport_match: {}, size_match: {})",
                current_render_id,
                cache.viewport.as_ref() == Some(viewport),
                cache.canvas_size == Some((width, height))
            )));
            self.render_with_computation(viewport, canvas, &mut cache, current_render_id);
        }
    }

    fn render_with_computation(
        &self,
        viewport: &Viewport<S>,
        canvas: &HtmlCanvasElement,
        cache: &mut CachedState<S, D>,
        render_id: u32,
    ) {
        let width = canvas.width();
        let height = canvas.height();

        // Pre-allocate cache in raster order (row-by-row)
        cache.data.clear();
        cache
            .data
            .resize((width * height) as usize, D::default());

        // Progressive tiled rendering
        for tile_rect in compute_tiles(width, height, self.tile_size) {
            // Check if this render has been cancelled
            if cache.render_id.load(Ordering::SeqCst) != render_id {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "Render {} cancelled during tiling",
                    render_id
                )));
                return;
            }

            // Compute tile data
            let tile_data = self.renderer.render(viewport, tile_rect, (width, height));

            // Store tile data in cache at correct raster positions
            let mut tile_idx = 0;
            for local_y in 0..tile_rect.height {
                let canvas_y = tile_rect.y + local_y;
                for local_x in 0..tile_rect.width {
                    let canvas_x = tile_rect.x + local_x;
                    let cache_idx = (canvas_y * width + canvas_x) as usize;
                    cache.data[cache_idx] = tile_data[tile_idx].clone();
                    tile_idx += 1;
                }
            }

            // Colorize and display tile immediately (progressive!)
            self.colorize_and_display_tile(&tile_data, tile_rect, canvas);
        }

        // Update cache metadata
        cache.viewport = Some(viewport.clone());
        cache.canvas_size = Some((width, height));
    }

    fn recolorize_from_cache(&self, render_id: u32, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();
        let full_rect = PixelRect::full_canvas(width, height);

        let cache = self.cached_state.lock().unwrap();

        // Check if render was cancelled
        if cache.render_id.load(Ordering::SeqCst) != render_id {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Render {} cancelled during recolorize",
                render_id
            )));
            return;
        }

        // Verify data dimensions match before rendering
        let expected_pixels = (width * height) as usize;
        if cache.data.len() != expected_pixels {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "DIMENSION MISMATCH: cache has {} pixels but canvas expects {} ({}x{})",
                cache.data.len(),
                expected_pixels,
                width,
                height
            )));
            // Don't render with mismatched dimensions - this would cause garbled output
            return;
        }

        // Cache is in raster order, so we can recolorize the entire canvas at once
        self.colorize_and_display_tile(&cache.data, full_rect, canvas);
    }

    fn colorize_and_display_tile(
        &self,
        data: &[D],
        rect: PixelRect,
        canvas: &HtmlCanvasElement,
    ) {
        // Verify data length matches rect dimensions
        let expected_pixels = (rect.width * rect.height) as usize;
        if data.len() != expected_pixels {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "TILE DIMENSION MISMATCH: data has {} pixels but rect expects {} ({}x{} at {},{})",
                data.len(),
                expected_pixels,
                rect.width,
                rect.height,
                rect.x,
                rect.y
            )));
            return;
        }

        let rgba_bytes: Vec<u8> = data
            .iter()
            .flat_map(|d| {
                let (r, g, b, a) = (self.colorizer)(d);
                [r, g, b, a]
            })
            .collect();

        // Verify RGBA byte count
        let expected_bytes = expected_pixels * 4;
        if rgba_bytes.len() != expected_bytes {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "RGBA BYTE MISMATCH: got {} bytes but expected {} for {}x{} rect",
                rgba_bytes.len(),
                expected_bytes,
                rect.width,
                rect.height
            )));
            return;
        }

        // Get canvas context and put image data
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&rgba_bytes),
            rect.width,
            rect.height,
        )
        .unwrap();

        context
            .put_image_data(&image_data, rect.x as f64, rect.y as f64)
            .unwrap();
    }
}

impl<S: Clone + PartialEq, D: Clone + Default> CanvasRenderer for TilingCanvasRenderer<S, D> {
    type Scalar = S;
    type Data = D;

    fn set_renderer(&mut self, renderer: Box<dyn Renderer<Scalar = S, Data = D>>) {
        self.set_renderer(renderer);
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
        self.set_colorizer(colorizer);
    }

    fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
        self.render(viewport, canvas);
    }

    fn natural_bounds(&self) -> Rect<S> {
        self.natural_bounds()
    }

    fn cancel_render(&self) {
        self.cancel_render();
    }
}

/// Compute tile rectangles for progressive rendering
fn compute_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y in (0..height).step_by(tile_size as usize) {
        for x in (0..width).step_by(tile_size as usize) {
            let tile_width = tile_size.min(width - x);
            let tile_height = tile_size.min(height - y);
            tiles.push(PixelRect::new(x, y, tile_width, tile_height));
        }
    }

    tiles
}
