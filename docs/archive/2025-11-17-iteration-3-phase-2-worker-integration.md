# Iteration 3 Phase 2: Worker Integration - Complete Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete Iteration 3 by integrating Web Workers with the UI for parallel fractal rendering

**Architecture:** Main thread spawns N workers, sends render requests via postMessage, polls SharedArrayBuffer for results, progressively displays colorized tiles. Workers compute tiles in parallel using work-stealing pattern.

**Tech Stack:** Rust stable, wasm-bindgen, Trunk, web_sys::Worker, js_sys::ArrayBuffer, Leptos reactivity

**Prerequisites:**
- Phase 1 complete (Tasks 1-11) - worker infrastructure exists
- Worker WASM builds successfully via Trunk
- All Phase 1 tests passing

---

## Task 12: Create WorkerPool Structure

**Files:**
- Create: `fractalwonder-ui/src/workers/mod.rs`
- Create: `fractalwonder-ui/src/workers/worker_pool.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create workers module**

Create `fractalwonder-ui/src/workers/mod.rs`:

```rust
pub mod worker_pool;

pub use worker_pool::WorkerPool;
```

**Step 2: Create WorkerPool skeleton**

Create `fractalwonder-ui/src/workers/worker_pool.rs`:

```rust
use fractalwonder_compute::{SharedBufferLayout, WorkerRequest, WorkerResponse};
use fractalwonder_core::Viewport;
use js_sys::ArrayBuffer;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

pub struct WorkerPool {
    workers: Vec<Worker>,
    shared_buffer: Option<ArrayBuffer>,
    current_render_id: Arc<AtomicU32>,
}

impl WorkerPool {
    pub fn new() -> Result<Self, JsValue> {
        // Get hardware concurrency (CPU core count)
        let worker_count = web_sys::window()
            .and_then(|w| w.navigator().hardware_concurrency())
            .map(|c| c as usize)
            .unwrap_or(4);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating WorkerPool with {} workers",
            worker_count
        )));

        let workers = Vec::new();

        Ok(Self {
            workers,
            shared_buffer: None,
            current_render_id: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn current_render_id(&self) -> u32 {
        self.current_render_id.load(Ordering::SeqCst)
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        // Terminate all workers on cleanup
        for worker in &self.workers {
            let request = WorkerRequest::Terminate;
            if let Ok(message) = serde_json::to_string(&request) {
                worker.post_message(&JsValue::from_str(&message)).ok();
            }
        }
    }
}
```

**Step 3: Export workers module**

Modify `fractalwonder-ui/src/lib.rs`, add to existing modules:

```rust
pub mod workers;
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 5: Commit WorkerPool skeleton**

```bash
git add fractalwonder-ui/src/workers/
git add fractalwonder-ui/src/lib.rs
git commit -m "feat: add WorkerPool skeleton structure"
```

---

## Task 13: Implement Worker Spawning

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add worker spawning logic**

In `fractalwonder-ui/src/workers/worker_pool.rs`, replace the `new()` implementation:

```rust
impl WorkerPool {
    pub fn new() -> Result<Self, JsValue> {
        // Get hardware concurrency (CPU core count)
        let worker_count = web_sys::window()
            .and_then(|w| w.navigator().hardware_concurrency())
            .map(|c| c as usize)
            .unwrap_or(4);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating WorkerPool with {} workers",
            worker_count
        )));

        let mut workers = Vec::new();

        for i in 0..worker_count {
            // Create worker options (no-modules type for cross-browser compatibility)
            let mut options = WorkerOptions::new();
            // Note: Not using WorkerType::Module because worker uses no-modules target

            // Worker script path (Trunk generates this in dist/)
            let worker = Worker::new("./fractalwonder-compute.js")?;

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Worker {} created",
                i
            )));

            // Set up message handler for worker responses
            let worker_id = i;
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(msg) = e.data().as_string() {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Worker {} message: {}",
                        worker_id,
                        msg
                    )));
                }
            }) as Box<dyn FnMut(_)>);

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget(); // Keep closure alive

            // Set up error handler
            let error_handler = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Worker {} error: {:?}",
                    worker_id,
                    e.message()
                )));
            }) as Box<dyn FnMut(_)>);

            worker.set_onerror(Some(error_handler.as_ref().unchecked_ref()));
            error_handler.forget(); // Keep closure alive

            workers.push(worker);
        }

        Ok(Self {
            workers,
            shared_buffer: None,
            current_render_id: Arc::new(AtomicU32::new(0)),
        })
    }

    // ... rest of impl stays the same
}
```

**Step 2: Add web-sys features to Cargo.toml**

Modify `fractalwonder-ui/Cargo.toml`, ensure these web-sys features exist (should already be there from Phase 1):

```toml
[dependencies]
web-sys = { workspace = true, features = [
    "Worker",
    "WorkerOptions",
    "WorkerType",
    "MessageEvent",
    "ErrorEvent",
] }
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 4: Commit worker spawning**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git add fractalwonder-ui/Cargo.toml
git commit -m "feat: implement worker spawning in WorkerPool"
```

---

## Task 14: Implement start_render Method

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add start_render method**

Add to `impl WorkerPool` in `fractalwonder-ui/src/workers/worker_pool.rs`:

```rust
pub fn start_render(
    &mut self,
    viewport: &Viewport<f64>,
    canvas_width: u32,
    canvas_height: u32,
    tile_size: u32,
) -> Result<u32, JsValue> {
    // Increment render ID
    let render_id = self.current_render_id.fetch_add(1, Ordering::SeqCst) + 1;

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Starting render {} ({}x{}, tile_size={})",
        render_id, canvas_width, canvas_height, tile_size
    )));

    // Create SharedArrayBuffer layout
    let layout = SharedBufferLayout::new(canvas_width, canvas_height);
    let buffer_size = layout.buffer_size();

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Creating SharedArrayBuffer of {} bytes",
        buffer_size
    )));

    // Create SharedArrayBuffer
    let shared_buffer = js_sys::SharedArrayBuffer::new(buffer_size as u32);
    self.shared_buffer = Some(shared_buffer.clone());

    // Initialize atomic counters in buffer
    let int32_array = js_sys::Int32Array::new(&shared_buffer);
    int32_array.set_index(0, 0); // tile_index counter = 0
    int32_array.set_index(1, render_id as i32); // render_id

    // Zero out pixel data
    let view = js_sys::Uint8Array::new(&shared_buffer);
    for i in 8..buffer_size {
        view.set_index(i as u32, 0);
    }

    // Serialize viewport to JSON
    let viewport_json = serde_json::to_string(viewport)
        .map_err(|e| JsValue::from_str(&format!("Serialize viewport error: {}", e)))?;

    // Create render request
    let request = WorkerRequest::Render {
        viewport_json,
        canvas_width,
        canvas_height,
        render_id,
        tile_size,
    };

    let message = serde_json::to_string(&request)
        .map_err(|e| JsValue::from_str(&format!("Serialize request error: {}", e)))?;

    // Send to all workers
    for (i, worker) in self.workers.iter().enumerate() {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Sending render request to worker {}",
            i
        )));

        // Post message with shared buffer
        // Note: We send the buffer reference, not transfer it
        worker.post_message(&JsValue::from_str(&message))?;
    }

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Render {} started on {} workers",
        render_id,
        self.workers.len()
    )));

    Ok(render_id)
}

pub fn get_shared_buffer(&self) -> Option<&ArrayBuffer> {
    self.shared_buffer.as_ref()
}
```

**Step 2: Add SharedArrayBuffer to web-sys features**

Modify `fractalwonder-ui/Cargo.toml`, add SharedArrayBuffer feature:

```toml
web-sys = { workspace = true, features = [
    # ... existing features ...
    "Worker",
    "MessageEvent",
    "ErrorEvent",
] }

js-sys.workspace = true
```

Note: SharedArrayBuffer is in js-sys, not web-sys, so no feature flag needed.

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 4: Commit start_render method**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat: implement start_render for WorkerPool"
```

---

## Task 15: Create ParallelCanvasRenderer

**Files:**
- Create: `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Create ParallelCanvasRenderer structure**

Create `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`:

```rust
use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::colorizers::Colorizer;
use crate::workers::WorkerPool;
use fractalwonder_compute::SharedBufferLayout;
use fractalwonder_core::{AppData, Viewport};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

pub struct ParallelCanvasRenderer {
    worker_pool: Arc<WorkerPool>,
    colorizer: Arc<dyn Colorizer<AppData>>,
    tile_size: u32,
}

impl ParallelCanvasRenderer {
    pub fn new(colorizer: Arc<dyn Colorizer<AppData>>, tile_size: u32) -> Result<Self, JsValue> {
        let worker_pool = Arc::new(WorkerPool::new()?);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer created with {} workers, tile_size={}",
            worker_pool.worker_count(),
            tile_size
        )));

        Ok(Self {
            worker_pool,
            colorizer,
            tile_size,
        })
    }

    pub fn worker_count(&self) -> usize {
        self.worker_pool.worker_count()
    }
}

impl Clone for ParallelCanvasRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Arc::clone(&self.worker_pool),
            colorizer: Arc::clone(&self.colorizer),
            tile_size: self.tile_size,
        }
    }
}
```

**Step 2: Implement CanvasRenderer trait**

Add to `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`:

```rust
impl CanvasRenderer for ParallelCanvasRenderer {
    type Precision = f64;
    type Data = AppData;

    fn render(
        &self,
        canvas: &HtmlCanvasElement,
        viewport: &Viewport<Self::Precision>,
    ) -> Result<(), JsValue> {
        let width = canvas.width();
        let height = canvas.height();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer::render starting ({}x{})",
            width, height
        )));

        // Start render on workers (BLOCKS THREAD - will fix in next task)
        let worker_pool_mut = Arc::get_mut(&mut self.worker_pool.clone())
            .ok_or_else(|| JsValue::from_str("Cannot get mutable worker pool"))?;

        let render_id = worker_pool_mut.start_render(viewport, width, height, self.tile_size)?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Render {} dispatched to workers",
            render_id
        )));

        // TODO: Poll SharedArrayBuffer and display results progressively
        // For now, just log that we started

        Ok(())
    }
}
```

**Step 3: Export ParallelCanvasRenderer**

Modify `fractalwonder-ui/src/rendering/mod.rs`, add:

```rust
pub mod parallel_canvas_renderer;

pub use parallel_canvas_renderer::ParallelCanvasRenderer;
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 5: Commit ParallelCanvasRenderer skeleton**

```bash
git add fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat: add ParallelCanvasRenderer skeleton"
```

---

## Task 16: Implement Progressive Buffer Polling

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 1: Add poll_and_render method**

Add to `impl ParallelCanvasRenderer` in `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`:

```rust
use fractalwonder_core::MandelbrotData;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ParallelCanvasRenderer {
    worker_pool: Arc<WorkerPool>,
    colorizer: Arc<dyn Colorizer<AppData>>,
    tile_size: u32,
    poll_closure: RefCell<Option<Closure<dyn FnMut()>>>,
}

impl ParallelCanvasRenderer {
    pub fn new(colorizer: Arc<dyn Colorizer<AppData>>, tile_size: u32) -> Result<Self, JsValue> {
        let worker_pool = Arc::new(WorkerPool::new()?);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer created with {} workers, tile_size={}",
            worker_pool.worker_count(),
            tile_size
        )));

        Ok(Self {
            worker_pool,
            colorizer,
            tile_size,
            poll_closure: RefCell::new(None),
        })
    }

    fn poll_and_render(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let Some(buffer) = self.worker_pool.get_shared_buffer() else {
            return Ok(()); // No active render
        };

        let width = canvas.width();
        let height = canvas.height();
        let layout = SharedBufferLayout::new(width, height);

        // Read all pixel data from SharedArrayBuffer
        let view = js_sys::Uint8Array::new(buffer);
        let mut pixel_data = Vec::with_capacity(layout.total_pixels);

        for pixel_idx in 0..layout.total_pixels {
            let offset = layout.pixel_offset(pixel_idx);
            let mut bytes = [0u8; 8];
            for i in 0..8 {
                bytes[i] = view.get_index((offset + i) as u32);
            }

            let data = SharedBufferLayout::decode_pixel(&bytes);
            pixel_data.push(data);
        }

        // Colorize pixels
        let colors = pixel_data
            .iter()
            .map(|data| self.colorizer.colorize(data))
            .collect::<Vec<_>>();

        // Draw to canvas
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("No 2d context"))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

        let image_data =
            web_sys::ImageData::new_with_u8_clamped_array(wasm_bindgen::Clamped(&colors), width)?;

        context.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }

    fn start_progressive_poll(&self, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let self_clone = self.clone();
        let canvas_clone = canvas.clone();

        let closure = Closure::wrap(Box::new(move || {
            if let Err(e) = self_clone.poll_and_render(&canvas_clone) {
                web_sys::console::error_1(&e);
            }

            // Continue polling
            if let Err(e) = self_clone.start_progressive_poll(canvas_clone.clone()) {
                web_sys::console::error_1(&e);
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
```

**Step 2: Update render() to start polling**

Replace the `render()` implementation in `parallel_canvas_renderer.rs`:

```rust
impl CanvasRenderer for ParallelCanvasRenderer {
    type Precision = f64;
    type Data = AppData;

    fn render(
        &self,
        canvas: &HtmlCanvasElement,
        viewport: &Viewport<Self::Precision>,
    ) -> Result<(), JsValue> {
        let width = canvas.width();
        let height = canvas.height();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "ParallelCanvasRenderer::render starting ({}x{})",
            width, height
        )));

        // SAFETY: This is not ideal but needed for mutable access
        // In production, we'd use RefCell or interior mutability pattern
        let worker_pool_ptr = Arc::as_ptr(&self.worker_pool) as *mut WorkerPool;
        let worker_pool_mut = unsafe { &mut *worker_pool_ptr };

        let render_id = worker_pool_mut.start_render(viewport, width, height, self.tile_size)?;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Render {} dispatched to workers",
            render_id
        )));

        // Start progressive polling
        self.start_progressive_poll(canvas.clone())?;

        Ok(())
    }
}
```

**Step 3: Fix Clone implementation**

Update Clone impl in `parallel_canvas_renderer.rs`:

```rust
impl Clone for ParallelCanvasRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Arc::clone(&self.worker_pool),
            colorizer: Arc::clone(&self.colorizer),
            tile_size: self.tile_size,
            poll_closure: RefCell::new(None),
        }
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully (with potential warnings about unsafe code)

**Step 5: Commit progressive polling**

```bash
git add fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs
git commit -m "feat: implement progressive buffer polling and rendering"
```

---

## Task 17: Fix Worker Communication (SharedArrayBuffer Transfer)

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Update worker communication to pass SharedArrayBuffer**

The current implementation doesn't pass the SharedArrayBuffer to workers. We need to fix this.

Modify `start_render()` in `fractalwonder-ui/src/workers/worker_pool.rs`:

```rust
pub fn start_render(
    &mut self,
    viewport: &Viewport<f64>,
    canvas_width: u32,
    canvas_height: u32,
    tile_size: u32,
) -> Result<u32, JsValue> {
    // ... (existing code for creating buffer) ...

    // Create render request
    let request = WorkerRequest::Render {
        viewport_json,
        canvas_width,
        canvas_height,
        render_id,
        tile_size,
    };

    let message = serde_json::to_string(&request)
        .map_err(|e| JsValue::from_str(&format!("Serialize request error: {}", e)))?;

    // Send to all workers WITH SharedArrayBuffer
    for (i, worker) in self.workers.iter().enumerate() {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Sending render request to worker {}",
            i
        )));

        // Create message object with both the JSON request and the buffer
        let msg_obj = js_sys::Object::new();
        js_sys::Reflect::set(&msg_obj, &JsValue::from_str("request"), &JsValue::from_str(&message))?;
        js_sys::Reflect::set(&msg_obj, &JsValue::from_str("buffer"), &shared_buffer)?;

        worker.post_message(&msg_obj)?;
    }

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Render {} started on {} workers",
        render_id,
        self.workers.len()
    )));

    Ok(render_id)
}
```

**Step 2: Update worker to receive SharedArrayBuffer**

We need to update the worker entry point. Currently, `process_render_request` expects the buffer as a parameter, but we're sending it via postMessage.

Create a new worker handler. Modify `fractalwonder-compute/src/worker.rs`, add:

```rust
#[wasm_bindgen]
pub fn handle_message(event_data: JsValue) -> Result<(), JsValue> {
    // Parse the message object
    let request_str = js_sys::Reflect::get(&event_data, &JsValue::from_str("request"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("No request field"))?;

    let buffer = js_sys::Reflect::get(&event_data, &JsValue::from_str("buffer"))?
        .dyn_into::<js_sys::ArrayBuffer>()?;

    // Call existing process_render_request with the buffer
    process_render_request(request_str, buffer)
}
```

**Step 3: Update worker message handler in WorkerPool**

Modify worker creation in `fractalwonder-ui/src/workers/worker_pool.rs` to call the right entry point:

```rust
// In WorkerPool::new(), update the onmessage handler:

let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
    let data = e.data();

    // First message is "Ready", subsequent messages are progress updates
    if let Ok(msg) = data.as_string() {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Worker {} message: {}",
            worker_id,
            msg
        )));
    }
}) as Box<dyn FnMut(_)>);
```

**Step 4: Add JavaScript glue code to call worker handler**

We need to add a JavaScript wrapper to call our Rust worker code. This needs to be done in the worker script itself.

For now, we'll document that the worker needs to set up its own message handler when it initializes. The worker's `init_worker()` should set this up.

Modify `fractalwonder-compute/src/worker.rs`, update `init_worker()`:

```rust
#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Err(err) = handle_message(e.data()) {
            web_sys::console::error_1(&err);
        }
    }) as Box<dyn FnMut(_)>);

    let global = js_sys::global().dyn_into::<web_sys::DedicatedWorkerGlobalScope>().unwrap();
    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Send ready message
    let response = WorkerResponse::Ready;
    if let Ok(message) = serde_json::to_string(&response) {
        global.post_message(&JsValue::from_str(&message)).ok();
    }
}
```

**Step 5: Export handle_message**

Modify `fractalwonder-compute/src/lib.rs`, ensure worker module exports are correct:

```rust
#[cfg(target_arch = "wasm32")]
pub mod worker;

#[cfg(target_arch = "wasm32")]
pub use worker::{init_worker, process_render_request, handle_message};
```

**Step 6: Verify compilation**

Run: `cargo check --workspace`
Expected: Compiles successfully

**Step 7: Commit worker communication fixes**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git add fractalwonder-compute/src/worker.rs
git add fractalwonder-compute/src/lib.rs
git commit -m "fix: correctly pass SharedArrayBuffer to workers via postMessage"
```

---

## Task 18: Integrate ParallelCanvasRenderer with App

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Add ParallelCanvasRenderer option to AppRenderer enum**

Modify `fractalwonder-ui/src/app.rs`, find the `AppRenderer` enum and add variant:

```rust
pub enum AppRenderer {
    F64(AsyncProgressiveCanvasRenderer<f64, AppData>),
    BigFloat(AsyncProgressiveCanvasRenderer<BigFloat, AppData>),
    Parallel(ParallelCanvasRenderer), // NEW
}
```

**Step 2: Add factory function for ParallelCanvasRenderer**

Add to `fractalwonder-ui/src/app.rs`:

```rust
use crate::rendering::ParallelCanvasRenderer;

fn create_parallel_renderer(
    colorizer: Arc<dyn Colorizer<AppData>>,
) -> Result<ParallelCanvasRenderer, JsValue> {
    ParallelCanvasRenderer::new(colorizer, 128)
}
```

**Step 3: Update App initialization to use parallel renderer**

Find the `App` component and update renderer creation. Look for where `AppRenderer::F64` is created and change it to:

```rust
// In App component, find renderer initialization and change to:
let renderer = create_signal(AppRenderer::Parallel(
    create_parallel_renderer(Arc::new(mandelbrot_fire_colorizer()))
        .expect("Failed to create parallel renderer")
));
```

**Step 4: Update CanvasRenderer trait implementation for AppRenderer**

Find `impl CanvasRenderer for AppRenderer` and add the parallel variant:

```rust
impl CanvasRenderer for AppRenderer {
    type Precision = f64; // Parallel renderer always uses f64
    type Data = AppData;

    fn render(
        &self,
        canvas: &HtmlCanvasElement,
        viewport: &Viewport<Self::Precision>,
    ) -> Result<(), JsValue> {
        match self {
            AppRenderer::F64(renderer) => renderer.render(canvas, viewport),
            AppRenderer::BigFloat(renderer) => {
                // Convert viewport to f64 for rendering
                let f64_viewport = Viewport::new(
                    Point::new(viewport.center.x().to_f64(), viewport.center.y().to_f64()),
                    viewport.zoom,
                );
                // This is a bit of a hack - we should handle this better
                unimplemented!("BigFloat with parallel renderer not yet supported")
            }
            AppRenderer::Parallel(renderer) => renderer.render(canvas, viewport), // NEW
        }
    }
}
```

**Step 5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 6: Build with Trunk**

Run: `trunk build`
Expected: Builds successfully, worker WASM generated

**Step 7: Commit app integration**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat: integrate ParallelCanvasRenderer with App"
```

---

## Task 19: Manual Browser Testing

**Files:**
- None (manual testing)

**Step 1: Start Trunk dev server**

Run: `trunk serve`
Expected: Server starts on http://localhost:8080

**Step 2: Open browser DevTools**

1. Open http://localhost:8080 in browser
2. Open DevTools (F12)
3. Go to Console tab

**Step 3: Verify worker creation**

Expected console output:
```
Creating WorkerPool with N workers
Worker 0 created
Worker 1 created
...
ParallelCanvasRenderer created with N workers
```

**Step 4: Verify render starts**

Expected console output:
```
ParallelCanvasRenderer::render starting (WxH)
Starting render 1 (WxH, tile_size=128)
Creating SharedArrayBuffer of NNNN bytes
Sending render request to worker 0
Sending render request to worker 1
...
Render 1 started on N workers
```

**Step 5: Check for worker messages**

Expected console output from workers:
```
Worker: Starting render 1
Worker: Render 1 - processing tile 0
Worker: Render 1 - processing tile 1
...
```

**Step 6: Verify fractal appears**

Expected: Fractal image appears progressively in the canvas

**Step 7: Check Performance tab**

1. Go to DevTools → Performance
2. Click Record
3. Wait for fractal to render
4. Stop recording
5. Look for multiple "Worker" threads active

Expected: Multiple worker threads showing activity

**Step 8: Check CPU usage**

Expected: Task Manager / Activity Monitor shows multi-core CPU usage (200-400%+ depending on cores)

**Step 9: Document results**

Create: `docs/testing/2025-11-17-iteration-3-manual-test-results.md`

```markdown
# Iteration 3 Manual Test Results

**Date:** 2025-11-17
**Browser:** [Browser name and version]
**CPU:** [CPU model and core count]

## Test Results

- [ ] Workers spawn successfully: YES/NO
- [ ] Worker count matches hardware_concurrency: YES/NO
- [ ] SharedArrayBuffer created: YES/NO
- [ ] Render request sent to all workers: YES/NO
- [ ] Workers receive messages: YES/NO
- [ ] Workers compute tiles: YES/NO
- [ ] Fractal appears in browser: YES/NO
- [ ] Progressive display visible: YES/NO
- [ ] Multiple CPU cores utilized: YES/NO

## Console Output

[Paste relevant console output here]

## Performance

- Render time: XX seconds
- CPU utilization: XX%
- Number of workers: XX

## Issues Found

[List any issues discovered]
```

**Step 10: No commit (manual testing)**

---

## Task 20: Fix Worker Script Loading (if needed)

**This task is conditional - only do it if Task 19 shows workers failing to load**

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`
- Possibly: `index.html`

**Step 1: Check worker script path in browser DevTools**

In DevTools → Network tab, look for worker script request.

Possible paths:
- `./fractalwonder-compute.js` (relative)
- `/fractalwonder-compute.js` (absolute)
- `./fractalwonder_compute.js` (underscore instead of hyphen)

**Step 2: Update worker path if needed**

If workers fail to load, update the path in `WorkerPool::new()`:

```rust
// Try different paths:
let worker = Worker::new("/fractalwonder-compute.js")?; // absolute
// OR
let worker = Worker::new("./fractalwonder_compute.js")?; // underscore
```

**Step 3: Check Trunk dist/ output**

Run: `ls dist/*.js | grep compute`
Expected: Shows exact filename Trunk generated

**Step 4: Update path to match**

Update worker creation to use exact filename from dist/

**Step 5: Rebuild and test**

Run: `trunk build && trunk serve`

**Step 6: Commit fix if changes made**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "fix: correct worker script path for Trunk output"
```

---

## Task 21: Final Validation

**Files:**
- Create: `docs/testing/2025-11-17-iteration-3-validation.md`

**Step 1: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 2: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run formatter**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Build release**

Run: `trunk build --release`
Expected: Builds successfully

**Step 5: Verify Iteration 3 success criteria**

From design document:

- [ ] CPU utilization shows multi-core usage
- [ ] Render time decreases vs. single-threaded
- [ ] Progressive display still works

**Step 6: Benchmark comparison**

Create simple benchmark comparing:
- AsyncProgressiveCanvasRenderer (single-threaded)
- ParallelCanvasRenderer (multi-threaded)

Expected: 2-4x speedup depending on core count

**Step 7: Create validation document**

Create: `docs/testing/2025-11-17-iteration-3-validation.md`

```markdown
# Iteration 3 Validation Results

**Date:** 2025-11-17

## Success Criteria

✅/❌ CPU utilization shows multi-core usage
✅/❌ Render time decreases vs. single-threaded
✅/❌ Progressive display still works

## Benchmark Results

**Single-threaded (AsyncProgressiveCanvasRenderer):**
- Render time: XX seconds

**Multi-threaded (ParallelCanvasRenderer):**
- Render time: XX seconds
- Speedup: XXx
- CPU cores used: XX

## Test Results

- Total tests: XXX
- Passing: XXX
- Failing: 0
- Clippy warnings: 0

## Conclusion

Iteration 3 [COMPLETE/INCOMPLETE]

## Next Steps

[If complete: Proceed to Iteration 4]
[If incomplete: List blocking issues]
```

**Step 8: Commit validation results**

```bash
git add docs/testing/
git commit -m "docs: add Iteration 3 validation results"
```

---

## Success Criteria - Iteration 3 Complete

At this point, you should have:

✅ Workers spawn successfully from main thread
✅ SharedArrayBuffer created and shared with workers
✅ Workers receive render requests via postMessage
✅ Workers compute tiles in parallel using work-stealing
✅ Workers write results to SharedArrayBuffer
✅ Main thread polls buffer and displays results progressively
✅ Fractal renders appear in browser
✅ Multi-core CPU utilization visible
✅ 2-4x performance improvement vs. single-threaded
✅ Progressive display working
✅ All tests passing
✅ No Clippy warnings

## Validation Checklist

Run through this checklist to verify Iteration 3 is complete:

- [ ] Task 12: WorkerPool structure created ✅
- [ ] Task 13: Worker spawning implemented ✅
- [ ] Task 14: start_render method works ✅
- [ ] Task 15: ParallelCanvasRenderer created ✅
- [ ] Task 16: Progressive polling implemented ✅
- [ ] Task 17: Worker communication fixed ✅
- [ ] Task 18: App integration complete ✅
- [ ] Task 19: Manual browser testing passed ✅
- [ ] Task 20: Worker loading works ✅
- [ ] Task 21: Final validation complete ✅

**If all checkboxes are ticked: Iteration 3 is COMPLETE! 🎉**

---

## Known Issues & Future Work

**Interior Mutability:**
Task 16 uses `unsafe` to get mutable access to WorkerPool. This should be refactored to use `RefCell<WorkerPool>` or similar interior mutability pattern.

**Error Handling:**
Worker errors should be propagated back to main thread and displayed to user.

**Cancellation:**
Render cancellation (Iteration 4) not yet implemented. Workers will complete current render even if user starts new one.

**Optimization:**
Tile size, polling frequency, and worker count can be tuned for better performance (Iteration 5).

---

## Next Iteration

**Iteration 4: Responsive Cancellation**

Goal: Pan/zoom immediately stops render, UI never freezes

See: `docs/multicore-plans/2025-11-17-progressive-parallel-rendering-design.md` lines 152-167
