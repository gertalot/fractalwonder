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
