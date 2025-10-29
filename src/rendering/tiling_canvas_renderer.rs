use crate::rendering::{
    points::Rect,
    renderer_trait::Renderer,
    viewport::Viewport,
    Colorizer, PixelRect,
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
}
