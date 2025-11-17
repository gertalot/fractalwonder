use fractalwonder_compute::{MainToWorker, WorkerToMain};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Viewport};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

#[derive(Clone)]
pub struct TileResult {
    pub tile: PixelRect,
    pub data: Vec<AppData>,
    pub compute_time_ms: f64,
}

struct TileRequest {
    tile: PixelRect,
}

pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    pending_tiles: VecDeque<TileRequest>,
    current_render_id: u32,
    current_viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
}

impl MessageWorkerPool {
    pub fn new<F>(on_tile_complete: F) -> Result<Self, JsValue>
    where
        F: Fn(TileResult) + 'static,
    {
        // Get hardware concurrency
        let worker_count = web_sys::window()
            .map(|w| w.navigator().hardware_concurrency() as usize)
            .unwrap_or(4);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating MessageWorkerPool with {} workers",
            worker_count
        )));

        let on_tile_complete = Rc::new(on_tile_complete);

        // Create pool structure
        let pool = Rc::new(RefCell::new(Self {
            workers: Vec::new(),
            pending_tiles: VecDeque::new(),
            current_render_id: 0,
            current_viewport: Viewport::new(
                fractalwonder_core::Point::new(BigFloat::from(0.0), BigFloat::from(0.0)),
                1.0,
            ),
            canvas_size: (0, 0),
            on_tile_complete,
        }));

        // Create workers
        let mut workers = Vec::new();
        for i in 0..worker_count {
            let worker = Worker::new("./message-compute-worker.js")?;

            let worker_id = i;
            let pool_clone = Rc::clone(&pool);

            // Message handler
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(msg_str) = e.data().as_string() {
                    if let Ok(msg) = serde_json::from_str::<WorkerToMain>(&msg_str) {
                        pool_clone
                            .borrow_mut()
                            .handle_worker_message(worker_id, msg);
                    } else {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Worker {} sent invalid message: {}",
                            worker_id, msg_str
                        )));
                    }
                }
            }) as Box<dyn FnMut(_)>);

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            // Error handler
            let error_handler = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Worker {} error: {}",
                    worker_id,
                    e.message()
                )));
            }) as Box<dyn FnMut(_)>);

            worker.set_onerror(Some(error_handler.as_ref().unchecked_ref()));
            error_handler.forget();

            workers.push(worker);

            web_sys::console::log_1(&JsValue::from_str(&format!("Worker {} created", i)));
        }

        pool.borrow_mut().workers = workers;

        Ok(Rc::try_unwrap(pool)
            .unwrap_or_else(|_| panic!("Failed to unwrap worker pool Rc"))
            .into_inner())
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    fn handle_worker_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::RequestWork { render_id } => {
                let should_send_work = match render_id {
                    None => true,
                    Some(id) => id == self.current_render_id,
                };

                if should_send_work {
                    self.send_work_to_worker(worker_id);
                } else {
                    self.send_no_work(worker_id);
                }
            }

            WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            } => {
                if render_id == self.current_render_id {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Worker {} completed tile ({}, {}) in {:.2}ms",
                        worker_id, tile.x, tile.y, compute_time_ms
                    )));

                    (self.on_tile_complete)(TileResult {
                        tile,
                        data,
                        compute_time_ms,
                    });
                } else {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Worker {} completed stale tile (render {} vs current {})",
                        worker_id, render_id, self.current_render_id
                    )));
                }
            }

            WorkerToMain::Error {
                render_id: _,
                tile,
                error,
            } => {
                let tile_str = tile
                    .map(|t| format!("({}, {})", t.x, t.y))
                    .unwrap_or_else(|| "unknown".to_string());

                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Worker {} error on tile {}: {}",
                    worker_id, tile_str, error
                )));
            }
        }
    }

    fn send_work_to_worker(&mut self, worker_id: usize) {
        if let Some(tile_request) = self.pending_tiles.pop_front() {
            let viewport_json = serde_json::to_string(&self.current_viewport)
                .expect("Failed to serialize viewport");

            let msg = MainToWorker::RenderTile {
                render_id: self.current_render_id,
                viewport_json,
                tile: tile_request.tile,
                canvas_width: self.canvas_size.0,
                canvas_height: self.canvas_size.1,
            };

            let msg_json = serde_json::to_string(&msg).expect("Failed to serialize message");
            self.workers[worker_id]
                .post_message(&JsValue::from_str(&msg_json))
                .expect("Failed to post message to worker");
        } else {
            self.send_no_work(worker_id);
        }
    }

    fn send_no_work(&self, worker_id: usize) {
        let msg = MainToWorker::NoWork;
        let msg_json = serde_json::to_string(&msg).expect("Failed to serialize message");
        self.workers[worker_id]
            .post_message(&JsValue::from_str(&msg_json))
            .expect("Failed to post message to worker");
    }

    pub fn start_render(
        &mut self,
        viewport: Viewport<BigFloat>,
        canvas_width: u32,
        canvas_height: u32,
        tile_size: u32,
    ) {
        self.current_render_id += 1;
        self.current_viewport = viewport;
        self.canvas_size = (canvas_width, canvas_height);

        self.pending_tiles = generate_tiles(canvas_width, canvas_height, tile_size);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Starting render {} with {} tiles ({}x{})",
            self.current_render_id,
            self.pending_tiles.len(),
            canvas_width,
            canvas_height
        )));
    }

    pub fn cancel_current_render(&mut self) {
        self.current_render_id += 1;
        self.pending_tiles.clear();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Cancelled render, new render_id: {}",
            self.current_render_id
        )));
    }
}

impl Drop for MessageWorkerPool {
    fn drop(&mut self) {
        let msg = MainToWorker::Terminate;
        let msg_json = serde_json::to_string(&msg).expect("Failed to serialize terminate message");

        for worker in &self.workers {
            worker.post_message(&JsValue::from_str(&msg_json)).ok();
        }
    }
}

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
