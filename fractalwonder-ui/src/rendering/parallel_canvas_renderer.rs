use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::WorkerPool;
#[cfg(target_arch = "wasm32")]
use fractalwonder_compute::atomics::atomic_load_u32;
use fractalwonder_compute::SharedBufferLayout;
use fractalwonder_core::{AppData, Point, Rect, Viewport};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub struct ParallelCanvasRenderer {
    worker_pool: Rc<RefCell<WorkerPool>>,
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    poll_closure: RefCell<Option<Closure<dyn FnMut()>>>,
    current_render_id: RefCell<u32>,
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
            poll_closure: RefCell::new(None),
            current_render_id: RefCell::new(0),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.borrow().worker_count()
    }

    fn poll_and_render(&self, canvas: &HtmlCanvasElement) -> Result<bool, JsValue> {
        let worker_pool = self.worker_pool.borrow();
        let Some(buffer) = worker_pool.get_shared_buffer() else {
            return Ok(false); // No active render
        };

        let width = canvas.width();
        let height = canvas.height();
        let layout = SharedBufferLayout::new(width, height);

        // Calculate total tiles
        let tiles_x = width.div_ceil(self.tile_size);
        let tiles_y = height.div_ceil(self.tile_size);
        let _total_tiles = tiles_x * tiles_y; // Used in wasm32 cfg below

        // Check if render is complete or cancelled (WASM only - uses atomics)
        #[cfg(target_arch = "wasm32")]
        {
            let tile_index = atomic_load_u32(buffer, layout.tile_index_offset() as u32);
            let stored_render_id = atomic_load_u32(buffer, layout.render_id_offset() as u32);
            let current_render_id = *self.current_render_id.borrow();

            // Check for cancellation (render_id mismatch)
            if stored_render_id != current_render_id {
                return Ok(false); // Render was cancelled, stop polling
            }

            // Note: We continue with rendering to display the result
            // Return value indicates whether to continue polling
            // This will be used at the end of the function
            let _ = tile_index; // Will use below
        }

        // Read all pixel data from SharedArrayBuffer
        let view = js_sys::Uint8Array::new(buffer);
        let mut pixel_data = Vec::with_capacity(layout.total_pixels);

        for pixel_idx in 0..layout.total_pixels {
            let offset = layout.pixel_offset(pixel_idx);
            let mut bytes = [0u8; 8];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = view.get_index((offset + i) as u32);
            }

            let data = SharedBufferLayout::decode_pixel(&bytes);
            pixel_data.push(data);
        }

        // Colorize pixels
        let colors = pixel_data
            .iter()
            .flat_map(|data| {
                let app_data = AppData::MandelbrotData(*data);
                let (r, g, b, a) = (self.colorizer)(&app_data);
                [r, g, b, a]
            })
            .collect::<Vec<_>>();

        // Draw to canvas
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("No 2d context"))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        let image_data =
            web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&colors), width)?;

        context.put_image_data(&image_data, 0.0, 0.0)?;

        // Return whether polling should continue
        #[cfg(target_arch = "wasm32")]
        {
            let completed_tiles = atomic_load_u32(buffer, layout.completed_tiles_offset() as u32);
            Ok(completed_tiles < _total_tiles)
        }

        #[cfg(not(target_arch = "wasm32"))]
        Ok(true) // Continue polling on non-WASM platforms (testing)
    }

    fn start_progressive_poll(&self, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let self_clone = self.clone();
        let canvas_clone = canvas.clone();

        let closure = Closure::wrap(Box::new(move || {
            let should_continue = match self_clone.poll_and_render(&canvas_clone) {
                Ok(should_continue) => should_continue,
                Err(e) => {
                    web_sys::console::error_1(&e);
                    false // Stop on error
                }
            };

            // Continue polling only if render is not complete
            if should_continue {
                if let Err(e) = self_clone.start_progressive_poll(canvas_clone.clone()) {
                    web_sys::console::error_1(&e);
                }
            }
        }) as Box<dyn FnMut()>);

        web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window"))?
            .request_animation_frame(closure.as_ref().unchecked_ref())?;

        // Store closure to keep it alive
        *self.poll_closure.borrow_mut() = Some(closure);

        Ok(())
    }
}

impl Clone for ParallelCanvasRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
            poll_closure: RefCell::new(None),
            current_render_id: RefCell::new(*self.current_render_id.borrow()),
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
        // Not used in parallel renderer - workers handle their own renderers
        web_sys::console::log_1(&JsValue::from_str(
            "ParallelCanvasRenderer::set_renderer called (no-op)",
        ));
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        self.colorizer = colorizer;
        web_sys::console::log_1(&JsValue::from_str(
            "ParallelCanvasRenderer::set_colorizer called",
        ));
    }

    fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer::render starting ({}x{})",
            width, height
        )));

        // Start render on workers
        let render_id = match self.worker_pool.borrow_mut().start_render(
            viewport,
            width,
            height,
            self.tile_size,
        ) {
            Ok(render_id) => {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Render {} dispatched to workers",
                    render_id
                )));
                render_id
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to start render: {:?}",
                    e
                )));
                return;
            }
        };

        // Store current render_id for cancellation detection
        *self.current_render_id.borrow_mut() = render_id;

        // Start progressive polling
        if let Err(e) = self.start_progressive_poll(canvas.clone()) {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Failed to start progressive poll: {:?}",
                e
            )));
        }
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        // Return default Mandelbrot bounds
        // Workers will use their own renderer's bounds
        Rect::new(Point::new(-2.5, -1.25), Point::new(1.0, 1.25))
    }

    fn cancel_render(&self) {
        web_sys::console::log_1(&JsValue::from_str(
            "ParallelCanvasRenderer::cancel_render called",
        ));

        // Stop the polling loop by clearing the closure
        *self.poll_closure.borrow_mut() = None;

        // Increment render_id in the worker pool to cancel workers
        self.worker_pool.borrow_mut().cancel_current_render();
    }
}
