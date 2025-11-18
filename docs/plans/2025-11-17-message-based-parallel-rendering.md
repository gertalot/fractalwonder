# Message-Based Parallel Rendering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace SharedArrayBuffer-based parallel rendering with message-based worker coordination to eliminate race conditions and enable progressive rendering with data caching.

**Architecture:** Workers use request/response message protocol with transferable arrays for zero-copy data transfer. Main thread maintains work queue and caches computed fractal data for instant recolorization. Workers create AdaptiveMandelbrotRenderer once at startup.

**Tech Stack:** Rust + wasm-bindgen, Web Workers, Transferable Arrays, serde_json for message serialization

---

## Prerequisites

**Context:** See design document at `docs/plans/2025-11-17-message-based-parallel-rendering-design.md`

**Current state:**
- `fractalwonder-compute/src/worker.rs` - SharedArrayBuffer-based worker (to be replaced)
- `fractalwonder-ui/src/workers/worker_pool.rs` - WorkerPool using SharedArrayBuffer (to be replaced)
- `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs` - Renderer with polling (to be replaced)
- `fractalwonder-ui/src/rendering/tiling_canvas_renderer.rs` - Reference for caching pattern

**Approach:** Implement new message-based system alongside old one, test thoroughly, then swap.

---

## Task 1: Define Message Protocol Types

**Files:**
- Create: `fractalwonder-compute/src/messages.rs`
- Modify: `fractalwonder-compute/src/lib.rs` (add module export)

**Step 1: Create messages module**

Create `fractalwonder-compute/src/messages.rs`:

```rust
use fractalwonder_core::{MandelbrotData, PixelRect};
use serde::{Deserialize, Serialize};

/// Messages sent from worker to main thread
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker requests work assignment
    RequestWork {
        /// None = worker just started, will accept any work
        /// Some(id) = finished work for this render, wants more from same render
        render_id: Option<u32>,
    },

    /// Worker completed a tile
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        #[serde(skip)]  // Will be sent as transferable array
        data: Vec<MandelbrotData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error
    Error {
        render_id: Option<u32>,
        tile: Option<PixelRect>,
        error: String,
    },
}

/// Messages sent from main thread to worker
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Assign tile to render
    RenderTile {
        render_id: u32,
        viewport_json: String,
        tile: PixelRect,
        canvas_width: u32,
        canvas_height: u32,
    },

    /// No work available
    NoWork,

    /// Terminate worker
    Terminate,
}
```

**Step 2: Export messages module**

In `fractalwonder-compute/src/lib.rs`, add after existing exports:

```rust
pub mod messages;
pub use messages::{MainToWorker, WorkerToMain};
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-compute`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/messages.rs fractalwonder-compute/src/lib.rs
git commit -m "feat: define message protocol types for worker communication

Add WorkerToMain and MainToWorker message enums with serde support.
Foundation for message-based parallel rendering.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Implement New Worker Entry Point

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add new worker initialization function**

In `fractalwonder-compute/src/worker.rs`, add at the end of file:

```rust
use crate::{AdaptiveMandelbrotRenderer, MainToWorker, WorkerToMain};
use js_sys::Date;

/// New message-based worker initialization
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    // Create adaptive renderer once at startup
    let renderer = AdaptiveMandelbrotRenderer::new(1e10);

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Err(err) = handle_worker_message(&renderer, e.data()) {
            web_sys::console::error_1(&JsValue::from_str(&format!("Worker error: {:?}", err)));
        }
    }) as Box<dyn FnMut(_)>);

    let global: DedicatedWorkerGlobalScope = js_sys::global()
        .dyn_into()
        .expect("Failed to get worker global scope");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Request work immediately (no "Ready" message)
    send_message(&WorkerToMain::RequestWork { render_id: None });
}

fn handle_worker_message(
    renderer: &AdaptiveMandelbrotRenderer,
    data: JsValue,
) -> Result<(), JsValue> {
    let msg_str = data
        .as_string()
        .ok_or_else(|| JsValue::from_str("Message data is not a string"))?;

    let msg: MainToWorker = serde_json::from_str(&msg_str)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse message: {}", e)))?;

    match msg {
        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
            canvas_width,
            canvas_height,
        } => {
            handle_render_tile(renderer, render_id, viewport_json, tile, canvas_width, canvas_height)?;
        }
        MainToWorker::NoWork => {
            // Render complete, go idle
        }
        MainToWorker::Terminate => {
            let global: DedicatedWorkerGlobalScope = js_sys::global()
                .dyn_into()
                .expect("Failed to get worker global scope");
            global.close();
        }
    }

    Ok(())
}

fn handle_render_tile(
    renderer: &AdaptiveMandelbrotRenderer,
    render_id: u32,
    viewport_json: String,
    tile: PixelRect,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<(), JsValue> {
    let start_time = Date::now();

    // Parse viewport
    let viewport: Viewport<BigFloat> = serde_json::from_str(&viewport_json).map_err(|e| {
        let err_msg = format!("Failed to parse viewport: {}", e);
        send_error(Some(render_id), Some(tile), &err_msg);
        JsValue::from_str(&err_msg)
    })?;

    // Render tile
    let tile_data = renderer.render(&viewport, tile, (canvas_width, canvas_height));

    let compute_time_ms = Date::now() - start_time;

    // Send result
    send_tile_complete(render_id, tile, tile_data, compute_time_ms);

    // Request next work
    send_message(&WorkerToMain::RequestWork {
        render_id: Some(render_id),
    });

    Ok(())
}

fn send_message(msg: &WorkerToMain) {
    if let Ok(json) = serde_json::to_string(msg) {
        let global: DedicatedWorkerGlobalScope = js_sys::global()
            .dyn_into()
            .expect("Failed to get worker global scope");
        global
            .post_message(&JsValue::from_str(&json))
            .ok();
    }
}

fn send_tile_complete(
    render_id: u32,
    tile: PixelRect,
    data: Vec<MandelbrotData>,
    compute_time_ms: f64,
) {
    let msg = WorkerToMain::TileComplete {
        render_id,
        tile,
        data: vec![], // Will be sent separately as transferable
        compute_time_ms,
    };

    if let Ok(json) = serde_json::to_string(&msg) {
        // TODO: Implement transferable array sending in next task
        let global: DedicatedWorkerGlobalScope = js_sys::global()
            .dyn_into()
            .expect("Failed to get worker global scope");

        // For now, serialize data in JSON (will optimize with transferable arrays later)
        let msg_with_data = WorkerToMain::TileComplete {
            render_id,
            tile,
            data,
            compute_time_ms,
        };

        if let Ok(json_with_data) = serde_json::to_string(&msg_with_data) {
            global
                .post_message(&JsValue::from_str(&json_with_data))
                .ok();
        }
    }
}

fn send_error(render_id: Option<u32>, tile: Option<PixelRect>, error: &str) {
    send_message(&WorkerToMain::Error {
        render_id,
        tile,
        error: error.to_string(),
    });
}
```

**Step 2: Add necessary imports**

At top of `fractalwonder-compute/src/worker.rs`, add:

```rust
use fractalwonder_core::{BigFloat, Viewport};
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-compute`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat: implement message-based worker with AdaptiveMandelbrotRenderer

Add init_message_worker() that:
- Creates AdaptiveMandelbrotRenderer once at startup (fixes precision bug)
- Handles RenderTile, NoWork, Terminate messages
- Sends RequestWork, TileComplete, Error messages
- Uses JSON serialization (transferable arrays in next task)

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Create Worker Pool with Message Handling

**Files:**
- Create: `fractalwonder-ui/src/workers/message_worker_pool.rs`
- Modify: `fractalwonder-ui/src/workers/mod.rs`

**Step 1: Create message worker pool module**

Create `fractalwonder-ui/src/workers/message_worker_pool.rs`:

```rust
use fractalwonder_compute::{MainToWorker, WorkerToMain};
use fractalwonder_core::{BigFloat, MandelbrotData, PixelRect, Viewport};
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

#[derive(Clone)]
pub struct TileResult {
    pub tile: PixelRect,
    pub data: Vec<MandelbrotData>,
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
                        pool_clone.borrow_mut().handle_worker_message(worker_id, msg);
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

        Ok(Rc::try_unwrap(pool).unwrap().into_inner())
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
                render_id,
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
        let a_dist_sq = (a_center_x - canvas_center_x).powi(2) + (a_center_y - canvas_center_y).powi(2);

        let b_center_x = b.tile.x as f64 + b.tile.width as f64 / 2.0;
        let b_center_y = b.tile.y as f64 + b.tile.height as f64 / 2.0;
        let b_dist_sq = (b_center_x - canvas_center_x).powi(2) + (b_center_y - canvas_center_y).powi(2);

        a_dist_sq
            .partial_cmp(&b_dist_sq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles.into_iter().collect()
}
```

**Step 2: Export in mod.rs**

In `fractalwonder-ui/src/workers/mod.rs`, add:

```rust
pub mod message_worker_pool;
pub use message_worker_pool::{MessageWorkerPool, TileResult};
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs fractalwonder-ui/src/workers/mod.rs
git commit -m "feat: implement MessageWorkerPool with request/response pattern

Add worker pool that:
- Creates N workers based on hardware concurrency
- Handles RequestWork, TileComplete, Error messages
- Maintains work queue with center-first tile ordering
- Tracks render_id for cancellation
- Provides on_tile_complete callback

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Implement Message-Based Parallel Renderer with Caching

**Files:**
- Create: `fractalwonder-ui/src/rendering/message_parallel_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create renderer module**

Create `fractalwonder-ui/src/rendering/message_parallel_renderer.rs`:

```rust
use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::{MessageWorkerPool, TileResult};
use fractalwonder_core::{AppData, BigFloat, MandelbrotData, Point, Rect, Viewport};
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
    data: Vec<MandelbrotData>,
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
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
}

impl MessageParallelRenderer {
    pub fn new(colorizer: Colorizer<AppData>, tile_size: u32) -> Result<Self, JsValue> {
        let canvas = Rc::new(RefCell::new(None));
        let canvas_clone = Rc::clone(&canvas);
        let colorizer_clone = colorizer;
        let cached_state = Arc::new(Mutex::new(CachedState::default()));
        let cached_state_clone = Arc::clone(&cached_state);

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
                            cache.data[cache_idx] = tile_result.data[tile_idx];
                        }
                    }
                }

                drop(cache);

                // Draw tile immediately
                if let Err(e) = draw_tile(canvas, &tile_result, &colorizer_clone) {
                    web_sys::console::error_1(&e);
                }
            }
        };

        let worker_pool = Rc::new(RefCell::new(MessageWorkerPool::new(on_tile_complete)?));

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
        })
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.borrow().worker_count()
    }

    fn recolorize_from_cache(&self, render_id: u32, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let cache = self.cached_state.lock().unwrap();

        if cache.render_id.load(Ordering::SeqCst) != render_id {
            return Ok(()); // Cancelled
        }

        let width = canvas.width();
        let height = canvas.height();

        let colors: Vec<u8> = cache
            .data
            .iter()
            .flat_map(|data| {
                let app_data = AppData::MandelbrotData(*data);
                let (r, g, b, a) = (self.colorizer)(&app_data);
                [r, g, b, a]
            })
            .collect();

        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("No 2d context"))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        let image_data = web_sys::ImageData::new_with_u8_clamped_array(
            wasm_bindgen::Clamped(&colors),
            width,
        )?;

        context.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }
}

impl Clone for MessageParallelRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
            canvas: Rc::clone(&self.canvas),
            cached_state: Arc::clone(&self.cached_state),
        }
    }
}

impl CanvasRenderer for MessageParallelRenderer {
    type Scalar = f64;
    type Data = AppData;

    fn set_renderer(
        &mut self,
        _renderer: Box<dyn fractalwonder_compute::Renderer<Scalar = Self::Scalar, Data = Self::Data>>,
    ) {
        // Not used - workers have their own AdaptiveMandelbrotRenderer
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<Self::Data>) {
        self.colorizer = colorizer;
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
                BigFloat::from(viewport.center.x()),
                BigFloat::from(viewport.center.y()),
            ),
            viewport.zoom,
        );

        if cache.viewport.as_ref() == Some(&viewport_bf) && cache.canvas_size == Some((width, height)) {
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
            cache.data.resize((width * height) as usize, MandelbrotData::default());
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
            let app_data = AppData::MandelbrotData(*data);
            let (r, g, b, a) = colorizer(&app_data);
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
```

**Step 2: Export in rendering mod.rs**

In `fractalwonder-ui/src/rendering/mod.rs`, add:

```rust
pub mod message_parallel_renderer;
pub use message_parallel_renderer::MessageParallelRenderer;
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/message_parallel_renderer.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat: implement MessageParallelRenderer with caching

Add renderer that:
- Integrates MessageWorkerPool with canvas rendering
- Caches fractal data for instant recolorization
- Detects viewport changes (recompute vs recolorize)
- Draws tiles progressively as they complete
- Converts f64 viewport to BigFloat for workers

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Create Worker JavaScript Entry Point

**Files:**
- Create: `fractalwonder-ui/public/message-compute-worker.js`

**Step 1: Create worker entry point**

Create `fractalwonder-ui/public/message-compute-worker.js`:

```javascript
// Load the WASM compute module
importScripts('./fractalwonder-compute.js');

// Initialize when WASM module is ready
self.onmessage = async (e) => {
  // Import and initialize the WASM module
  const { init_message_worker, default: init } = wasm_bindgen;

  try {
    // Initialize WASM
    await init('./fractalwonder-compute_bg.wasm');

    // Call worker initialization (sets up message handlers)
    init_message_worker();

    console.log('Worker initialized successfully');
  } catch (err) {
    console.error('Worker initialization failed:', err);
    self.postMessage(JSON.stringify({
      type: 'Error',
      render_id: null,
      tile: null,
      error: `Initialization failed: ${err.message}`
    }));
  }
};
```

**Step 2: Verify file created**

Run: `ls -la fractalwonder-ui/public/message-compute-worker.js`
Expected: File exists

**Step 3: Commit**

```bash
git add fractalwonder-ui/public/message-compute-worker.js
git commit -m "feat: add JavaScript entry point for message-based worker

Creates worker that loads WASM module and calls init_message_worker().

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Test Message-Based Renderer

**Files:**
- Modify: `fractalwonder-ui/src/app.rs` (temporarily switch renderer)

**Step 1: Comment out old renderer, add new one**

In `fractalwonder-ui/src/app.rs`, find the `CanvasRendererHolder` enum and replace:

```rust
#[derive(Clone)]
enum CanvasRendererHolder {
    Parallel(ParallelCanvasRenderer),
    // MessageParallel(MessageParallelRenderer),  // Add this after testing
}
```

With:

```rust
#[derive(Clone)]
enum CanvasRendererHolder {
    // Parallel(ParallelCanvasRenderer),  // Old SharedArrayBuffer version
    MessageParallel(MessageParallelRenderer),
}
```

**Step 2: Update renderer methods**

Update all match statements in `CanvasRendererHolder` impl to use `MessageParallel`:

```rust
impl CanvasRendererHolder {
    fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.render(viewport, canvas)
    }

    fn natural_bounds(&self) -> crate::rendering::Rect<f64> {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.natural_bounds()
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.set_colorizer(colorizer)
    }

    fn cancel_render(&self) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.cancel_render()
    }
}
```

**Step 3: Update renderer creation**

Find `create_parallel_renderer` and update:

```rust
fn create_message_parallel_renderer(
    colorizer: Colorizer<AppData>,
) -> Result<MessageParallelRenderer, JsValue> {
    MessageParallelRenderer::new(colorizer, 128)
}
```

Find renderer creation in `App` component and update:

```rust
let initial_canvas_renderer = CanvasRendererHolder::MessageParallel(
    create_message_parallel_renderer(initial_colorizer)
        .expect("Failed to create message parallel renderer"),
);
```

And in the effect that creates new renderers:

```rust
let new_canvas_renderer = CanvasRendererHolder::MessageParallel(
    create_message_parallel_renderer(colorizer)
        .expect("Failed to create message parallel renderer"),
);
```

**Step 4: Add import**

At top of `app.rs`, add:

```rust
use crate::rendering::MessageParallelRenderer;
```

**Step 5: Build and test**

Run: `trunk build`
Expected: Build succeeds

Open browser to `http://localhost:8080` and verify:
- No black rectangles
- Progressive rendering (center tiles first)
- Smooth pan/zoom cancellation
- No console errors

**Step 6: Test recolorization**

In browser:
1. Wait for render to complete
2. Change color scheme
3. Verify: Instant recolor with "RECOLORIZE from cache" in console

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "test: switch to MessageParallelRenderer for testing

Temporarily replace SharedArrayBuffer renderer with message-based renderer.
Test progressive rendering, cancellation, and recolorization.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: Fix Worker Entry Point Issue

**Context:** The worker JavaScript may need adjustment after testing reveals initialization timing.

**Step 1: Test worker initialization in browser**

Open browser console and check for:
- "Worker initialized successfully" messages
- No "Worker initialization failed" errors

**Step 2: If issues found, update worker entry point**

If workers fail to initialize, update `message-compute-worker.js`:

```javascript
// Load the WASM compute module
importScripts('./fractalwonder-compute.js');

// Initialize immediately on worker start
(async () => {
  try {
    // Initialize WASM
    await wasm_bindgen('./fractalwonder-compute_bg.wasm');

    // Call worker initialization (sets up message handlers and requests work)
    wasm_bindgen.init_message_worker();

    console.log('Worker initialized successfully');
  } catch (err) {
    console.error('Worker initialization failed:', err);
    self.postMessage(JSON.stringify({
      type: 'Error',
      render_id: null,
      tile: null,
      error: `Initialization failed: ${err.message}`
    }));
  }
})();
```

**Step 3: Rebuild and retest**

Run: `trunk build`
Test: Verify workers initialize and render completes

**Step 4: Commit if changes made**

```bash
git add fractalwonder-ui/public/message-compute-worker.js
git commit -m "fix: initialize worker immediately on start

Change from onmessage trigger to immediate IIFE execution.
Ensures worker sends RequestWork without waiting.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: Remove Old Parallel Renderer Code

**Files:**
- Remove: Old `ParallelCanvasRenderer` code
- Modify: `fractalwonder-ui/src/app.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`
- Remove: `fractalwonder-compute/src/worker.rs` (old functions)
- Remove: `fractalwonder-ui/src/workers/worker_pool.rs` (old version)

**Step 1: Remove old worker functions**

In `fractalwonder-compute/src/worker.rs`, remove:
- `process_render_request()` function
- `compute_tiles()` function
- `generate_tiles()` function
- `write_tile_to_buffer()` function
- Keep only `init_worker()` (for compatibility) and `init_message_worker()`

**Step 2: Remove SharedArrayBuffer types**

In `fractalwonder-compute/src/lib.rs`, keep `SharedBufferLayout` for now (might be used elsewhere).

**Step 3: Clean up app.rs**

In `fractalwonder-ui/src/app.rs`:
- Remove `ParallelCanvasRenderer` import
- Remove `create_parallel_renderer` function
- Remove commented-out `Parallel` variant from enum
- Rename `MessageParallel` to `Parallel`
- Rename `create_message_parallel_renderer` to `create_parallel_renderer`

**Step 4: Clean up rendering mod**

In `fractalwonder-ui/src/rendering/mod.rs`:
- Keep old `parallel_canvas_renderer` module commented for reference
- Keep exporting `MessageParallelRenderer`

**Step 5: Remove old worker pool**

In `fractalwonder-ui/src/workers/mod.rs`:
- Comment out old `worker_pool` module (keep for reference)
- Keep exporting `MessageWorkerPool`

**Step 6: Build and verify**

Run: `cargo check --workspace`
Expected: No errors

Run: `trunk build`
Expected: Build succeeds

Test in browser: All functionality works

**Step 7: Commit**

```bash
git add -A
git commit -m "refactor: remove old SharedArrayBuffer parallel renderer

Remove old worker implementation:
- Old worker.rs functions (process_render_request, etc.)
- Old WorkerPool with SharedArrayBuffer
- Old ParallelCanvasRenderer with polling

Keep message-based implementation as new standard.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 9: Add Error Handling and Retry Logic

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs`

**Step 1: Add retry tracking**

In `MessageWorkerPool` struct, add:

```rust
use std::collections::HashMap;

pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    pending_tiles: VecDeque<TileRequest>,
    failed_tiles: HashMap<(u32, u32), u32>, // (x, y) -> retry_count
    current_render_id: u32,
    // ... rest
}
```

Update `new()` to initialize:

```rust
failed_tiles: HashMap::new(),
```

**Step 2: Handle errors with retry**

In `handle_worker_message`, update Error case:

```rust
WorkerToMain::Error {
    render_id,
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
                tile.x, tile.y, *retry_count + 1, error
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
            "Worker error: {}", error
        )));
    }
}
```

**Step 3: Clear retry tracking on new render**

In `start_render`, add:

```rust
self.failed_tiles.clear();
```

**Step 4: Test error handling**

Manually test by introducing an error in worker (e.g., divide by zero).
Verify retry logic works and tile is retried once.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "feat: add error handling with single retry for failed tiles

Track failed tiles and retry once before giving up.
Clear retry tracking on new render.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 10: Optimize with Transferable Arrays

**Note:** This is an optimization task. If JSON serialization performance is acceptable, this can be deferred.

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs`

**Step 1: Update worker to send transferable array**

In `fractalwonder-compute/src/worker.rs`, update `send_tile_complete`:

```rust
fn send_tile_complete(
    render_id: u32,
    tile: PixelRect,
    data: Vec<MandelbrotData>,
    compute_time_ms: f64,
) {
    // Create transferable array from MandelbrotData
    let mut buffer = Vec::with_capacity(data.len() * 2);
    for pixel in &data {
        buffer.push(pixel.iterations);
        buffer.push(if pixel.escaped { 1 } else { 0 });
    }

    let array = js_sys::Uint32Array::from(&buffer[..]);
    let array_buffer = array.buffer();

    // Send metadata as JSON
    let msg = serde_json::json!({
        "type": "TileComplete",
        "render_id": render_id,
        "tile": {
            "x": tile.x,
            "y": tile.y,
            "width": tile.width,
            "height": tile.height,
        },
        "compute_time_ms": compute_time_ms,
    });

    let global: DedicatedWorkerGlobalScope = js_sys::global()
        .dyn_into()
        .expect("Failed to get worker global scope");

    // Create message with transferable
    let msg_obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &msg_obj,
        &JsValue::from_str("metadata"),
        &JsValue::from_str(&msg.to_string()),
    ).ok();
    js_sys::Reflect::set(
        &msg_obj,
        &JsValue::from_str("data"),
        &array,
    ).ok();

    // Post with transfer
    let transfer_array = js_sys::Array::new();
    transfer_array.push(&array_buffer);

    global.post_message_with_transfer(&msg_obj, &transfer_array).ok();
}
```

**Step 2: Update pool to receive transferable array**

In message handler, parse split format:

```rust
let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
    let data = e.data();

    // Check if it's the new format with transferable
    if let Ok(obj) = data.dyn_into::<js_sys::Object>() {
        if let Ok(metadata_val) = js_sys::Reflect::get(&obj, &JsValue::from_str("metadata")) {
            if let Some(metadata_str) = metadata_val.as_string() {
                // Parse metadata
                // Extract transferable data array
                // Reconstruct MandelbrotData
                // Continue with existing logic
            }
        }
    }

    // Fall back to old JSON format
    if let Some(msg_str) = data.as_string() {
        // Existing logic
    }
}));
```

**Step 3: Test performance**

Benchmark render times before/after.
Verify transferable arrays improve performance (should be faster for large tiles).

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "perf: use transferable arrays for zero-copy tile data transfer

Send pixel data as Uint32Array with transfer instead of JSON.
Significant performance improvement for large tiles.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 11: Final Testing and Documentation

**Files:**
- Update: `README.md` or relevant docs
- Test: Various zoom levels and canvas sizes

**Step 1: Comprehensive testing**

Test scenarios:
1. Low zoom (f64 precision): Fast rendering, smooth
2. High zoom (BigFloat precision): No blockiness, correct precision
3. Pan/zoom rapidly: Clean cancellation, no black rectangles
4. Change color scheme: Instant recolorization
5. Resize window: Correct recomputation
6. Multiple back-to-back renders: No memory leaks

**Step 2: Verify success criteria**

From design doc:
- ✅ No black rectangles at any zoom level
- ✅ Smooth progressive rendering (tiles appear center-first)
- ✅ Instant recolorization when changing color schemes
- ✅ Clean cancellation (pan/zoom immediately starts new render)
- ✅ Correct precision at extreme zoom (no blockiness)
- ✅ Performance comparable to or better than current system
- ✅ No memory leaks over extended use
- ✅ Clean error messages for worker failures

**Step 3: Update documentation**

Add to `CLAUDE.md` or create `docs/architecture/parallel-rendering.md`:

```markdown
## Parallel Rendering Architecture

Uses message-based worker coordination for fractal computation:

- **Workers**: Create AdaptiveMandelbrotRenderer, compute fractal data
- **Main thread**: Manages work queue, colorizes, caches results
- **Protocol**: Request/response pattern with transferable arrays
- **Cancellation**: Increment render_id, workers ignore stale results
- **Caching**: Full canvas data stored for instant recolorization

See `docs/plans/2025-11-17-message-based-parallel-rendering-design.md` for details.
```

**Step 4: Commit documentation**

```bash
git add docs/architecture/parallel-rendering.md
git commit -m "docs: add parallel rendering architecture overview

Document message-based worker coordination system.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Success Criteria

Implementation complete when:

- ✅ All tasks committed to git
- ✅ `cargo check --workspace` passes
- ✅ `trunk build` succeeds
- ✅ Browser testing shows:
  - No black rectangles
  - Progressive center-first rendering
  - Instant recolorization
  - Clean cancellation
  - Correct precision at all zoom levels
- ✅ No console errors during normal operation
- ✅ Documentation updated

---

## Notes for Implementation

**Testing focus:**
- Watch browser console for errors
- Test extreme zoom levels (check adaptive renderer works)
- Test rapid pan/zoom (check cancellation)
- Test color scheme changes (check caching)

**Common issues:**
- Worker initialization timing (Step 7 addresses this)
- Message serialization errors (check JSON format)
- Cache index out of bounds (verify canvas size tracking)

**Performance:**
- Should see ~240ms for 1920×1080 canvas on 8-core machine
- Center tiles should appear first (better UX)
- Color scheme changes should be instant

**Reference implementations:**
- `tiling_canvas_renderer.rs` - Caching pattern
- Old `worker.rs` - Worker structure (being replaced)
