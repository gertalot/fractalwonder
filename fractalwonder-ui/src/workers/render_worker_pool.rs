use crate::rendering::parallel_canvas_renderer::TileRequest;
use fractalwonder_compute::{MainToWorker, WorkerToMain};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Viewport};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

/// Path to the Web Worker script that wraps the WASM compute module
/// This file is copied to dist/ by Trunk (see index.html data-trunk directive)
/// This is a js file that loads the WASM built by the fractalwonder-compute crate
const WORKER_SCRIPT_PATH: &str = "./message-compute-worker.js";

#[derive(Clone)]
pub struct TileResult {
    pub tile: PixelRect,
    pub data: Vec<AppData>,
    pub compute_time_ms: f64,
}

pub struct RenderWorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,
    pending_tiles: VecDeque<TileRequest>,
    failed_tiles: HashMap<(u32, u32), u32>, // (x, y) -> retry_count
    current_render_id: u32,
    current_viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,
    render_start_time: Rc<RefCell<Option<f64>>>,
    self_ref: Weak<RefCell<Self>>,
}

fn create_workers(
    worker_count: usize,
    pool: Rc<RefCell<RenderWorkerPool>>,
) -> Result<Vec<Worker>, JsValue> {
    let mut workers = Vec::new();

    for i in 0..worker_count {
        let worker = Worker::new(WORKER_SCRIPT_PATH)?;

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

    Ok(workers)
}

impl RenderWorkerPool {
    pub fn new<F>(
        on_tile_complete: F,
        progress_signal: RwSignal<crate::rendering::RenderProgress>,
        renderer_id: String,
    ) -> Result<Rc<RefCell<Self>>, JsValue>
    where
        F: Fn(TileResult) + 'static,
    {
        // Get hardware concurrency
        let worker_count = web_sys::window()
            .map(|w| w.navigator().hardware_concurrency() as usize)
            .unwrap_or(4);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating RenderWorkerPool with {} workers",
            worker_count
        )));

        let on_tile_complete = Rc::new(on_tile_complete);

        // Create pool structure
        let pool = Rc::new(RefCell::new(Self {
            workers: Vec::new(),
            renderer_id,
            pending_tiles: VecDeque::new(),
            failed_tiles: HashMap::new(),
            current_render_id: 0,
            current_viewport: Viewport::new(
                fractalwonder_core::Point::new(BigFloat::from(0.0), BigFloat::from(0.0)),
                1.0,
            ),
            canvas_size: (0, 0),
            on_tile_complete,
            progress_signal,
            render_start_time: Rc::new(RefCell::new(None)),
            self_ref: Weak::new(),
        }));

        // Store weak reference to self
        pool.borrow_mut().self_ref = Rc::downgrade(&pool);

        // Create workers using extracted function
        let workers = create_workers(worker_count, Rc::clone(&pool))?;
        pool.borrow_mut().workers = workers;

        Ok(pool)
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    fn handle_worker_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => {
                // TODO: Send Initialize message with renderer_id (Task 8)
                // For now, just log and request work to maintain current behavior
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Worker {} ready (Initialize protocol not yet implemented)",
                    worker_id
                )));
            }

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

                    // Calculate elapsed time and update progress
                    let elapsed_ms = if let Some(start) = *self.render_start_time.borrow() {
                        web_sys::window().unwrap().performance().unwrap().now() - start
                    } else {
                        0.0
                    };

                    self.progress_signal.update(|p| {
                        if p.render_id == render_id {
                            p.completed_tiles += 1;
                            p.elapsed_ms = elapsed_ms;
                            p.is_complete = p.completed_tiles >= p.total_tiles;
                        }
                    });

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
                if let Some(tile) = tile {
                    let tile_key = (tile.x, tile.y);
                    let retry_count = self.failed_tiles.entry(tile_key).or_insert(0);

                    if *retry_count < 1 {
                        // Retry once
                        *retry_count += 1;
                        web_sys::console::warn_1(&JsValue::from_str(&format!(
                            "Tile ({}, {}) failed, retrying (attempt {}): {}",
                            tile.x,
                            tile.y,
                            *retry_count + 1,
                            error
                        )));

                        self.pending_tiles.push_back(TileRequest { tile });
                    } else {
                        // Give up after one retry
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Tile ({}, {}) failed after retry: {}",
                            tile.x, tile.y, error
                        )));
                    }
                } else {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Worker {} error: {}",
                        worker_id, error
                    )));
                }
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

    pub(crate) fn start_render(
        &mut self,
        viewport: Viewport<BigFloat>,
        canvas_width: u32,
        canvas_height: u32,
        tiles: VecDeque<TileRequest>,
        render_id: u32,
    ) {
        self.current_render_id = render_id;
        self.current_viewport = viewport;
        self.canvas_size = (canvas_width, canvas_height);

        // Clear retry tracking for new render
        self.failed_tiles.clear();

        self.pending_tiles = tiles;
        let total_tiles = self.pending_tiles.len() as u32;

        // Record start time
        let start_time = web_sys::window().unwrap().performance().unwrap().now();
        *self.render_start_time.borrow_mut() = Some(start_time);

        // Initialize progress signal
        self.progress_signal
            .set(crate::rendering::RenderProgress::new(
                total_tiles,
                self.current_render_id,
            ));

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Starting render {} with {} tiles ({}x{})",
            self.current_render_id, total_tiles, canvas_width, canvas_height
        )));

        // Wake up all idle workers by sending them work requests
        // Workers that are busy will ignore this (they already have work)
        // Workers that are idle will receive tiles and start working
        for worker_id in 0..self.workers.len() {
            self.send_work_to_worker(worker_id);
        }
    }

    pub fn cancel_current_render(&mut self) {
        // 1. Terminate all workers immediately
        for worker in &self.workers {
            worker.terminate();
        }

        web_sys::console::log_1(&JsValue::from_str(
            "Terminated all workers for cancellation",
        ));

        // 2. Recreate workers using stored self-reference
        if let Some(pool_rc) = self.self_ref.upgrade() {
            match create_workers(self.workers.len(), pool_rc) {
                Ok(new_workers) => {
                    self.workers = new_workers;
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Recreated {} workers",
                        self.workers.len()
                    )));
                }
                Err(e) => {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Failed to recreate workers: {:?}",
                        e
                    )));
                    // Keep empty workers vec - pool is broken
                    self.workers.clear();
                }
            }
        }

        // 3. Clear pending work
        self.pending_tiles.clear();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Cancelled render at render_id: {}",
            self.current_render_id
        )));
    }
}

impl Drop for RenderWorkerPool {
    fn drop(&mut self) {
        let msg = MainToWorker::Terminate;
        let msg_json = serde_json::to_string(&msg).expect("Failed to serialize terminate message");

        for worker in &self.workers {
            worker.post_message(&JsValue::from_str(&msg_json)).ok();
        }
    }
}
