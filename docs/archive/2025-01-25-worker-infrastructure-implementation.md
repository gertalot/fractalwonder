# Worker Infrastructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move fractal computation from main thread to Web Workers for parallel rendering.

**Architecture:** Workers load separate WASM instances via JS loader, communicate with main thread via JSON messages. Pull-based work distribution where workers request tiles when idle. Terminate-and-recreate cancellation for immediate responsiveness.

**Tech Stack:** Rust/WASM, wasm-bindgen, web-sys, serde_json, Leptos

---

## Task 1: Add Serialize/Deserialize to ComputeData

**Files:**
- Modify: `fractalwonder-core/src/compute_data.rs`

**Step 1: Add serde derives to TestImageData**

```rust
// fractalwonder-core/src/compute_data.rs line 3-6
use serde::{Deserialize, Serialize};

/// Data computed for a test image pixel.
/// All fields are bools derived from normalized coordinate comparisons.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestImageData {
```

**Step 2: Add serde derives to MandelbrotData**

```rust
// fractalwonder-core/src/compute_data.rs line 36-38
/// Data computed for a Mandelbrot pixel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MandelbrotData {
```

**Step 3: Add serde derives to ComputeData enum**

```rust
// fractalwonder-core/src/compute_data.rs line 47-49
/// Unified enum for all compute results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComputeData {
```

**Step 4: Run tests to verify serialization works**

Run: `cargo test -p fractalwonder-core`
Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-core/src/compute_data.rs
git commit -m "feat(core): add Serialize/Deserialize to ComputeData types"
```

---

## Task 2: Create Message Types in Core

**Files:**
- Create: `fractalwonder-core/src/messages.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Create messages.rs with message enums**

```rust
// fractalwonder-core/src/messages.rs
use crate::{ComputeData, PixelRect};
use serde::{Deserialize, Serialize};

/// Messages sent from main thread to worker.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Initialize worker with specified renderer type.
    Initialize { renderer_id: String },

    /// Render a tile. viewport_json is JSON-serialized Viewport to preserve BigFloat precision.
    RenderTile {
        render_id: u32,
        viewport_json: String,
        tile: PixelRect,
    },

    /// No work available - worker should idle.
    NoWork,

    /// Terminate worker.
    Terminate,
}

/// Messages sent from worker to main thread.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker is loaded and ready for initialization.
    Ready,

    /// Worker requests work assignment.
    /// render_id is None after Initialize, Some(id) after completing work for that render.
    RequestWork { render_id: Option<u32> },

    /// Worker completed a tile.
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error.
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_to_worker_initialize_roundtrip() {
        let msg = MainToWorker::Initialize {
            renderer_id: "mandelbrot".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Initialize""#));
        assert!(json.contains(r#""renderer_id":"mandelbrot""#));

        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::Initialize { renderer_id } => assert_eq!(renderer_id, "mandelbrot"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn main_to_worker_render_tile_roundtrip() {
        let msg = MainToWorker::RenderTile {
            render_id: 42,
            viewport_json: r#"{"center":...}"#.to_string(),
            tile: PixelRect::new(10, 20, 64, 64),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::RenderTile { render_id, tile, .. } => {
                assert_eq!(render_id, 42);
                assert_eq!(tile.x, 10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn worker_to_main_ready_roundtrip() {
        let msg = WorkerToMain::Ready;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Ready""#));

        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WorkerToMain::Ready));
    }

    #[test]
    fn worker_to_main_tile_complete_roundtrip() {
        use crate::MandelbrotData;

        let msg = WorkerToMain::TileComplete {
            render_id: 1,
            tile: PixelRect::new(0, 0, 64, 64),
            data: vec![ComputeData::Mandelbrot(MandelbrotData {
                iterations: 100,
                max_iterations: 1000,
                escaped: true,
            })],
            compute_time_ms: 12.5,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::TileComplete { render_id, data, .. } => {
                assert_eq!(render_id, 1);
                assert_eq!(data.len(), 1);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
```

**Step 2: Export messages from lib.rs**

Add to `fractalwonder-core/src/lib.rs`:

```rust
pub mod messages;

// In the pub use section:
pub use messages::{MainToWorker, WorkerToMain};
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-core`
Expected: All tests pass including new message tests

**Step 4: Commit**

```bash
git add fractalwonder-core/src/messages.rs fractalwonder-core/src/lib.rs
git commit -m "feat(core): add worker message types MainToWorker and WorkerToMain"
```

---

## Task 3: Set Up Compute Crate for Worker WASM Build

**Files:**
- Modify: `fractalwonder-compute/Cargo.toml`
- Modify: `Cargo.toml` (workspace)

**Step 1: Update workspace Cargo.toml with new dependencies**

Check if these are already in workspace `[workspace.dependencies]`, add if missing:

```toml
wasm-bindgen = "0.2"
js-sys = "0.3"
console_error_panic_hook = "0.1"
```

**Step 2: Update compute crate Cargo.toml**

```toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-core = { path = "../fractalwonder-core" }
wasm-bindgen = { workspace = true }
js-sys = { workspace = true }
web-sys = { workspace = true, features = [
    "console",
    "DedicatedWorkerGlobalScope",
    "MessageEvent",
] }
serde = { workspace = true }
serde_json = { workspace = true }
console_error_panic_hook = { workspace = true }
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-compute`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add Cargo.toml fractalwonder-compute/Cargo.toml
git commit -m "build(compute): configure crate for worker WASM build"
```

---

## Task 4: Create Worker Entry Point

**Files:**
- Create: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create worker.rs**

```rust
// fractalwonder-compute/src/worker.rs
use crate::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{ComputeData, MainToWorker, Viewport, WorkerToMain};
use js_sys::Date;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Boxed renderer trait object for dynamic dispatch.
type BoxedRenderer = Box<dyn Renderer<Data = ComputeData>>;

fn create_renderer(renderer_id: &str) -> Option<BoxedRenderer> {
    match renderer_id {
        "test_image" => Some(Box::new(TestImageRendererWrapper)),
        "mandelbrot" => Some(Box::new(MandelbrotRendererWrapper { max_iterations: 1000 })),
        _ => None,
    }
}

// Wrapper to unify renderer output types
struct TestImageRendererWrapper;

impl Renderer for TestImageRendererWrapper {
    type Data = ComputeData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data> {
        TestImageRenderer
            .render(viewport, canvas_size)
            .into_iter()
            .map(ComputeData::TestImage)
            .collect()
    }
}

struct MandelbrotRendererWrapper {
    max_iterations: u32,
}

impl Renderer for MandelbrotRendererWrapper {
    type Data = ComputeData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data> {
        MandelbrotRenderer::new(self.max_iterations)
            .render(viewport, canvas_size)
            .into_iter()
            .map(ComputeData::Mandelbrot)
            .collect()
    }
}

fn post_message(msg: &WorkerToMain) {
    if let Ok(json) = serde_json::to_string(msg) {
        let global: web_sys::DedicatedWorkerGlobalScope =
            js_sys::global().dyn_into().expect("Not in worker context");
        let _ = global.post_message(&JsValue::from_str(&json));
    }
}

fn handle_message(renderer: &Rc<RefCell<Option<BoxedRenderer>>>, data: JsValue) {
    let Some(msg_str) = data.as_string() else {
        post_message(&WorkerToMain::Error {
            message: "Message is not a string".to_string(),
        });
        return;
    };

    let msg: MainToWorker = match serde_json::from_str(&msg_str) {
        Ok(m) => m,
        Err(e) => {
            post_message(&WorkerToMain::Error {
                message: format!("Failed to parse message: {}", e),
            });
            return;
        }
    };

    match msg {
        MainToWorker::Initialize { renderer_id } => {
            match create_renderer(&renderer_id) {
                Some(r) => {
                    *renderer.borrow_mut() = Some(r);
                    // Signal ready for work
                    post_message(&WorkerToMain::RequestWork { render_id: None });
                }
                None => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Unknown renderer: {}", renderer_id),
                    });
                }
            }
        }

        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
        } => {
            let borrowed = renderer.borrow();
            let Some(r) = borrowed.as_ref() else {
                post_message(&WorkerToMain::Error {
                    message: "Renderer not initialized".to_string(),
                });
                return;
            };

            // Parse viewport
            let viewport: Viewport = match serde_json::from_str(&viewport_json) {
                Ok(v) => v,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse viewport: {}", e),
                    });
                    return;
                }
            };

            let start_time = Date::now();

            // Render tile
            let data = r.render(&viewport, (tile.width, tile.height));

            let compute_time_ms = Date::now() - start_time;

            // Send result
            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            });

            // Request next work
            post_message(&WorkerToMain::RequestWork {
                render_id: Some(render_id),
            });
        }

        MainToWorker::NoWork => {
            // Idle - wait for next message
        }

        MainToWorker::Terminate => {
            let global: web_sys::DedicatedWorkerGlobalScope =
                js_sys::global().dyn_into().expect("Not in worker context");
            global.close();
        }
    }
}

/// Entry point called by worker JS loader.
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    let renderer: Rc<RefCell<Option<BoxedRenderer>>> = Rc::new(RefCell::new(None));

    let renderer_clone = Rc::clone(&renderer);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        handle_message(&renderer_clone, e.data());
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope =
        js_sys::global().dyn_into().expect("Not in worker context");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready for initialization
    post_message(&WorkerToMain::Ready);
}
```

**Step 2: Export worker module and Renderer trait**

Update `fractalwonder-compute/src/lib.rs`:

```rust
mod mandelbrot;
mod test_image;
pub mod worker;

use fractalwonder_core::Viewport;

pub use mandelbrot::MandelbrotRenderer;
pub use test_image::TestImageRenderer;

/// Renders a viewport to a grid of computed data.
pub trait Renderer {
    type Data;

    /// Render the given viewport at the specified canvas resolution.
    /// Returns a row-major Vec of pixel data (width * height elements).
    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::Viewport;

    #[test]
    fn test_image_renderer_produces_correct_size() {
        let renderer = TestImageRenderer;
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn test_image_renderer_origin_detected() {
        let renderer = TestImageRenderer;
        let vp = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 100));
        let center_idx = 50 * 100 + 50;
        assert!(result[center_idx].is_on_origin);
    }
}
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-compute`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/worker.rs fractalwonder-compute/src/lib.rs
git commit -m "feat(compute): add worker entry point init_message_worker"
```

---

## Task 5: Create JavaScript Worker Loader

**Files:**
- Create: `message-compute-worker.js`
- Modify: `index.html`

**Step 1: Create JS loader file**

```javascript
// message-compute-worker.js
import init, { init_message_worker } from './fractalwonder-compute.js';

async function run() {
    await init();
    init_message_worker();
}

run();
```

**Step 2: Add Trunk directive to index.html**

Add after the tailwind-css link:

```html
<link data-trunk rel="copy-file" href="message-compute-worker.js" />
<link data-trunk rel="rust" data-wasm-opt="z" data-bin="fractalwonder-compute" href="./fractalwonder-compute/Cargo.toml" data-type="worker" />
```

**Note:** The second directive tells Trunk to build the compute crate as a separate WASM module.

**Step 3: Verify Trunk builds both WASM modules**

Run: `trunk build`
Expected: `dist/` contains both main app WASM and `fractalwonder-compute.js` + `.wasm`

**Step 4: Commit**

```bash
git add message-compute-worker.js index.html
git commit -m "build: add worker JS loader and Trunk configuration"
```

---

## Task 6: Create WorkerPool in UI

**Files:**
- Create: `fractalwonder-ui/src/workers/mod.rs`
- Create: `fractalwonder-ui/src/workers/worker_pool.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create workers/mod.rs**

```rust
// fractalwonder-ui/src/workers/mod.rs
mod worker_pool;

pub use worker_pool::{TileResult, WorkerPool};
```

**Step 2: Create worker_pool.rs**

```rust
// fractalwonder-ui/src/workers/worker_pool.rs
use crate::rendering::RenderProgress;
use fractalwonder_core::{MainToWorker, PixelRect, Viewport, WorkerToMain, ComputeData};
use leptos::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

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
    initialized_count: usize,
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    current_viewport: Option<Viewport>,
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
    let mut workers = Vec::with_capacity(count);

    for worker_id in 0..count {
        let worker = Worker::new(WORKER_SCRIPT_PATH)?;

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
            initialized_count: 0,
            pending_tiles: VecDeque::new(),
            current_render_id: 0,
            current_viewport: None,
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
                // Track initialization
                if render_id.is_none() {
                    self.initialized_count += 1;
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
        if let Some(tile) = self.pending_tiles.pop_front() {
            let viewport_json = self
                .current_viewport
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok())
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

    pub fn start_render(&mut self, viewport: Viewport, tiles: Vec<PixelRect>) {
        self.current_render_id = self.current_render_id.wrapping_add(1);
        self.current_viewport = Some(viewport);
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
        // Terminate all workers
        for worker in &self.workers {
            worker.terminate();
        }

        self.pending_tiles.clear();
        self.initialized_count = 0;

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
```

**Step 3: Add workers module to lib.rs**

Add to `fractalwonder-ui/src/lib.rs`:

```rust
pub mod workers;
```

**Step 4: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/workers/
git add fractalwonder-ui/src/lib.rs
git commit -m "feat(ui): add WorkerPool for managing render workers"
```

---

## Task 7: Create ParallelRenderer

**Files:**
- Create: `fractalwonder-ui/src/rendering/parallel_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create parallel_renderer.rs**

```rust
// fractalwonder-ui/src/rendering/parallel_renderer.rs
use crate::config::FractalConfig;
use crate::rendering::canvas_utils::{draw_pixels_to_canvas, get_2d_context};
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles, tile_to_viewport};
use crate::rendering::RenderProgress;
use crate::workers::{TileResult, WorkerPool};
use fractalwonder_core::Viewport;
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Parallel renderer that distributes tiles across Web Workers.
pub struct ParallelRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>>,
}

impl ParallelRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> =
            Rc::new(RefCell::new(None));

        let ctx_clone = Rc::clone(&canvas_ctx);
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                // Colorize
                let pixels: Vec<u8> = result.data.iter().flat_map(colorize).collect();

                // Draw to canvas
                let _ = draw_pixels_to_canvas(
                    ctx,
                    &pixels,
                    result.tile.width,
                    result.tile.x as f64,
                    result.tile.y as f64,
                );
            }
        };

        let worker_pool = WorkerPool::new(config.id, on_tile_complete, progress)?;

        Ok(Self {
            config,
            worker_pool,
            progress,
            canvas_ctx,
        })
    }

    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }

    pub fn cancel(&self) {
        self.worker_pool.borrow_mut().cancel();
    }

    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Store canvas context for tile callbacks
        if let Ok(ctx) = get_2d_context(canvas) {
            *self.canvas_ctx.borrow_mut() = Some(ctx);
        }

        // Calculate tile size based on zoom
        let reference_width = self
            .config
            .default_viewport(viewport.precision_bits())
            .width;
        let zoom = reference_width.to_f64() / viewport.width.to_f64();
        let tile_size = calculate_tile_size(zoom);

        // Generate tiles with their fractal-space viewports
        let pixel_tiles = generate_tiles(width, height, tile_size);

        // Convert pixel tiles to tile viewports for workers
        let tiles_with_viewports: Vec<_> = pixel_tiles
            .iter()
            .map(|tile| {
                let tile_vp = tile_to_viewport(tile, viewport, (width, height));
                (tile.clone(), tile_vp)
            })
            .collect();

        // For now, workers receive the tile's viewport directly
        // We pass the pixel tile for placement, viewport is serialized per-tile
        let tiles: Vec<_> = pixel_tiles.clone();

        // Start render with main viewport (workers will receive tile viewports)
        self.worker_pool
            .borrow_mut()
            .start_render(viewport.clone(), tiles);
    }

    pub fn switch_config(&mut self, config: &'static FractalConfig) -> Result<(), JsValue> {
        self.config = config;
        self.worker_pool.borrow_mut().switch_renderer(config.id);
        Ok(())
    }
}
```

**Step 2: Update rendering/mod.rs**

Add to `fractalwonder-ui/src/rendering/mod.rs`:

```rust
mod parallel_renderer;
pub use parallel_renderer::ParallelRenderer;
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(ui): add ParallelRenderer for worker-based rendering"
```

---

## Task 8: Update WorkerPool to Send Tile Viewports

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

The current design sends the main viewport to workers, but workers need the tile's specific viewport. Update the worker pool to compute tile viewports.

**Step 1: Update WorkerPool to store canvas size and compute tile viewports**

Add field and update `start_render`:

```rust
// In WorkerPool struct, add:
canvas_size: (u32, u32),

// In new(), initialize:
canvas_size: (0, 0),

// Update start_render signature and implementation:
pub fn start_render(&mut self, viewport: Viewport, canvas_size: (u32, u32), tiles: Vec<PixelRect>) {
    self.current_render_id = self.current_render_id.wrapping_add(1);
    self.current_viewport = Some(viewport);
    self.canvas_size = canvas_size;
    self.pending_tiles = tiles.into();
    // ... rest unchanged
}

// Update dispatch_work to compute tile viewport:
fn dispatch_work(&mut self, worker_id: usize) {
    if let Some(tile) = self.pending_tiles.pop_front() {
        let tile_viewport = self.current_viewport.as_ref().map(|vp| {
            crate::rendering::tiles::tile_to_viewport(&tile, vp, self.canvas_size)
        });

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
```

**Step 2: Update ParallelRenderer to pass canvas_size**

```rust
// In ParallelRenderer::render(), update the start_render call:
self.worker_pool
    .borrow_mut()
    .start_render(viewport.clone(), (width, height), tiles);
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "fix(ui): send tile-specific viewports to workers"
```

---

## Task 9: Wire Up ParallelRenderer to InteractiveCanvas

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Replace AsyncProgressiveRenderer with ParallelRenderer**

Update imports:

```rust
use crate::rendering::ParallelRenderer;
// Remove: use crate::rendering::AsyncProgressiveRenderer;
```

Update renderer creation (note: `ParallelRenderer::new` returns `Result`):

```rust
// Create renderer - handle Result
let renderer = match ParallelRenderer::new(config.get_untracked()) {
    Ok(r) => store_value(r),
    Err(e) => {
        web_sys::console::error_1(&e);
        return view! { <div>"Failed to create renderer"</div> }.into_view();
    }
};
```

Update config change effect:

```rust
create_effect(move |_| {
    let cfg = config.get();
    renderer.update_value(|r| {
        if let Err(e) = r.switch_config(cfg) {
            web_sys::console::error_1(&e);
        }
    });

    if let Some(callback) = on_progress_signal {
        renderer.with_value(|r| callback.call(r.progress()));
    }
});
```

**Step 2: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 3: Test in browser**

Run: `trunk serve`
Expected: App loads, renders fractals using workers (check DevTools console for worker messages)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): switch InteractiveCanvas to ParallelRenderer"
```

---

## Task 10: Export tile_to_viewport Function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/tiles.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

The `tile_to_viewport` function is currently in `async_progressive_renderer.rs`. Move it to `tiles.rs` where it belongs.

**Step 1: Check if tile_to_viewport already exists in tiles.rs**

If not, move from async_progressive_renderer.rs:

```rust
// fractalwonder-ui/src/rendering/tiles.rs

use fractalwonder_core::{BigFloat, PixelRect, Viewport};

/// Convert a pixel-space tile to its corresponding fractal-space viewport.
pub fn tile_to_viewport(tile: &PixelRect, viewport: &Viewport, canvas_size: (u32, u32)) -> Viewport {
    let (canvas_width, canvas_height) = canvas_size;
    let precision = viewport.precision_bits();

    let pixel_width = viewport
        .width
        .div(&BigFloat::with_precision(canvas_width as f64, precision));
    let pixel_height = viewport
        .height
        .div(&BigFloat::with_precision(canvas_height as f64, precision));

    let canvas_center_x = canvas_width as f64 / 2.0;
    let canvas_center_y = canvas_height as f64 / 2.0;
    let tile_center_x = tile.x as f64 + tile.width as f64 / 2.0;
    let tile_center_y = tile.y as f64 + tile.height as f64 / 2.0;

    let offset_x = tile_center_x - canvas_center_x;
    let offset_y = tile_center_y - canvas_center_y;

    let offset_x_bf = pixel_width.mul(&BigFloat::with_precision(offset_x, precision));
    let offset_y_bf = pixel_height.mul(&BigFloat::with_precision(offset_y, precision));

    let center_x = viewport.center.0.add(&offset_x_bf);
    let center_y = viewport.center.1.add(&offset_y_bf);

    let tile_width = pixel_width.mul(&BigFloat::with_precision(tile.width as f64, precision));
    let tile_height = pixel_height.mul(&BigFloat::with_precision(tile.height as f64, precision));

    Viewport::with_bigfloat(center_x, center_y, tile_width, tile_height)
}
```

**Step 2: Export from mod.rs**

Ensure `tiles.rs` exports are public in `rendering/mod.rs`:

```rust
pub use tiles::{calculate_tile_size, generate_tiles, tile_to_viewport};
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/tiles.rs
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "refactor(ui): move tile_to_viewport to tiles module"
```

---

## Task 11: Clean Up AsyncProgressiveRenderer

**Files:**
- Delete: `fractalwonder-ui/src/rendering/async_progressive_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Remove async_progressive_renderer module**

Remove from `rendering/mod.rs`:

```rust
// Remove these lines:
mod async_progressive_renderer;
pub use async_progressive_renderer::AsyncProgressiveRenderer;
```

**Step 2: Delete the file**

```bash
rm fractalwonder-ui/src/rendering/async_progressive_renderer.rs
```

**Step 3: Verify build**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors (no remaining references)

**Step 4: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor(ui): remove AsyncProgressiveRenderer (replaced by ParallelRenderer)"
```

---

## Task 12: Browser Integration Test

**Files:** None (manual testing)

**Step 1: Start dev server**

Run: `trunk serve`

**Step 2: Open browser DevTools**

Open http://localhost:8080, open DevTools Console

**Step 3: Verify workers spawn**

Expected: Console shows worker initialization messages (if logging enabled) or no errors

**Step 4: Test rendering**

- Initial render should show fractal
- Pan/zoom should cancel and re-render
- Multiple tiles should appear in parallel (faster than before)

**Step 5: Test cancellation**

- Start render on deep zoom
- Immediately pan
- Render should cancel, preview should show, new render should start

**Step 6: Verify progress**

- Progress indicator should update as tiles complete
- Should show accurate tile count

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add Serialize to ComputeData | core/compute_data.rs |
| 2 | Create message types | core/messages.rs |
| 3 | Configure compute for WASM | Cargo.toml files |
| 4 | Create worker entry point | compute/worker.rs |
| 5 | Create JS loader | message-compute-worker.js |
| 6 | Create WorkerPool | ui/workers/ |
| 7 | Create ParallelRenderer | ui/rendering/parallel_renderer.rs |
| 8 | Fix tile viewport sending | ui/workers/worker_pool.rs |
| 9 | Wire up InteractiveCanvas | ui/components/interactive_canvas.rs |
| 10 | Move tile_to_viewport | ui/rendering/tiles.rs |
| 11 | Clean up old renderer | delete async_progressive_renderer.rs |
| 12 | Browser integration test | manual |
