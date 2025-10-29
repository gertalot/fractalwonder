use crate::rendering::{
    points::Rect, renderer_trait::Renderer, viewport::Viewport, Colorizer, PixelRect,
};
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Cached rendering state
struct CachedState<R: Renderer> {
    viewport: Option<Viewport<R::Coord>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<R::Data>,
}

impl<R: Renderer> Default for CachedState<R> {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
        }
    }
}

/// Canvas renderer with tiling, progressive rendering, and caching
pub struct TilingCanvasRenderer<R: Renderer> {
    renderer: R,
    colorizer: Colorizer<R::Data>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<R>>>,
}

impl<R: Renderer> TilingCanvasRenderer<R> {
    pub fn new(renderer: R, colorizer: Colorizer<R::Data>, tile_size: u32) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
        }
    }

    /// Create new renderer with different colorizer, preserving cached data
    pub fn with_colorizer(&self, colorizer: Colorizer<R::Data>) -> Self
    where
        R: Clone,
    {
        Self {
            renderer: self.renderer.clone(),
            colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state), // Shared cache!
        }
    }

    pub fn natural_bounds(&self) -> Rect<R::Coord>
    where
        R::Coord: Clone,
    {
        self.renderer.natural_bounds()
    }

    /// Main render entry point
    pub fn render(&self, viewport: &Viewport<R::Coord>, canvas: &HtmlCanvasElement)
    where
        R::Coord: Clone + PartialEq,
    {
        let width = canvas.width();
        let height = canvas.height();
        let mut cache = self.cached_state.lock().unwrap();

        // Decision: compute vs recolorize
        if cache.viewport.as_ref() == Some(viewport) && cache.canvas_size == Some((width, height)) {
            // Same viewport/size → recolorize from cache
            self.recolorize_from_cache(&cache, canvas);
        } else {
            // Viewport/size changed → recompute
            self.render_with_computation(viewport, canvas, &mut cache);
        }
    }

    fn render_with_computation(
        &self,
        viewport: &Viewport<R::Coord>,
        canvas: &HtmlCanvasElement,
        cache: &mut CachedState<R>,
    ) where
        R::Coord: Clone,
    {
        let width = canvas.width();
        let height = canvas.height();

        cache.data.clear();
        cache.data.reserve((width * height) as usize);

        // Progressive tiled rendering
        for tile_rect in compute_tiles(width, height, self.tile_size) {
            // Compute tile data
            let tile_data = self.renderer.render(viewport, tile_rect, (width, height));

            // Store in cache
            cache.data.extend(tile_data.iter().cloned());

            // Colorize and display tile immediately (progressive!)
            self.colorize_and_display_tile(&tile_data, tile_rect, canvas);
        }

        // Update cache metadata
        cache.viewport = Some(viewport.clone());
        cache.canvas_size = Some((width, height));
    }

    fn recolorize_from_cache(&self, cache: &CachedState<R>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();
        let full_rect = PixelRect::full_canvas(width, height);

        self.colorize_and_display_tile(&cache.data, full_rect, canvas);
    }

    fn colorize_and_display_tile(
        &self,
        data: &[R::Data],
        rect: PixelRect,
        canvas: &HtmlCanvasElement,
    ) {
        let rgba_bytes: Vec<u8> = data
            .iter()
            .flat_map(|d| {
                let (r, g, b, a) = (self.colorizer)(d);
                [r, g, b, a]
            })
            .collect();

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
