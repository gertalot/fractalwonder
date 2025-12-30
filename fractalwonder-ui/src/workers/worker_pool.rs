use crate::config::get_config;
use crate::rendering::RenderProgress;
use crate::workers::perturbation::{OrbitData, PerturbationCoordinator};
use crate::workers::worker_pool_types::{
    performance_now, OrbitCompleteCallback, OrbitCompleteData, PendingOrbitRequest,
    RenderCompleteCallback, TileResult,
};
use fractalwonder_core::{ComputeData, MainToWorker, PixelRect, Viewport, WorkerToMain};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

const WORKER_SCRIPT_PATH: &str = "./message-compute-worker.js";

pub struct WorkerPool {
    pub(super) workers: Vec<Worker>,
    renderer_id: String,
    initialized_workers: HashSet<usize>,
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    pub(super) current_viewport: Option<Viewport>,
    pub(super) canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    on_render_complete: RenderCompleteCallback,
    on_orbit_complete: OrbitCompleteCallback,
    progress: RwSignal<RenderProgress>,
    render_start_time: Option<f64>,
    self_ref: Weak<RefCell<Self>>,
    /// Perturbation coordinator (handles state, glitch resolution, tile messages)
    pub(super) perturbation: PerturbationCoordinator,
    /// Whether current render is using perturbation mode
    is_perturbation_render: bool,
    /// GPU mode: orbit complete callback handles rendering, skip tile dispatch
    gpu_mode: bool,
    /// Pending orbit computation (waiting for worker to initialize)
    pending_orbit_request: Option<PendingOrbitRequest>,
    /// Cached orbit data for callbacks
    pending_orbit_data: Option<OrbitData>,
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
            let Some(msg_str) = e.data().as_string() else {
                web_sys::console::error_1(
                    &format!("[WorkerPool] Worker {worker_id} non-string msg").into(),
                );
                return;
            };
            match serde_json::from_str::<WorkerToMain>(&msg_str) {
                Ok(msg) => pool_clone.borrow_mut().handle_message(worker_id, msg),
                Err(err) => {
                    let preview: String = msg_str.chars().take(200).collect();
                    web_sys::console::error_1(&format!("[WorkerPool] Parse error worker {worker_id}: {err}. Preview: {preview}").into());
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
        // Determine worker count from config (0 = use hardware concurrency)
        let config_worker_count = get_config(renderer_id).map(|c| c.worker_count).unwrap_or(0);

        let worker_count = if config_worker_count == 0 {
            web_sys::window()
                .map(|w| w.navigator().hardware_concurrency() as usize)
                .unwrap_or(4)
                .max(1)
        } else {
            config_worker_count
        };

        let pool = Rc::new(RefCell::new(Self {
            workers: Vec::new(),
            renderer_id: renderer_id.to_string(),
            initialized_workers: HashSet::new(),
            pending_tiles: VecDeque::new(),
            current_render_id: 0,
            current_viewport: None,
            canvas_size: (0, 0),
            on_tile_complete: Rc::new(on_tile_complete),
            on_render_complete: Rc::new(RefCell::new(None)),
            on_orbit_complete: Rc::new(RefCell::new(None)),
            progress,
            render_start_time: None,
            self_ref: Weak::new(),
            perturbation: PerturbationCoordinator::new(renderer_id),
            is_perturbation_render: false,
            gpu_mode: false,
            pending_orbit_request: None,
            pending_orbit_data: None,
        }));

        pool.borrow_mut().self_ref = Rc::downgrade(&pool);

        let workers = create_workers(worker_count, Rc::clone(&pool))?;
        pool.borrow_mut().workers = workers;

        Ok(pool)
    }

    pub(super) fn send_to_worker(&self, worker_id: usize, msg: &MainToWorker) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.workers[worker_id].post_message(&JsValue::from_str(&json));
        }
    }

    fn handle_ready(&mut self, worker_id: usize) {
        self.send_to_worker(
            worker_id,
            &MainToWorker::Initialize {
                renderer_id: self.renderer_id.clone(),
            },
        );
    }

    fn handle_request_work(&mut self, worker_id: usize, render_id: Option<u32>) {
        if render_id.is_none() {
            let was_empty = self.initialized_workers.is_empty();
            self.initialized_workers.insert(worker_id);
            if was_empty {
                if let Some(pending) = self.pending_orbit_request.take() {
                    web_sys::console::log_1(
                        &"[WorkerPool] First worker ready, dispatching queued orbit request".into(),
                    );
                    self.send_to_worker(
                        worker_id,
                        &MainToWorker::ComputeReferenceOrbit {
                            render_id: pending.request.render_id,
                            orbit_id: pending.request.orbit_id,
                            c_ref_json: pending.request.c_ref_json,
                            max_iterations: pending.request.max_iterations,
                        },
                    );
                    return;
                }
            }
        }

        if render_id.is_none_or(|id| id == self.current_render_id) {
            self.dispatch_work(worker_id);
        } else {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
        }
    }

    fn handle_tile_complete(
        &mut self,
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
        bla_iterations: u64,
        total_iterations: u64,
    ) {
        if render_id != self.current_render_id {
            web_sys::console::warn_1(
                &format!(
                    "[WorkerPool] Ignoring stale tile from render #{} (current: #{})",
                    render_id, self.current_render_id
                )
                .into(),
            );
            return;
        }

        if self.is_perturbation_render {
            let glitched_count = data
                .iter()
                .filter(|d| matches!(d, ComputeData::Mandelbrot(m) if m.glitched))
                .count();
            let bla_pct = if total_iterations > 0 {
                (bla_iterations as f64 / total_iterations as f64) * 100.0
            } else {
                0.0
            };
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Tile ({},{}): {}/{} glitched, {:.1}% BLA ({}/{})",
                    tile.x, tile.y, glitched_count, data.len(), bla_pct,
                    bla_iterations, total_iterations
                )
                .into(),
            );
            if glitched_count > 0 {
                self.perturbation.glitch_resolver_mut().record_glitched_tile(tile);
            }
        }

        let elapsed = self
            .render_start_time
            .map(|start| performance_now() - start)
            .unwrap_or(0.0);
        let is_complete = {
            let mut complete = false;
            self.progress.update(|p| {
                p.completed_steps += 1;
                p.elapsed_ms = elapsed;
                p.is_complete = p.completed_steps >= p.total_steps;
                complete = p.is_complete;
            });
            complete
        };

        if is_complete && self.is_perturbation_render {
            let total = self.progress.get_untracked().total_steps;
            let glitched_count = self.perturbation.glitch_resolver().glitched_tile_count();
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Render complete: {} tiles had glitches (of {} total)",
                    glitched_count, total
                )
                .into(),
            );
        }

        (self.on_tile_complete)(TileResult {
            tile,
            data,
            compute_time_ms,
        });

        if is_complete {
            if let Some(ref callback) = *self.on_render_complete.borrow() {
                callback();
            }
        }
    }

    fn handle_error(&self, worker_id: usize, message: String) {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Worker {} error: {}",
            worker_id, message
        )));
    }

    fn handle_orbit_complete(
        &mut self,
        render_id: u32,
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        derivative: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    ) {
        if render_id != self.current_render_id {
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Reference orbit complete: {} points, escaped_at={:?}",
                orbit.len(),
                escaped_at
            )
            .into(),
        );

        let orbit_data = OrbitData {
            c_ref,
            orbit: orbit.clone(),
            derivative: derivative.clone(),
            escaped_at,
        };
        self.pending_orbit_data = Some(orbit_data.clone());

        if self.gpu_mode {
            web_sys::console::log_1(&"[WorkerPool] GPU mode: triggering orbit callback".into());
            if let Some(callback) = self.on_orbit_complete.borrow().as_ref() {
                callback(OrbitCompleteData {
                    orbit,
                    derivative,
                    orbit_id,
                    max_iterations: self.perturbation.max_iterations(),
                    escaped_at,
                });
            }
            return;
        }

        let msg = self.perturbation.build_orbit_broadcast(&orbit_data);
        for worker_id in 0..self.workers.len() {
            self.send_to_worker(worker_id, &msg);
        }
    }

    fn handle_orbit_stored(&mut self, worker_id: usize, orbit_id: u32) {
        if self
            .perturbation
            .glitch_resolver()
            .is_tracking_orbit(orbit_id)
        {
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Worker {} stored orbit #{}",
                    worker_id, orbit_id
                )
                .into(),
            );
            let all_confirmed = self
                .perturbation
                .glitch_resolver_mut()
                .confirm_orbit_stored(orbit_id, worker_id, &self.initialized_workers);
            if all_confirmed {
                web_sys::console::log_1(
                    &format!(
                        "[WorkerPool] Phase 8: All workers confirmed orbit #{}",
                        orbit_id
                    )
                    .into(),
                );
            }
            return;
        }

        if orbit_id != self.perturbation.orbit_id() {
            return;
        }

        self.perturbation.record_worker_has_orbit(worker_id);

        if self
            .perturbation
            .all_workers_have_orbit(&self.initialized_workers)
            && !self.pending_tiles.is_empty()
        {
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] All {} workers have orbit, dispatching {} tiles",
                    self.perturbation.workers_with_orbit_count(),
                    self.pending_tiles.len()
                )
                .into(),
            );
            for worker_id in 0..self.workers.len() {
                if self.initialized_workers.contains(&worker_id) {
                    self.dispatch_work(worker_id);
                }
            }
        }
    }

    fn handle_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => self.handle_ready(worker_id),
            WorkerToMain::RequestWork { render_id } => {
                self.handle_request_work(worker_id, render_id)
            }
            WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
                bla_iterations,
                total_iterations,
            } => self.handle_tile_complete(
                render_id,
                tile,
                data,
                compute_time_ms,
                bla_iterations,
                total_iterations,
            ),
            WorkerToMain::Error { message } => self.handle_error(worker_id, message),
            WorkerToMain::ReferenceOrbitComplete {
                render_id,
                orbit_id,
                c_ref,
                orbit,
                derivative,
                escaped_at,
            } => self
                .handle_orbit_complete(render_id, orbit_id, c_ref, orbit, derivative, escaped_at),
            WorkerToMain::OrbitStored { orbit_id } => self.handle_orbit_stored(worker_id, orbit_id),
        }
    }

    fn dispatch_work(&mut self, worker_id: usize) {
        if !self.initialized_workers.contains(&worker_id) {
            return;
        }

        if self.is_perturbation_render && !self.perturbation.worker_ready_for_tiles(worker_id) {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
            return;
        }

        if let Some(tile) = self.pending_tiles.pop_front() {
            if self.is_perturbation_render {
                if let Some(msg) = self
                    .perturbation
                    .build_tile_message(self.current_render_id, tile)
                {
                    self.send_to_worker(worker_id, &msg);
                } else {
                    self.send_to_worker(worker_id, &MainToWorker::NoWork);
                }
            } else {
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
            }
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
        self.is_perturbation_render = false;
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

        for worker_id in 0..self.workers.len() {
            self.dispatch_work(worker_id);
        }
    }

    pub fn start_perturbation_render(
        &mut self,
        viewport: Viewport,
        canvas_size: (u32, u32),
        tiles: Vec<PixelRect>,
        force_hdr_float: bool,
    ) {
        self.is_perturbation_render = true;
        self.gpu_mode = false;
        self.current_render_id = self.current_render_id.wrapping_add(1);

        // Set force_hdr_float before starting render
        self.perturbation.set_force_hdr_float(force_hdr_float);

        let orbit_request =
            match self
                .perturbation
                .start_render(self.current_render_id, &viewport, canvas_size)
            {
                Ok(req) => req,
                Err(e) => {
                    web_sys::console::error_1(&format!("[WorkerPool] {}", e).into());
                    return;
                }
            };

        let zoom_exponent = (4.0 / viewport.width.to_f64()).log10();
        web_sys::console::log_1(&format!(
            "[WorkerPool] Starting perturbation render #{} with {} tiles, zoom=10^{:.1}, max_iter={}",
            self.current_render_id, tiles.len(), zoom_exponent, orbit_request.max_iterations
        ).into());

        self.current_viewport = Some(viewport);
        self.canvas_size = canvas_size;
        self.pending_tiles = tiles.into();
        self.render_start_time = Some(performance_now());
        self.progress
            .set(RenderProgress::new(self.pending_tiles.len() as u32));

        if let Some(&worker_id) = self.initialized_workers.iter().next() {
            self.pending_orbit_request = None;
            self.send_to_worker(
                worker_id,
                &MainToWorker::ComputeReferenceOrbit {
                    render_id: orbit_request.render_id,
                    orbit_id: orbit_request.orbit_id,
                    c_ref_json: orbit_request.c_ref_json,
                    max_iterations: orbit_request.max_iterations,
                },
            );
        } else {
            web_sys::console::log_1(
                &"[WorkerPool] No workers initialized yet, queueing orbit request".into(),
            );
            self.pending_orbit_request = Some(PendingOrbitRequest {
                request: orbit_request,
            });
        }
    }

    pub fn cancel(&mut self) {
        let pending_count = self.pending_tiles.len();
        if pending_count == 0 && self.progress.get_untracked().is_complete {
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Cancelling render #{}, {} tiles pending - terminating workers",
                self.current_render_id, pending_count
            )
            .into(),
        );

        self.is_perturbation_render = false;
        self.perturbation.reset();

        self.recreate_workers();

        self.progress.update(|p| {
            p.is_complete = true;
        });

        self.current_render_id = self.current_render_id.wrapping_add(1);
    }

    // Note: subdivide_glitched_cells and related methods are in worker_pool_glitch.rs

    /// Terminate and recreate all workers. Used when switching renderers.
    fn recreate_workers(&mut self) {
        web_sys::console::log_1(
            &format!("[WorkerPool] Recreating {} workers", self.workers.len()).into(),
        );

        for worker in &self.workers {
            worker.terminate();
        }

        self.pending_tiles.clear();
        self.initialized_workers.clear();

        if let Some(pool_rc) = self.self_ref.upgrade() {
            if let Ok(new_workers) = create_workers(self.workers.len(), pool_rc) {
                self.workers = new_workers;
            }
        }
    }

    pub fn switch_renderer(&mut self, renderer_id: &str) {
        self.renderer_id = renderer_id.to_string();
        self.perturbation.set_renderer_id(renderer_id);
        self.recreate_workers();
    }

    /// Set callback for when orbit computation completes.
    /// Used by GPU rendering path to receive orbit data.
    pub fn set_orbit_complete_callback<F>(&self, callback: F)
    where
        F: Fn(OrbitCompleteData) + 'static,
    {
        *self.on_orbit_complete.borrow_mut() = Some(Box::new(callback));
    }

    /// Clear the orbit complete callback.
    pub fn clear_orbit_complete_callback(&self) {
        *self.on_orbit_complete.borrow_mut() = None;
    }

    /// Set callback for when all tiles are complete.
    pub fn set_render_complete_callback<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_render_complete.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn compute_orbit_for_gpu(&mut self, viewport: Viewport, canvas_size: (u32, u32)) {
        self.gpu_mode = true;
        self.is_perturbation_render = false;
        self.current_render_id = self.current_render_id.wrapping_add(1);

        let orbit_request =
            match self
                .perturbation
                .start_gpu_render(self.current_render_id, &viewport, canvas_size)
            {
                Ok(req) => req,
                Err(e) => {
                    web_sys::console::error_1(&format!("[WorkerPool] {}", e).into());
                    return;
                }
            };

        let zoom_exponent = (4.0 / viewport.width.to_f64()).log10();
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Computing orbit for GPU render #{}, zoom=10^{:.1}, max_iter={}",
                self.current_render_id, zoom_exponent, orbit_request.max_iterations
            )
            .into(),
        );

        self.current_viewport = Some(viewport);
        self.canvas_size = canvas_size;
        self.render_start_time = Some(performance_now());

        if let Some(&worker_id) = self.initialized_workers.iter().next() {
            self.pending_orbit_request = None;
            self.send_to_worker(
                worker_id,
                &MainToWorker::ComputeReferenceOrbit {
                    render_id: orbit_request.render_id,
                    orbit_id: orbit_request.orbit_id,
                    c_ref_json: orbit_request.c_ref_json,
                    max_iterations: orbit_request.max_iterations,
                },
            );
        } else {
            web_sys::console::log_1(
                &"[WorkerPool] No workers initialized yet, queueing orbit request".into(),
            );
            self.pending_orbit_request = Some(PendingOrbitRequest {
                request: orbit_request,
            });
        }
    }

    pub fn get_orbit(&self) -> Option<(Vec<(f64, f64)>, u32)> {
        self.pending_orbit_data
            .as_ref()
            .map(|o| (o.orbit.clone(), self.perturbation.orbit_id()))
    }

    pub fn get_max_iterations(&self) -> u32 {
        self.perturbation.max_iterations()
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
