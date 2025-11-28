use crate::config::get_config;
use crate::rendering::RenderProgress;
use crate::workers::quadtree::{Bounds, QuadtreeCell};
use fractalwonder_compute::ReferenceOrbit;
use fractalwonder_core::{
    calculate_max_iterations, pixel_to_fractal, BigFloat, ComputeData, MainToWorker, PixelRect,
    Viewport, WorkerToMain,
};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
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
struct PerturbationState {
    /// Current orbit ID being used
    orbit_id: u32,
    /// Workers that have confirmed storing the orbit
    workers_with_orbit: HashSet<usize>,
    /// Orbit data to broadcast
    pending_orbit: Option<OrbitData>,
    /// Maximum iterations for perturbation tiles
    max_iterations: u32,
    /// Delta step per pixel in fractal space (BigFloat for deep zoom)
    delta_step: (BigFloat, BigFloat),
    /// Pending orbit computation (waiting for worker to initialize)
    pending_orbit_request: Option<PendingOrbitRequest>,
    /// Glitch detection threshold squared (τ²)
    tau_sq: f64,
    /// Maximum |δc| for BLA table construction
    dc_max: f64,
}

impl Default for PerturbationState {
    fn default() -> Self {
        Self {
            orbit_id: 0,
            workers_with_orbit: HashSet::new(),
            pending_orbit: None,
            max_iterations: 0,
            delta_step: (BigFloat::zero(64), BigFloat::zero(64)),
            pending_orbit_request: None,
            tau_sq: 1e-6,
            dc_max: 0.0,
        }
    }
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
    /// Count of tiles with glitched pixels in current render
    glitched_tile_count: u32,
    /// Quadtree for spatial tracking of glitched regions
    quadtree: Option<QuadtreeCell>,
    /// Bounds of tiles that have glitched pixels (for associating with quadtree cells)
    glitched_tiles: Vec<PixelRect>,
    /// Computed reference orbits for quadtree cell centers (Phase 7)
    /// Key: (x, y, width, height) of cell bounds
    cell_orbits: HashMap<(u32, u32, u32, u32), ReferenceOrbit>,
    /// Mapping from cell bounds to orbit_id for worker distribution (Phase 8)
    cell_orbit_ids: HashMap<(u32, u32, u32, u32), u32>,
    /// Counter for generating unique cell orbit IDs (separate from main perturbation orbit)
    cell_orbit_id_counter: u32,
    /// Tracks which workers have confirmed storing which cell orbits
    /// Key: orbit_id, Value: set of worker_ids that have confirmed
    cell_orbit_confirmations: HashMap<u32, HashSet<usize>>,
}

fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Calculate maximum |δc| for any pixel in the viewport.
/// This is the distance from viewport center to the farthest corner.
fn calculate_dc_max(viewport: &Viewport, _canvas_size: (u32, u32)) -> f64 {
    // Half-width and half-height in fractal coordinates
    let half_width = viewport.width.to_f64() / 2.0;
    let half_height = viewport.height.to_f64() / 2.0;

    // Euclidean distance to corner
    (half_width * half_width + half_height * half_height).sqrt()
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
            progress,
            render_start_time: None,
            self_ref: Weak::new(),
            perturbation: PerturbationState::default(),
            is_perturbation_render: false,
            glitched_tile_count: 0,
            quadtree: None,
            glitched_tiles: Vec::new(),
            cell_orbits: HashMap::new(),
            cell_orbit_ids: HashMap::new(),
            cell_orbit_id_counter: 1000, // Start at 1000 to distinguish from main orbit IDs
            cell_orbit_confirmations: HashMap::new(),
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
                    // Count glitched pixels (perturbation mode only)
                    if self.is_perturbation_render {
                        let glitched_count = data
                            .iter()
                            .filter(|d| matches!(d, ComputeData::Mandelbrot(m) if m.glitched))
                            .count();

                        if glitched_count > 0 {
                            let total_pixels = data.len();
                            web_sys::console::log_1(
                                &format!(
                                    "[WorkerPool] Tile ({},{}): {}/{} pixels glitched",
                                    tile.x, tile.y, glitched_count, total_pixels
                                )
                                .into(),
                            );
                            self.glitched_tile_count += 1;
                            self.glitched_tiles.push(tile);
                        }
                    }

                    // Update progress
                    let elapsed = self
                        .render_start_time
                        .map(|start| performance_now() - start)
                        .unwrap_or(0.0);

                    let is_complete = {
                        let mut complete = false;
                        self.progress.update(|p| {
                            p.completed_tiles += 1;
                            p.elapsed_ms = elapsed;
                            p.is_complete = p.completed_tiles >= p.total_tiles;
                            complete = p.is_complete;
                        });
                        complete
                    };

                    // Log render completion summary (perturbation mode only for glitch stats)
                    if is_complete && self.is_perturbation_render {
                        let total = self.progress.get_untracked().total_tiles;
                        web_sys::console::log_1(
                            &format!(
                                "[WorkerPool] Render complete: {} tiles had glitches (of {} total)",
                                self.glitched_tile_count, total
                            )
                            .into(),
                        );

                        // Log quadtree cell glitch tracking
                        if let Some(qt) = &self.quadtree {
                            let b = &qt.bounds;
                            web_sys::console::log_1(
                                &format!(
                                    "[WorkerPool] Quadtree cell ({},{})-({},{}): {} glitched tiles",
                                    b.x,
                                    b.y,
                                    b.x + b.width,
                                    b.y + b.height,
                                    self.glitched_tile_count
                                )
                                .into(),
                            );
                        }
                    }

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
                            dc_max: self.perturbation.dc_max,
                        },
                    );
                }
            }

            WorkerToMain::OrbitStored { orbit_id } => {
                // Check if this is a cell orbit (Phase 8)
                if self.cell_orbit_confirmations.contains_key(&orbit_id) {
                    // Log worker confirmation for cell orbit
                    web_sys::console::log_1(
                        &format!(
                            "[WorkerPool] Worker {} stored orbit #{}",
                            worker_id, orbit_id
                        )
                        .into(),
                    );

                    // Track confirmation
                    if let Some(confirmations) = self.cell_orbit_confirmations.get_mut(&orbit_id) {
                        confirmations.insert(worker_id);

                        // Check if all workers have confirmed
                        let all_confirmed = self
                            .initialized_workers
                            .iter()
                            .all(|&id| confirmations.contains(&id));

                        if all_confirmed {
                            web_sys::console::log_1(
                                &format!(
                                    "[WorkerPool] Phase 8: All {} workers confirmed orbit #{}",
                                    confirmations.len(),
                                    orbit_id
                                )
                                .into(),
                            );
                        }
                    }
                    return;
                }

                // Main perturbation orbit handling
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

                // Calculate delta_c_origin for this tile's top-left pixel using BigFloat
                //
                // PRECISION FIX: At deep zoom (10^14+), computing tile fractal coords
                // then subtracting c_ref causes catastrophic cancellation because both
                // values are nearly identical when converted to f64.
                //
                // Since c_ref IS the viewport center, we compute delta directly:
                //   delta = (tile_pos - canvas_center) * pixel_size
                // This avoids adding a tiny delta to a large coordinate then subtracting.
                let (canvas_width, canvas_height) = self.canvas_size;
                let precision = viewport.width.precision_bits();

                // Tile top-left pixel offset from canvas center in normalized coords [-0.5, 0.5]
                let norm_x = tile.x as f64 / canvas_width as f64 - 0.5;
                let norm_y = tile.y as f64 / canvas_height as f64 - 0.5;

                // Delta from reference point (viewport center) computed using BigFloat
                let norm_x_bf = BigFloat::with_precision(norm_x, precision);
                let norm_y_bf = BigFloat::with_precision(norm_y, precision);
                let delta_c_origin = (
                    norm_x_bf.mul(&viewport.width),
                    norm_y_bf.mul(&viewport.height),
                );

                // Serialize BigFloat deltas to JSON strings
                let delta_c_origin_json =
                    serde_json::to_string(&delta_c_origin).unwrap_or_default();
                let delta_c_step_json =
                    serde_json::to_string(&self.perturbation.delta_step).unwrap_or_default();

                // Get BigFloat threshold from config (default to 1024 bits = ~10^300 zoom)
                let bigfloat_threshold_bits = get_config(&self.renderer_id)
                    .map(|c| c.bigfloat_threshold_bits)
                    .unwrap_or(1024);

                self.send_to_worker(
                    worker_id,
                    &MainToWorker::RenderTilePerturbation {
                        render_id: self.current_render_id,
                        tile,
                        orbit_id: self.perturbation.orbit_id,
                        delta_c_origin_json,
                        delta_c_step_json,
                        max_iterations: self.perturbation.max_iterations,
                        tau_sq: self.perturbation.tau_sq,
                        bigfloat_threshold_bits,
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
        self.glitched_tile_count = 0;
        self.glitched_tiles.clear();
        self.quadtree = None; // Clear quadtree for non-perturbation renders
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
        self.glitched_tile_count = 0;
        self.glitched_tiles.clear();
        self.perturbation.orbit_id = self.perturbation.orbit_id.wrapping_add(1);
        self.perturbation.workers_with_orbit.clear();
        self.perturbation.pending_orbit = None;

        // Clear cell orbit tracking (Phase 8) - new render starts fresh
        self.cell_orbits.clear();
        self.cell_orbit_ids.clear();
        self.cell_orbit_confirmations.clear();

        // Create quadtree for spatial tracking of glitched regions
        self.quadtree = Some(QuadtreeCell::new_root(canvas_size));

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
        // Get config for iteration and glitch parameters
        let config = get_config("mandelbrot");
        let max_iterations = calculate_max_iterations(
            zoom_exponent,
            config.map(|c| c.iteration_multiplier).unwrap_or(200.0),
            config.map(|c| c.iteration_power).unwrap_or(2.5),
        );

        // Calculate delta step (per pixel in fractal space) using BigFloat for precision
        let precision = viewport.width.precision_bits();
        let canvas_width_bf = BigFloat::with_precision(canvas_size.0 as f64, precision);
        let canvas_height_bf = BigFloat::with_precision(canvas_size.1 as f64, precision);
        let delta_step = (
            viewport.width.div(&canvas_width_bf),
            viewport.height.div(&canvas_height_bf),
        );

        self.perturbation.max_iterations = max_iterations;
        self.perturbation.delta_step = delta_step;
        self.perturbation.tau_sq = config.map(|c| c.tau_sq).unwrap_or(1e-6);
        // Calculate dc_max for BLA table construction
        self.perturbation.dc_max = calculate_dc_max(&viewport, canvas_size);

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
        self.quadtree = None;
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

    /// Subdivide quadtree cells that contain glitched tiles.
    ///
    /// Performs ONE level of subdivision: finds all current leaf cells that
    /// intersect with at least one glitched tile and subdivides them.
    /// Press "d" multiple times for deeper subdivision.
    pub fn subdivide_glitched_cells(&mut self) {
        let Some(quadtree) = &mut self.quadtree else {
            web_sys::console::log_1(&"[WorkerPool] No quadtree exists, cannot subdivide".into());
            return;
        };

        if self.glitched_tiles.is_empty() {
            web_sys::console::log_1(&"[WorkerPool] No glitched tiles to subdivide for".into());
            return;
        }

        // Helper to convert PixelRect to quadtree Bounds for intersection check
        fn tile_to_bounds(tile: &PixelRect) -> crate::workers::quadtree::Bounds {
            crate::workers::quadtree::Bounds::new(tile.x, tile.y, tile.width, tile.height)
        }

        let mut subdivided_count = 0;

        // Subdivide only CURRENT leaves (one level at a time).
        // We do NOT recurse into newly created children - that's what
        // multiple "d" presses are for.
        fn subdivide_leaves_once(
            cell: &mut QuadtreeCell,
            glitched_tiles: &[PixelRect],
            subdivided_count: &mut u32,
        ) {
            // Check if any glitched tile intersects this cell
            let has_glitched = glitched_tiles
                .iter()
                .any(|tile| cell.bounds.intersects(&tile_to_bounds(tile)));

            if !has_glitched {
                return;
            }

            if cell.is_leaf() {
                // This is a leaf with glitched tiles - subdivide it
                if cell.subdivide() {
                    *subdivided_count += 1;
                }
                // Do NOT recurse into new children - one level only
                return;
            }

            // Only recurse into EXISTING children (cells that were already subdivided)
            if let Some(children) = &mut cell.children {
                for child in children.iter_mut() {
                    subdivide_leaves_once(child, glitched_tiles, subdivided_count);
                }
            }
        }

        subdivide_leaves_once(quadtree, &self.glitched_tiles, &mut subdivided_count);

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Subdivided {} cells with glitched tiles",
                subdivided_count
            )
            .into(),
        );

        // Log the new structure - collect all leaves and their glitch counts
        let mut leaves = Vec::new();
        quadtree.collect_leaves(&mut leaves);

        for leaf in &leaves {
            let glitched_count = self
                .glitched_tiles
                .iter()
                .filter(|tile| leaf.bounds.intersects(&tile_to_bounds(tile)))
                .count();

            if glitched_count > 0 {
                let b = &leaf.bounds;
                web_sys::console::log_1(
                    &format!(
                        "[WorkerPool] Cell ({},{})-({},{}): {} glitched tiles",
                        b.x,
                        b.y,
                        b.x + b.width,
                        b.y + b.height,
                        glitched_count
                    )
                    .into(),
                );
            }
        }

        // Phase 7: Compute reference orbits for cell centers
        self.compute_orbits_for_glitched_cells();
    }

    /// Compute reference orbits for cells containing glitched tiles.
    ///
    /// For each leaf cell with glitched tiles:
    /// 1. Computes the cell center in fractal coordinates
    /// 2. Computes a ReferenceOrbit at that point
    /// 3. Stores the orbit for later distribution to workers (Phase 8)
    fn compute_orbits_for_glitched_cells(&mut self) {
        let Some(viewport) = self.current_viewport.clone() else {
            web_sys::console::log_1(
                &"[WorkerPool] No viewport available, cannot compute cell center orbits".into(),
            );
            return;
        };

        let Some(quadtree) = &self.quadtree else {
            return;
        };

        // Helper to convert PixelRect to quadtree Bounds for intersection check
        fn tile_to_bounds(tile: &PixelRect) -> Bounds {
            Bounds::new(tile.x, tile.y, tile.width, tile.height)
        }

        // Collect leaves with glitched tiles
        let mut leaves = Vec::new();
        quadtree.collect_leaves(&mut leaves);

        let precision_bits = viewport.precision_bits();

        // Compute max_iterations from zoom level
        let zoom_exponent = viewport.width.to_f64().abs().log2().abs();
        let config = get_config("mandelbrot");
        let max_iterations = calculate_max_iterations(
            zoom_exponent,
            config.map(|c| c.iteration_multiplier).unwrap_or(200.0),
            config.map(|c| c.iteration_power).unwrap_or(2.5),
        );

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Computing orbits: zoom_exp={:.1}, max_iter={}, precision={}",
                zoom_exponent, max_iterations, precision_bits
            )
            .into(),
        );

        let start_time = performance_now();
        let mut computed_count = 0;

        for leaf in &leaves {
            let has_glitched = self
                .glitched_tiles
                .iter()
                .any(|tile| leaf.bounds.intersects(&tile_to_bounds(tile)));

            if !has_glitched {
                continue;
            }

            // Cell key for storage
            let cell_key = (
                leaf.bounds.x,
                leaf.bounds.y,
                leaf.bounds.width,
                leaf.bounds.height,
            );

            // Skip if we already computed an orbit for this cell
            if self.cell_orbits.contains_key(&cell_key) {
                continue;
            }

            // Compute cell center in pixel coordinates
            let center_px_x = leaf.bounds.x as f64 + leaf.bounds.width as f64 / 2.0;
            let center_px_y = leaf.bounds.y as f64 + leaf.bounds.height as f64 / 2.0;

            // Convert to fractal coordinates
            let (c_ref_x, c_ref_y) = pixel_to_fractal(
                center_px_x,
                center_px_y,
                &viewport,
                self.canvas_size,
                precision_bits,
            );

            // Compute the reference orbit
            let c_ref = (c_ref_x, c_ref_y);
            let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

            // Log the orbit details
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Cell ({},{})-({},{}): c_ref=({:.6e}, {:.6e}), escaped_at={:?}, orbit_len={}",
                    leaf.bounds.x,
                    leaf.bounds.y,
                    leaf.bounds.x + leaf.bounds.width,
                    leaf.bounds.y + leaf.bounds.height,
                    orbit.c_ref.0,
                    orbit.c_ref.1,
                    orbit.escaped_at,
                    orbit.orbit.len()
                )
                .into(),
            );

            // Store the orbit
            self.cell_orbits.insert(cell_key, orbit);
            computed_count += 1;
        }

        let elapsed = performance_now() - start_time;

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 7: Computed {} reference orbits in {:.1}ms (total stored: {})",
                computed_count,
                elapsed,
                self.cell_orbits.len()
            )
            .into(),
        );

        // Phase 8: Distribute computed orbits to workers
        self.broadcast_cell_orbits_to_workers();
    }

    /// Broadcast cell orbits to all workers (Phase 8).
    ///
    /// For each computed orbit that hasn't been assigned an orbit_id yet:
    /// 1. Assigns a unique orbit_id
    /// 2. Sends StoreReferenceOrbit to all workers
    /// 3. Initializes confirmation tracking
    fn broadcast_cell_orbits_to_workers(&mut self) {
        if self.cell_orbits.is_empty() {
            web_sys::console::log_1(&"[WorkerPool] Phase 8: No cell orbits to distribute".into());
            return;
        }

        let start_time = performance_now();
        let mut broadcast_count = 0;

        // Collect cell keys that need orbit_id assignment (can't mutate while iterating)
        let cells_without_id: Vec<(u32, u32, u32, u32)> = self
            .cell_orbits
            .keys()
            .filter(|key| !self.cell_orbit_ids.contains_key(*key))
            .cloned()
            .collect();

        for cell_key in cells_without_id {
            let Some(orbit) = self.cell_orbits.get(&cell_key) else {
                continue;
            };

            // Assign a unique orbit_id
            let orbit_id = self.cell_orbit_id_counter;
            self.cell_orbit_id_counter = self.cell_orbit_id_counter.wrapping_add(1);

            // Store the mapping
            self.cell_orbit_ids.insert(cell_key, orbit_id);

            // Initialize confirmation tracking
            self.cell_orbit_confirmations
                .insert(orbit_id, HashSet::new());

            // Broadcast to all workers
            let msg = MainToWorker::StoreReferenceOrbit {
                orbit_id,
                c_ref: orbit.c_ref,
                orbit: orbit.orbit.clone(),
                escaped_at: orbit.escaped_at,
            };

            for worker_id in 0..self.workers.len() {
                self.send_to_worker(worker_id, &msg);
            }

            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Phase 8: Broadcasting orbit #{} for cell ({},{})-({},{}) to {} workers",
                    orbit_id,
                    cell_key.0,
                    cell_key.1,
                    cell_key.0 + cell_key.2,
                    cell_key.1 + cell_key.3,
                    self.workers.len()
                )
                .into(),
            );

            broadcast_count += 1;
        }

        let elapsed = performance_now() - start_time;

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 8: Broadcast {} cell orbits to {} workers in {:.1}ms",
                broadcast_count,
                self.workers.len(),
                elapsed
            )
            .into(),
        );
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
