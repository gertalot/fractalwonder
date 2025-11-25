use crate::rendering::RenderProgress;
use fractalwonder_core::{ComputeData, MainToWorker, PixelRect, Viewport, WorkerToMain};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

const WORKER_SCRIPT_PATH: &str = "./message-compute-worker.js";

#[derive(Clone)]
pub struct TileResult {
    pub tile: PixelRect,
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,
    initialized_workers: HashSet<usize>,
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    current_viewport: Option<Viewport>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    progress: RwSignal<RenderProgress>,
    render_start_time: Option<f64>,
    self_ref: Weak<RefCell<Self>>,
}

fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn create_workers(count: usize, pool: Rc<RefCell<WorkerPool>>) -> Result<Vec<Worker>, JsValue> {
    web_sys::console::log_1(&format!("[WorkerPool] Creating {} workers", count).into());
    let mut workers = Vec::with_capacity(count);

    for worker_id in 0..count {
        // Create worker as ES module to support import statements
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(WORKER_SCRIPT_PATH, &options)?;

        let pool_clone = Rc::clone(&pool);
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(msg_str) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WorkerToMain>(&msg_str) {
                    pool_clone.borrow_mut().handle_message(worker_id, msg);
                }
            }
        }) as Box<dyn FnMut(_)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onerror = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Worker {} error: {}",
                worker_id,
                e.message()
            )));
        }) as Box<dyn FnMut(_)>);

        worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        workers.push(worker);
    }

    Ok(workers)
}

impl WorkerPool {
    pub fn new<F>(
        renderer_id: &str,
        on_tile_complete: F,
        progress: RwSignal<RenderProgress>,
    ) -> Result<Rc<RefCell<Self>>, JsValue>
    where
        F: Fn(TileResult) + 'static,
    {
        let worker_count = web_sys::window()
            .map(|w| w.navigator().hardware_concurrency() as usize)
            .unwrap_or(4)
            .max(1);

        let pool = Rc::new(RefCell::new(Self {
            workers: Vec::new(),
            renderer_id: renderer_id.to_string(),
            initialized_workers: HashSet::new(),
            pending_tiles: VecDeque::new(),
            current_render_id: 0,
            current_viewport: None,
            canvas_size: (0, 0),
            on_tile_complete: Rc::new(on_tile_complete),
            progress,
            render_start_time: None,
            self_ref: Weak::new(),
        }));

        pool.borrow_mut().self_ref = Rc::downgrade(&pool);

        let workers = create_workers(worker_count, Rc::clone(&pool))?;
        pool.borrow_mut().workers = workers;

        Ok(pool)
    }

    fn send_to_worker(&self, worker_id: usize, msg: &MainToWorker) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.workers[worker_id].post_message(&JsValue::from_str(&json));
        }
    }

    fn handle_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => {
                // Send Initialize
                self.send_to_worker(
                    worker_id,
                    &MainToWorker::Initialize {
                        renderer_id: self.renderer_id.clone(),
                    },
                );
            }

            WorkerToMain::RequestWork { render_id } => {
                // Track initialization - first RequestWork after Initialize has render_id: None
                if render_id.is_none() {
                    self.initialized_workers.insert(worker_id);
                }

                // Only send work if render_id matches or is None (just initialized)
                let should_send = match render_id {
                    None => true,
                    Some(id) => id == self.current_render_id,
                };

                if should_send {
                    self.dispatch_work(worker_id);
                } else {
                    self.send_to_worker(worker_id, &MainToWorker::NoWork);
                }
            }

            WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            } => {
                if render_id == self.current_render_id {
                    // let progress = self.progress.get_untracked();
                    // web_sys::console::log_1(
                    //     &format!(
                    //         "[WorkerPool] Tile complete: render #{}, {}/{} tiles, {:.0}ms",
                    //         render_id,
                    //         progress.completed_tiles + 1,
                    //         progress.total_tiles,
                    //         compute_time_ms
                    //     )
                    //     .into(),
                    // );
                    // Update progress
                    let elapsed = self
                        .render_start_time
                        .map(|start| performance_now() - start)
                        .unwrap_or(0.0);

                    self.progress.update(|p| {
                        p.completed_tiles += 1;
                        p.elapsed_ms = elapsed;
                        p.is_complete = p.completed_tiles >= p.total_tiles;
                    });

                    // Callback
                    (self.on_tile_complete)(TileResult {
                        tile,
                        data,
                        compute_time_ms,
                    });
                } else {
                    web_sys::console::warn_1(
                        &format!(
                            "[WorkerPool] Ignoring stale tile from render #{} (current: #{})",
                            render_id, self.current_render_id
                        )
                        .into(),
                    );
                }
            }

            WorkerToMain::Error { message } => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Worker {} error: {}",
                    worker_id, message
                )));
            }
        }
    }

    fn dispatch_work(&mut self, worker_id: usize) {
        // Only send work to initialized workers
        if !self.initialized_workers.contains(&worker_id) {
            return;
        }

        if let Some(tile) = self.pending_tiles.pop_front() {
            // Compute tile-specific viewport
            let tile_viewport = self
                .current_viewport
                .as_ref()
                .map(|vp| crate::rendering::tile_to_viewport(&tile, vp, self.canvas_size));

            let viewport_json = tile_viewport
                .and_then(|v| serde_json::to_string(&v).ok())
                .unwrap_or_default();

            self.send_to_worker(
                worker_id,
                &MainToWorker::RenderTile {
                    render_id: self.current_render_id,
                    viewport_json,
                    tile,
                },
            );
        } else {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
        }
    }

    pub fn start_render(
        &mut self,
        viewport: Viewport,
        canvas_size: (u32, u32),
        tiles: Vec<PixelRect>,
    ) {
        self.current_render_id = self.current_render_id.wrapping_add(1);
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Starting render #{} with {} tiles, precision={} bits",
                self.current_render_id,
                tiles.len(),
                viewport.precision_bits()
            )
            .into(),
        );
        self.current_viewport = Some(viewport);
        self.canvas_size = canvas_size;
        self.pending_tiles = tiles.into();
        self.render_start_time = Some(performance_now());

        let total = self.pending_tiles.len() as u32;
        self.progress.set(RenderProgress::new(total));

        // Wake all workers
        for worker_id in 0..self.workers.len() {
            self.dispatch_work(worker_id);
        }
    }

    pub fn cancel(&mut self) {
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Cancelling render #{}, {} tiles pending",
                self.current_render_id,
                self.pending_tiles.len()
            )
            .into(),
        );
        // Terminate all workers
        for worker in &self.workers {
            worker.terminate();
        }

        self.pending_tiles.clear();
        self.initialized_workers.clear();

        // Recreate workers
        if let Some(pool_rc) = self.self_ref.upgrade() {
            if let Ok(new_workers) = create_workers(self.workers.len(), pool_rc) {
                self.workers = new_workers;
            }
        }
    }

    pub fn switch_renderer(&mut self, renderer_id: &str) {
        self.renderer_id = renderer_id.to_string();
        self.cancel(); // Terminates and recreates with new renderer
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for worker in &self.workers {
            let _ = serde_json::to_string(&MainToWorker::Terminate)
                .map(|json| worker.post_message(&JsValue::from_str(&json)));
        }
    }
}
