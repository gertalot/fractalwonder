use crate::rendering::RenderProgress;
use fractalwonder_core::{
    calculate_max_iterations_perturbation, ComputeData, MainToWorker, PixelRect, Viewport,
    WorkerToMain,
};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

const WORKER_SCRIPT_PATH: &str = "./message-compute-worker.js";

/// Cached orbit data for broadcasting to workers.
#[allow(dead_code)] // Fields kept for future rebroadcast/debugging
struct OrbitData {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
}

/// Pending reference orbit computation request.
struct PendingOrbitRequest {
    render_id: u32,
    orbit_id: u32,
    c_ref_json: String,
    max_iterations: u32,
}

/// State for perturbation rendering flow.
#[derive(Default)]
struct PerturbationState {
    /// Current orbit ID being used
    orbit_id: u32,
    /// Workers that have confirmed storing the orbit
    workers_with_orbit: HashSet<usize>,
    /// Orbit data to broadcast
    pending_orbit: Option<OrbitData>,
    /// Maximum iterations for perturbation tiles
    max_iterations: u32,
    /// Delta step per pixel in fractal space
    delta_step: (f64, f64),
    /// Pending orbit computation (waiting for worker to initialize)
    pending_orbit_request: Option<PendingOrbitRequest>,
}

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
    /// Perturbation-specific state
    perturbation: PerturbationState,
    /// Whether current render is using perturbation mode
    is_perturbation_render: bool,
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
            perturbation: PerturbationState::default(),
            is_perturbation_render: false,
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
                    let was_empty = self.initialized_workers.is_empty();
                    self.initialized_workers.insert(worker_id);

                    // If this is the first worker to initialize and we have a pending orbit request,
                    // dispatch it now
                    if was_empty {
                        if let Some(req) = self.perturbation.pending_orbit_request.take() {
                            web_sys::console::log_1(
                                &"[WorkerPool] First worker ready, dispatching queued orbit request"
                                    .into(),
                            );
                            self.send_to_worker(
                                worker_id,
                                &MainToWorker::ComputeReferenceOrbit {
                                    render_id: req.render_id,
                                    orbit_id: req.orbit_id,
                                    c_ref_json: req.c_ref_json,
                                    max_iterations: req.max_iterations,
                                },
                            );
                            return; // Don't dispatch regular work yet, wait for orbit
                        }
                    }
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

            WorkerToMain::ReferenceOrbitComplete {
                render_id,
                orbit_id,
                c_ref,
                orbit,
                escaped_at,
            } => {
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

                // Store orbit data and broadcast to all workers
                self.perturbation.pending_orbit = Some(OrbitData {
                    c_ref,
                    orbit: orbit.clone(),
                    escaped_at,
                });
                self.perturbation.orbit_id = orbit_id;
                self.perturbation.workers_with_orbit.clear();

                // Broadcast to all workers
                for worker_id in 0..self.workers.len() {
                    self.send_to_worker(
                        worker_id,
                        &MainToWorker::StoreReferenceOrbit {
                            orbit_id,
                            c_ref,
                            orbit: orbit.clone(),
                            escaped_at,
                        },
                    );
                }
            }

            WorkerToMain::OrbitStored { orbit_id } => {
                if orbit_id != self.perturbation.orbit_id {
                    return;
                }

                self.perturbation.workers_with_orbit.insert(worker_id);

                // Check if all initialized workers have the orbit
                let all_ready = self
                    .initialized_workers
                    .iter()
                    .all(|&id| self.perturbation.workers_with_orbit.contains(&id));

                if all_ready && !self.pending_tiles.is_empty() {
                    web_sys::console::log_1(
                        &format!(
                            "[WorkerPool] All {} workers have orbit, dispatching {} tiles",
                            self.perturbation.workers_with_orbit.len(),
                            self.pending_tiles.len()
                        )
                        .into(),
                    );

                    // Start dispatching tiles
                    for worker_id in 0..self.workers.len() {
                        if self.initialized_workers.contains(&worker_id) {
                            self.dispatch_work(worker_id);
                        }
                    }
                }
            }
        }
    }

    fn dispatch_work(&mut self, worker_id: usize) {
        // Only send work to initialized workers
        if !self.initialized_workers.contains(&worker_id) {
            return;
        }

        // In perturbation mode, workers must have the orbit cached before receiving tiles
        if self.is_perturbation_render && !self.perturbation.workers_with_orbit.contains(&worker_id)
        {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
            return;
        }

        if let Some(tile) = self.pending_tiles.pop_front() {
            if self.is_perturbation_render {
                // Perturbation mode: send RenderTilePerturbation
                let Some(viewport) = self.current_viewport.as_ref() else {
                    self.send_to_worker(worker_id, &MainToWorker::NoWork);
                    return;
                };

                // Calculate delta_c_origin for this tile's top-left pixel
                //
                // PRECISION FIX: At deep zoom (10^14+), computing tile fractal coords
                // then subtracting c_ref causes catastrophic cancellation because both
                // values are nearly identical when converted to f64.
                //
                // Since c_ref IS the viewport center, we compute delta directly:
                //   delta = (tile_pos - canvas_center) * pixel_size
                // This avoids adding a tiny delta to a large coordinate then subtracting.
                let (canvas_width, canvas_height) = self.canvas_size;
                let vp_width = viewport.width.to_f64();
                let vp_height = viewport.height.to_f64();

                // Tile top-left pixel offset from canvas center in normalized coords [-0.5, 0.5]
                let norm_x = tile.x as f64 / canvas_width as f64 - 0.5;
                let norm_y = tile.y as f64 / canvas_height as f64 - 0.5;

                // Delta from reference point (viewport center) computed directly
                // This preserves full f64 precision for the small delta values
                let delta_c_origin = (norm_x * vp_width, norm_y * vp_height);

                self.send_to_worker(
                    worker_id,
                    &MainToWorker::RenderTilePerturbation {
                        render_id: self.current_render_id,
                        tile,
                        orbit_id: self.perturbation.orbit_id,
                        delta_c_origin,
                        delta_c_step: self.perturbation.delta_step,
                        max_iterations: self.perturbation.max_iterations,
                    },
                );
            } else {
                // Standard mode: send RenderTile
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

        // Wake all workers
        for worker_id in 0..self.workers.len() {
            self.dispatch_work(worker_id);
        }
    }

    /// Start a perturbation render.
    ///
    /// 1. Computes reference orbit at viewport center
    /// 2. Broadcasts orbit to all workers
    /// 3. Dispatches tiles using delta iteration
    pub fn start_perturbation_render(
        &mut self,
        viewport: Viewport,
        canvas_size: (u32, u32),
        tiles: Vec<PixelRect>,
    ) {
        self.is_perturbation_render = true;
        self.current_render_id = self.current_render_id.wrapping_add(1);
        self.perturbation.orbit_id = self.perturbation.orbit_id.wrapping_add(1);
        self.perturbation.workers_with_orbit.clear();
        self.perturbation.pending_orbit = None;

        // Validate viewport dimensions to prevent panics from edge cases
        let vp_width = viewport.width.to_f64();
        let vp_height = viewport.height.to_f64();

        if !vp_width.is_finite() || !vp_height.is_finite() || vp_width <= 0.0 || vp_height <= 0.0 {
            web_sys::console::error_1(
                &format!(
                    "[WorkerPool] Invalid viewport dimensions: width={}, height={}",
                    vp_width, vp_height
                )
                .into(),
            );
            return;
        }

        // Calculate zoom exponent from viewport width
        // Default Mandelbrot width is ~4, so zoom = 4 / width
        let zoom = 4.0 / vp_width;
        let zoom_exponent = if zoom.is_finite() && zoom > 0.0 {
            zoom.log10()
        } else {
            0.0 // Fallback for edge cases
        };
        let max_iterations = calculate_max_iterations_perturbation(zoom_exponent);

        // Calculate delta step (per pixel in fractal space)
        let delta_step = (
            vp_width / canvas_size.0 as f64,
            vp_height / canvas_size.1 as f64,
        );

        self.perturbation.max_iterations = max_iterations;
        self.perturbation.delta_step = delta_step;

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Starting perturbation render #{} with {} tiles, zoom=10^{:.1}, max_iter={}",
                self.current_render_id,
                tiles.len(),
                zoom_exponent,
                max_iterations
            )
            .into(),
        );

        self.current_viewport = Some(viewport.clone());
        self.canvas_size = canvas_size;
        self.pending_tiles = tiles.into();
        self.render_start_time = Some(performance_now());

        let total = self.pending_tiles.len() as u32;
        self.progress.set(RenderProgress::new(total));

        // Serialize viewport center for reference orbit computation
        let c_ref_json = serde_json::to_string(&viewport.center).unwrap_or_default();

        // Send ComputeReferenceOrbit to first available worker, or queue if none ready
        if let Some(&worker_id) = self.initialized_workers.iter().next() {
            self.perturbation.pending_orbit_request = None;
            self.send_to_worker(
                worker_id,
                &MainToWorker::ComputeReferenceOrbit {
                    render_id: self.current_render_id,
                    orbit_id: self.perturbation.orbit_id,
                    c_ref_json,
                    max_iterations,
                },
            );
        } else {
            // No workers ready yet - queue the request for when first worker initializes
            web_sys::console::log_1(
                &"[WorkerPool] No workers initialized yet, queueing orbit request".into(),
            );
            self.perturbation.pending_orbit_request = Some(PendingOrbitRequest {
                render_id: self.current_render_id,
                orbit_id: self.perturbation.orbit_id,
                c_ref_json,
                max_iterations,
            });
        }
    }

    pub fn cancel(&mut self) {
        let pending_count = self.pending_tiles.len();
        if pending_count == 0 && self.progress.get_untracked().is_complete {
            // Nothing to cancel
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Cancelling render #{}, {} tiles pending - terminating workers",
                self.current_render_id, pending_count
            )
            .into(),
        );

        // Reset perturbation state
        self.is_perturbation_render = false;
        self.perturbation.workers_with_orbit.clear();
        self.perturbation.pending_orbit = None;

        // Terminate all workers immediately and recreate them
        // This ensures no stale tiles waste CPU cycles
        self.recreate_workers();

        // Mark progress as complete (cancelled)
        self.progress.update(|p| {
            p.is_complete = true;
        });

        // Bump render_id so any in-flight results are ignored (belt and suspenders)
        self.current_render_id = self.current_render_id.wrapping_add(1);
    }

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
        self.recreate_workers(); // Must recreate workers with new renderer
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
