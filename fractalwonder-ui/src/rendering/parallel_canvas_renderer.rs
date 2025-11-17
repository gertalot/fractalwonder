use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::WorkerPool;
use fractalwonder_core::{AppData, Point, Rect, Viewport};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

pub struct ParallelCanvasRenderer {
    worker_pool: Rc<RefCell<WorkerPool>>,
    colorizer: Colorizer<AppData>,
    tile_size: u32,
}

impl ParallelCanvasRenderer {
    pub fn new(colorizer: Colorizer<AppData>, tile_size: u32) -> Result<Self, JsValue> {
        let worker_pool = Rc::new(RefCell::new(WorkerPool::new()?));

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer created with {} workers, tile_size={}",
            worker_pool.borrow().worker_count(),
            tile_size
        )));

        Ok(Self {
            worker_pool,
            colorizer,
            tile_size,
        })
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.borrow().worker_count()
    }
}

impl Clone for ParallelCanvasRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
        }
    }
}

impl CanvasRenderer for ParallelCanvasRenderer {
    type Scalar = f64;
    type Data = AppData;

    fn set_renderer(
        &mut self,
        _renderer: Box<dyn fractalwonder_compute::Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    ) {
        // Not used in parallel renderer - workers handle their own renderers
        web_sys::console::log_1(&JsValue::from_str(
            "ParallelCanvasRenderer::set_renderer called (no-op)"
        ));
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        self.colorizer = colorizer;
        web_sys::console::log_1(&JsValue::from_str("ParallelCanvasRenderer::set_colorizer called"));
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer::render starting ({}x{})",
            width, height
        )));

        // Start render on workers (BLOCKS THREAD - will fix in next task)
        match self.worker_pool.borrow_mut().start_render(viewport, width, height, self.tile_size) {
            Ok(render_id) => {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Render {} dispatched to workers",
                    render_id
                )));
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to start render: {:?}",
                    e
                )));
            }
        }

        // TODO: Poll SharedArrayBuffer and display results progressively
        // For now, just log that we started
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        // Return default Mandelbrot bounds
        // Workers will use their own renderer's bounds
        Rect::new(Point::new(-2.5, -1.25), Point::new(1.0, 1.25))
    }

    fn cancel_render(&self) {
        web_sys::console::log_1(&JsValue::from_str("ParallelCanvasRenderer::cancel_render called"));
        // TODO: Implement cancellation mechanism
    }
}
