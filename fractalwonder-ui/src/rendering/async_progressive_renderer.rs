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
}
