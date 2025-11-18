use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::{MessageWorkerPool, TileResult};
use fractalwonder_core::{AppData, BigFloat, Point, Rect, Viewport};
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

struct CachedState {
    viewport: Option<Viewport<BigFloat>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<AppData>,
    render_id: AtomicU32,
}

impl Default for CachedState {
    fn default() -> Self {
        Self {
            viewport: None,
            canvas_size: None,
            data: Vec::new(),
            render_id: AtomicU32::new(0),
        }
    }
}

pub struct MessageParallelRenderer {
    worker_pool: Rc<RefCell<MessageWorkerPool>>,
    colorizer: Rc<RefCell<Colorizer<AppData>>>,
    tile_size: u32,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
    progress: RwSignal<crate::rendering::RenderProgress>,
}

impl MessageParallelRenderer {
    pub fn new(colorizer: Colorizer<AppData>, tile_size: u32) -> Result<Self, JsValue> {
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

                // Store tile data in cache at raster positions
                let width = canvas.width();
                for local_y in 0..tile_result.tile.height {
                    for local_x in 0..tile_result.tile.width {
                        let canvas_x = tile_result.tile.x + local_x;
                        let canvas_y = tile_result.tile.y + local_y;
                        let cache_idx = (canvas_y * width + canvas_x) as usize;
                        let tile_idx = (local_y * tile_result.tile.width + local_x) as usize;

                        if cache_idx < cache.data.len() && tile_idx < tile_result.data.len() {
                            cache.data[cache_idx] = tile_result.data[tile_idx].clone();
                        }
                    }
                }

                drop(cache);

                // Draw tile immediately (progressive rendering)
                let colorizer = colorizer_clone.borrow();
                if let Err(e) = draw_tile(canvas, &tile_result, &colorizer) {
                    web_sys::console::error_1(&e);
                }
            }
        };

        let worker_pool = MessageWorkerPool::new(on_tile_complete)?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "MessageParallelRenderer created with {} workers, tile_size={}",
            worker_pool.borrow().worker_count(),
            tile_size
        )));

        Ok(Self {
            worker_pool,
            colorizer,
            tile_size,
            canvas,
            cached_state,
            progress,
        })
    }

    pub fn progress(&self) -> RwSignal<crate::rendering::RenderProgress> {
        self.progress
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.borrow().worker_count()
    }

    fn recolorize_from_cache(
        &self,
        render_id: u32,
        canvas: &HtmlCanvasElement,
    ) -> Result<(), JsValue> {
        let cache = self.cached_state.lock().unwrap();

        if cache.render_id.load(Ordering::SeqCst) != render_id {
            return Ok(()); // Cancelled
        }

        let width = canvas.width();

        let colorizer = self.colorizer.borrow();
        let colors: Vec<u8> = cache
            .data
            .iter()
            .flat_map(|data| {
                let (r, g, b, a) = (*colorizer)(data);
                [r, g, b, a]
            })
            .collect();

        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("No 2d context"))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        let image_data =
            web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&colors), width)?;

        context.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }
}

impl Clone for MessageParallelRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: Rc::clone(&self.colorizer),
            tile_size: self.tile_size,
            canvas: Rc::clone(&self.canvas),
            cached_state: Arc::clone(&self.cached_state),
            progress: self.progress,
        }
    }
}

impl CanvasRenderer for MessageParallelRenderer {
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

        let mut cache = self.cached_state.lock().unwrap();
        let render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

        // Convert f64 viewport to BigFloat
        let viewport_bf = Viewport::new(
            Point::new(
                BigFloat::from(*viewport.center.x()),
                BigFloat::from(*viewport.center.y()),
            ),
            viewport.zoom,
        );

        if cache.viewport.as_ref() == Some(&viewport_bf)
            && cache.canvas_size == Some((width, height))
        {
            // Recolorize from cache
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "RECOLORIZE from cache (render_id: {})",
                render_id
            )));
            drop(cache);
            let _ = self.recolorize_from_cache(render_id, canvas);
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

            self.worker_pool
                .borrow_mut()
                .start_render(viewport_bf, width, height, self.tile_size);
        }
    }

    fn natural_bounds(&self) -> Rect<Self::Scalar> {
        Rect::new(Point::new(-2.5, -1.25), Point::new(1.0, 1.25))
    }

    fn cancel_render(&self) {
        self.worker_pool.borrow_mut().cancel_current_render();
    }
}

fn draw_tile(
    canvas: &HtmlCanvasElement,
    tile_result: &TileResult,
    colorizer: &Colorizer<AppData>,
) -> Result<(), JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("No 2d context"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    let colors: Vec<u8> = tile_result
        .data
        .iter()
        .flat_map(|data| {
            let (r, g, b, a) = colorizer(data);
            [r, g, b, a]
        })
        .collect();

    let image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&colors),
        tile_result.tile.width,
    )?;

    context.put_image_data(
        &image_data,
        tile_result.tile.x as f64,
        tile_result.tile.y as f64,
    )?;

    Ok(())
}
