# Multiple Renderer Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable runtime selection of different renderers (TestImage, Mandelbrot) in workers via message protocol.

**Architecture:** Workers signal readiness via Ready message, receive Initialize message with renderer_id, create appropriate renderer using factory pattern, then process tiles. TestImageComputer becomes generic over scalar type like MandelbrotComputer. Worker pool recreates workers when renderer changes.

**Tech Stack:** Rust, WASM, Leptos, Web Workers, message passing via JSON serialization

---

## Task 1: Add Ready Message to Protocol

**Files:**
- Modify: `fractalwonder-compute/src/messages.rs`
- Test: `fractalwonder-compute/src/messages.rs` (inline tests)

**Step 1: Write failing test for Ready message serialization**

Add to bottom of `fractalwonder-compute/src/messages.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready_message_serialization() {
        let msg = WorkerToMain::Ready;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Ready\""));
    }

    #[test]
    fn test_ready_message_deserialization() {
        let json = r#"{"type":"Ready"}"#;
        let msg: WorkerToMain = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WorkerToMain::Ready));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute messages::tests::test_ready_message`

Expected: FAIL with "no variant named `Ready`"

**Step 3: Add Ready variant to WorkerToMain**

In `fractalwonder-compute/src/messages.rs`, modify the `WorkerToMain` enum:

```rust
/// Messages sent from worker to main thread
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker is initialized and ready for commands
    Ready,

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
        data: Vec<AppData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error
    Error {
        render_id: Option<u32>,
        tile: Option<PixelRect>,
        error: String,
    },
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-compute messages::tests`

Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/messages.rs
git commit -m "feat: add Ready message to WorkerToMain protocol"
```

---

## Task 2: Add Initialize Message to Protocol

**Files:**
- Modify: `fractalwonder-compute/src/messages.rs`

**Step 1: Write failing test for Initialize message**

Add to tests module in `fractalwonder-compute/src/messages.rs`:

```rust
#[test]
fn test_initialize_message_serialization() {
    let msg = MainToWorker::Initialize {
        renderer_id: "test_image".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"Initialize\""));
    assert!(json.contains("\"renderer_id\":\"test_image\""));
}

#[test]
fn test_initialize_message_deserialization() {
    let json = r#"{"type":"Initialize","renderer_id":"mandelbrot"}"#;
    let msg: MainToWorker = serde_json::from_str(json).unwrap();
    match msg {
        MainToWorker::Initialize { renderer_id } => {
            assert_eq!(renderer_id, "mandelbrot");
        }
        _ => panic!("Expected Initialize variant"),
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute messages::tests::test_initialize`

Expected: FAIL with "no variant named `Initialize`"

**Step 3: Add Initialize variant to MainToWorker**

In `fractalwonder-compute/src/messages.rs`, modify the `MainToWorker` enum:

```rust
/// Messages sent from main thread to worker
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Initialize worker with specified renderer
    Initialize { renderer_id: String },

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

**Step 4: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-compute messages::tests`

Expected: PASS (4 tests)

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/messages.rs
git commit -m "feat: add Initialize message to MainToWorker protocol"
```

---

## Task 3: Make TestImageComputer Generic - Part 1 (Structure)

**Files:**
- Modify: `fractalwonder-compute/src/computers/test_image.rs`

**Step 1: Read existing tests**

Run: `cargo test --package fractalwonder-compute test_image::tests -- --list`

Note: Tests exist at bottom of file (lines 95-150)

**Step 2: Add test for BigFloat instantiation**

Add to tests module in `fractalwonder-compute/src/computers/test_image.rs`:

```rust
#[test]
fn test_computer_instantiation_with_bigfloat() {
    use fractalwonder_core::BigFloat;
    let computer = TestImageComputer::<BigFloat>::new();
    let bounds = computer.natural_bounds();
    assert_eq!(bounds.min.x().to_f64(), -50.0);
    assert_eq!(bounds.max.x().to_f64(), 50.0);
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute test_image::tests::test_computer_instantiation_with_bigfloat`

Expected: FAIL with "struct takes 0 generic arguments but 1 generic argument was supplied"

**Step 4: Make TestImageComputer generic**

In `fractalwonder-compute/src/computers/test_image.rs`, replace the struct definition:

```rust
use crate::point_compute::ImagePointComputer;
use crate::renderer_info::{RendererInfo, RendererInfoData};
use fractalwonder_core::{Point, Rect, TestImageData, Viewport};
use num_traits::{Float, FromPrimitive, ToPrimitive};  // NEW

#[derive(Clone)]
pub struct TestImageComputer<T> {
    checkerboard_size: T,
    circle_radius_step: T,
    circle_line_thickness: T,
}

impl<T> Default for TestImageComputer<T>
where
    T: Clone + FromPrimitive,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TestImageComputer<T>
where
    T: Clone + FromPrimitive,
{
    pub fn new() -> Self {
        Self {
            checkerboard_size: T::from_f64(5.0).unwrap(),
            circle_radius_step: T::from_f64(10.0).unwrap(),
            circle_line_thickness: T::from_f64(0.1).unwrap(),
        }
    }

    fn compute_point_data(&self, x: T, y: T) -> TestImageData
    where
        T: Float + ToPrimitive,
    {
        // Calculate circle distance
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        // On circle if within line thickness and not at origin
        let circle_distance = if ring_distance < self.circle_line_thickness / T::from_f64(2.0).unwrap()
            && nearest_ring > T::zero()
        {
            ring_distance.to_f64().unwrap()
        } else {
            ring_distance.to_f64().unwrap() + 1.0
        };

        // Also treat vertical green line as a circle for now
        if x.abs() < self.circle_line_thickness {
            return TestImageData::new(false, 0.0);
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor().to_i32().unwrap();
        let square_y = (y / self.checkerboard_size).floor().to_i32().unwrap();
        let is_light = (square_x + square_y) % 2 == 0;

        TestImageData::new(is_light, circle_distance)
    }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute test_image::tests::test_computer_instantiation_with_bigfloat`

Expected: PASS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/computers/test_image.rs
git commit -m "feat: make TestImageComputer generic over scalar type"
```

---

## Task 4: Make TestImageComputer Generic - Part 2 (ImagePointComputer Trait)

**Files:**
- Modify: `fractalwonder-compute/src/computers/test_image.rs`

**Step 1: Write test for generic compute**

Add to tests module:

```rust
#[test]
fn test_compute_with_bigfloat() {
    use fractalwonder_core::BigFloat;
    let computer = TestImageComputer::<BigFloat>::new();
    let viewport = Viewport::new(
        Point::new(BigFloat::from(0.0), BigFloat::from(0.0)),
        1.0,
    );
    let coord = Point::new(BigFloat::from(10.0), BigFloat::from(0.0));
    let data = computer.compute(coord, &viewport);

    // Point at (10, 0) should be on a circle
    assert!(data.circle_distance < 0.1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-compute test_image::tests::test_compute_with_bigfloat`

Expected: FAIL with trait bound issues

**Step 3: Implement ImagePointComputer for generic T**

Replace the `impl ImagePointComputer` block in `fractalwonder-compute/src/computers/test_image.rs`:

```rust
impl<T> ImagePointComputer for TestImageComputer<T>
where
    T: Clone + Float + FromPrimitive + ToPrimitive,
{
    type Scalar = T;
    type Data = TestImageData;

    fn natural_bounds(&self) -> Rect<T> {
        Rect::new(
            Point::new(T::from_f64(-50.0).unwrap(), T::from_f64(-50.0).unwrap()),
            Point::new(T::from_f64(50.0).unwrap(), T::from_f64(50.0).unwrap()),
        )
    }

    fn compute(&self, coord: Point<T>, _viewport: &Viewport<T>) -> TestImageData {
        self.compute_point_data(coord.x().clone(), coord.y().clone())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-compute test_image::tests::test_compute_with_bigfloat`

Expected: PASS

**Step 5: Update RendererInfo impl to specify f64**

Replace the `impl RendererInfo` block:

```rust
impl RendererInfo for TestImageComputer<f64> {
    type Scalar = f64;

    fn info(&self, viewport: &Viewport<f64>) -> RendererInfoData {
        RendererInfoData {
            name: "Test Image".to_string(),
            center_display: format!(
                "x: {:.2}, y: {:.2}",
                viewport.center.x(),
                viewport.center.y()
            ),
            zoom_display: format!("{:.2}x", viewport.zoom),
            custom_params: vec![
                (
                    "Checkerboard size".to_string(),
                    format!("{:.1}", self.checkerboard_size),
                ),
                (
                    "Circle radius step".to_string(),
                    format!("{:.1}", self.circle_radius_step),
                ),
            ],
            render_time_ms: None,
        }
    }
}
```

**Step 6: Run all tests to verify they pass**

Run: `cargo test --package fractalwonder-compute test_image::tests`

Expected: PASS (all existing + new tests)

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/computers/test_image.rs
git commit -m "feat: implement ImagePointComputer for TestImageComputer<T>"
```

---

## Task 5: Update RenderConfig with Renderer Factory

**Files:**
- Modify: `fractalwonder-compute/src/render_config.rs`

**Step 1: Add create_renderer field to RenderConfig**

In `fractalwonder-compute/src/render_config.rs`, update the struct:

```rust
use crate::adaptive_mandelbrot_renderer::AdaptiveMandelbrotRenderer;  // NEW
use crate::app_data_renderer::AppDataRenderer;  // NEW
use crate::computers::{MandelbrotComputer, TestImageComputer};
use crate::pixel_renderer::PixelRenderer;  // NEW
use crate::renderer_info::RendererInfo;
use crate::renderer_trait::Renderer;  // NEW
use fractalwonder_core::{AppData, BigFloat};  // NEW - add BigFloat
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ColorScheme {
    pub id: &'static str,
    pub display_name: &'static str,
}

pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
    pub create_renderer: fn() -> Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>,  // NEW
}
```

**Step 2: Update RENDER_CONFIGS with factory functions**

Replace the `RENDER_CONFIGS` static:

```rust
pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        display_name: "Test Image",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
            },
            ColorScheme {
                id: "pastel",
                display_name: "Pastel",
            },
        ],
        default_color_scheme_id: "default",
        create_info_provider: || Box::new(TestImageComputer::<f64>::new()),
        create_renderer: || {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
            Box::new(app_renderer)
        },
    },
    RenderConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        color_schemes: &[
            ColorScheme {
                id: "default",
                display_name: "Default",
            },
            ColorScheme {
                id: "fire",
                display_name: "Fire",
            },
            ColorScheme {
                id: "opal",
                display_name: "Opal",
            },
        ],
        default_color_scheme_id: "default",
        create_info_provider: || Box::new(MandelbrotComputer::new()),
        create_renderer: || Box::new(AdaptiveMandelbrotRenderer::new(1e10)),
    },
];
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-compute`

Expected: SUCCESS

**Step 4: Add helper function to create renderer by ID**

Add to bottom of `fractalwonder-compute/src/render_config.rs`:

```rust
/// Create a renderer by ID, or return None if unknown
pub fn create_renderer(renderer_id: &str) -> Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>> {
    get_config(renderer_id).map(|config| (config.create_renderer)())
}
```

**Step 5: Verify compilation**

Run: `cargo check --package fractalwonder-compute`

Expected: SUCCESS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/render_config.rs
git commit -m "feat: add create_renderer factory to RenderConfig"
```

---

## Task 6: Update Worker Initialization Protocol

**Files:**
- Modify: `fractalwonder-compute/src/worker.rs`

**Step 1: Add renderer factory function**

In `fractalwonder-compute/src/worker.rs`, add at top after imports:

```rust
use crate::{AdaptiveMandelbrotRenderer, MainToWorker, Renderer, WorkerToMain};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Viewport};
use js_sys::Date;
use std::cell::RefCell;  // NEW
use std::rc::Rc;  // NEW
use wasm_bindgen::prelude::*;

fn create_renderer(renderer_id: &str) -> Result<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>, JsValue> {
    crate::render_config::create_renderer(renderer_id)
        .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))
}
```

**Step 2: Update init_message_worker to use Ready protocol**

Replace the `init_message_worker` function:

```rust
/// Message-based worker initialization
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    // No renderer created yet - wait for Initialize message
    let renderer: Rc<RefCell<Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>>>> =
        Rc::new(RefCell::new(None));

    // Set up message handler
    let renderer_clone = Rc::clone(&renderer);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Err(err) = handle_worker_message(&renderer_clone, e.data()) {
            web_sys::console::error_1(&JsValue::from_str(&format!("Worker error: {:?}", err)));
        }
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
        .dyn_into()
        .expect("Failed to get worker global scope");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready - wait for Initialize message
    send_message(&WorkerToMain::Ready);
}
```

**Step 3: Update handle_worker_message to handle Initialize**

Replace the `handle_worker_message` function:

```rust
fn handle_worker_message(
    renderer: &Rc<RefCell<Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>>>>,
    data: JsValue,
) -> Result<(), JsValue> {
    let msg_str = data
        .as_string()
        .ok_or_else(|| JsValue::from_str("Message data is not a string"))?;

    let msg: MainToWorker = serde_json::from_str(&msg_str)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse message: {}", e)))?;

    match msg {
        MainToWorker::Initialize { renderer_id } => {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Initializing worker with renderer: {}",
                renderer_id
            )));

            let new_renderer = create_renderer(&renderer_id)?;
            *renderer.borrow_mut() = Some(new_renderer);

            // Now ready for work
            send_message(&WorkerToMain::RequestWork { render_id: None });
        }
        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
            canvas_width,
            canvas_height,
        } => {
            let borrowed = renderer.borrow();
            let r = borrowed
                .as_ref()
                .ok_or_else(|| JsValue::from_str("Renderer not initialized"))?;

            handle_render_tile(
                r.as_ref(),
                render_id,
                viewport_json,
                tile,
                canvas_width,
                canvas_height,
            )?;
        }
        MainToWorker::NoWork => {
            // Render complete, go idle
        }
        MainToWorker::Terminate => {
            let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                .dyn_into()
                .expect("Failed to get worker global scope");
            global.close();
        }
    }

    Ok(())
}
```

**Step 4: Update handle_render_tile signature**

Change the function signature to take `&dyn Renderer` instead of `&AdaptiveMandelbrotRenderer`:

```rust
fn handle_render_tile(
    renderer: &dyn Renderer<Scalar = BigFloat, Data = AppData>,
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
```

**Step 5: Verify compilation**

Run: `cargo check --package fractalwonder-compute --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 6: Commit**

```bash
git add fractalwonder-compute/src/worker.rs
git commit -m "feat: update worker to use Ready/Initialize protocol"
```

---

## Task 7: Update RenderWorkerPool - Part 1 (Add renderer_id Field)

**Files:**
- Modify: `fractalwonder-ui/src/workers/render_worker_pool.rs`

**Step 1: Add renderer_id field to RenderWorkerPool**

In `fractalwonder-ui/src/workers/render_worker_pool.rs`, update the struct (around line 23):

```rust
pub struct RenderWorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,  // NEW - which renderer workers use
    pending_tiles: VecDeque<TileRequest>,
    failed_tiles: HashMap<(u32, u32), u32>,
    current_render_id: u32,
    current_viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,
    render_start_time: Rc<RefCell<Option<f64>>>,
    self_ref: Weak<RefCell<Self>>,
}
```

**Step 2: Update new() constructor to accept renderer_id**

Update the `new` function signature (around line 88):

```rust
pub fn new<F>(
    on_tile_complete: F,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,
    renderer_id: String,  // NEW
) -> Result<Rc<RefCell<Self>>, JsValue>
where
    F: Fn(TileResult) + 'static,
{
    // ... existing code ...

    let pool = Rc::new(RefCell::new(Self {
        workers: Vec::new(),
        renderer_id,  // NEW
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

    // ... rest of function ...
}
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: FAIL with "missing field `renderer_id` in struct" (from ParallelCanvasRenderer)

**Step 4: Temporarily fix ParallelCanvasRenderer**

In `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`, find the `RenderWorkerPool::new` call and add temporary renderer_id:

```rust
// Around line 50-60, update the new() call:
let worker_pool = RenderWorkerPool::new(
    on_tile_complete,
    progress,
    "mandelbrot".to_string(),  // NEW - temporary default
)?;
```

**Step 5: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/workers/render_worker_pool.rs fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs
git commit -m "feat: add renderer_id field to RenderWorkerPool"
```

---

## Task 8: Update RenderWorkerPool - Part 2 (Handle Ready Message)

**Files:**
- Modify: `fractalwonder-ui/src/workers/render_worker_pool.rs`

**Step 1: Add send_message_to_worker helper**

In `fractalwonder-ui/src/workers/render_worker_pool.rs`, add new method after `send_no_work`:

```rust
fn send_message_to_worker(&self, worker_id: usize, msg: &MainToWorker) {
    let msg_json = serde_json::to_string(msg).expect("Failed to serialize message");
    self.workers[worker_id]
        .post_message(&JsValue::from_str(&msg_json))
        .expect("Failed to post message to worker");
}
```

**Step 2: Handle Ready message in handle_worker_message**

In the `handle_worker_message` function (around line 138), add Ready case at the top of the match:

```rust
fn handle_worker_message(&mut self, worker_id: usize, msg: WorkerToMain) {
    match msg {
        WorkerToMain::Ready => {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Worker {} ready, sending Initialize with renderer: {}",
                worker_id, self.renderer_id
            )));

            let msg = MainToWorker::Initialize {
                renderer_id: self.renderer_id.clone(),
            };
            self.send_message_to_worker(worker_id, &msg);
        }

        WorkerToMain::RequestWork { render_id } => {
            // ... existing code unchanged ...
        }

        // ... rest of match arms ...
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/render_worker_pool.rs
git commit -m "feat: handle Ready message and send Initialize to workers"
```

---

## Task 9: Add switch_renderer Method

**Files:**
- Modify: `fractalwonder-ui/src/workers/render_worker_pool.rs`

**Step 1: Implement switch_renderer method**

Add new public method to `RenderWorkerPool` impl block:

```rust
pub fn switch_renderer(&mut self, new_renderer_id: String) {
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Switching renderer from {} to {}",
        self.renderer_id, new_renderer_id
    )));

    // Update renderer_id
    self.renderer_id = new_renderer_id;

    // Terminate all workers
    for worker in &self.workers {
        worker.terminate();
    }

    // Clear pending work
    self.pending_tiles.clear();
    self.failed_tiles.clear();

    // Recreate workers (they'll send Ready → receive Initialize with new renderer_id)
    let pool_rc = self
        .self_ref
        .upgrade()
        .expect("Failed to upgrade self reference");

    match create_workers(self.workers.len(), pool_rc) {
        Ok(workers) => {
            self.workers = workers;
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
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/render_worker_pool.rs
git commit -m "feat: add switch_renderer method to RenderWorkerPool"
```

---

## Task 10: Wire Up UI Renderer Selection - Part 1 (ParallelCanvasRenderer)

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`

**Step 1: Add renderer_id parameter to new()**

In `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`, update the `new` function (around line 40):

```rust
impl ParallelCanvasRenderer {
    pub fn new(
        colorizer: Colorizer<AppData>,
        renderer_id: String,  // NEW
    ) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());

        // ... existing closure code ...

        let worker_pool = RenderWorkerPool::new(
            on_tile_complete,
            progress,
            renderer_id,  // Changed from hardcoded "mandelbrot"
        )?;

        // ... rest of function ...
    }
}
```

**Step 2: Add switch_renderer method to ParallelCanvasRenderer**

Add new method to impl block:

```rust
pub fn switch_renderer(&self, new_renderer_id: String) {
    self.worker_pool
        .borrow_mut()
        .switch_renderer(new_renderer_id);
}
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: FAIL with "this function takes 2 arguments but 1 argument was supplied" (from app.rs)

**Step 4: Commit current changes**

```bash
git add fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs
git commit -m "feat: add renderer_id parameter to ParallelCanvasRenderer"
```

---

## Task 11: Wire Up UI Renderer Selection - Part 2 (App Component)

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update create_canvas_renderer to accept renderer_id**

In `fractalwonder-ui/src/app.rs`, update the helper function (around line 16):

```rust
fn create_canvas_renderer(
    colorizer: Colorizer<AppData>,
    renderer_id: String,  // NEW
) -> Result<Rc<dyn CanvasRenderer<Scalar = f64, Data = AppData>>, JsValue> {
    Ok(Rc::new(ParallelCanvasRenderer::new(colorizer, renderer_id)?))
}
```

**Step 2: Update initial canvas_renderer creation**

In the `App` function, find the initial canvas renderer creation (around line 45-50) and update:

```rust
// Create initial canvas renderer
let initial_colorizer =
    crate::rendering::get_colorizer(&initial_state.selected_renderer_id, &initial_color_scheme_id)
        .expect("Initial renderer/color scheme must be valid");

let canvas_renderer = create_rw_signal(
    create_canvas_renderer(initial_colorizer, initial_state.selected_renderer_id.clone())
        .expect("Failed to create initial canvas renderer"),
);
```

**Step 3: Update renderer switch effect to include renderer_id**

Find the effect that handles renderer switching (around line 96-110) and update:

```rust
// Only create new renderer if renderer_id actually changed
if new_renderer_id != old_renderer_id {
    previous_renderer_id.set(new_renderer_id.clone());

    // CRITICAL: Use get_untracked() to avoid re-running when color_scheme_id changes
    let states = renderer_states.get_untracked();
    let state = states.get(&new_renderer_id).unwrap();

    // Find colorizer for restored color scheme
    let colorizer =
        crate::rendering::get_colorizer(&new_renderer_id, &state.color_scheme_id)
            .expect("Renderer/color scheme combination must be valid");

    // Create new canvas renderer with renderer_id
    let new_canvas_renderer = create_canvas_renderer(colorizer, new_renderer_id.clone())
        .expect("Failed to create canvas renderer");

    // Swap renderer
    canvas_renderer.set(new_canvas_renderer);

    // Restore viewport (untracked to avoid circular effects)
    set_viewport.set_untracked(state.viewport.clone());

    // Save immediately
    let states = renderer_states.get_untracked();
    AppState {
        selected_renderer_id: new_renderer_id.clone(),
        renderer_states: states,
    }
    .save();
}
```

**Step 4: Verify compilation**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: SUCCESS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat: wire up renderer_id in App component"
```

---

## Task 12: Build and Test

**Files:**
- None (testing only)

**Step 1: Build WASM**

Run: `cargo build --package fractalwonder-compute --target wasm32-unknown-unknown --release`

Expected: SUCCESS

**Step 2: Build UI**

Run: `cargo build --package fractalwonder-ui --target wasm32-unknown-unknown --release`

Expected: SUCCESS

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: PASS (all tests)

**Step 4: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: No warnings

**Step 5: Format code**

Run: `cargo fmt --all`

Expected: All files formatted

**Step 6: Commit formatting**

```bash
git add -A
git commit -m "style: format code with rustfmt"
```

---

## Task 13: Manual Integration Testing

**Files:**
- None (manual testing)

**Step 1: Start dev server**

Run: `trunk serve`

Expected: Server starts on http://localhost:8080

**Step 2: Open browser and test Mandelbrot**

Open: http://localhost:8080

Verify:
- [ ] Mandelbrot renders correctly
- [ ] Console shows: Workers initialized with "mandelbrot"
- [ ] Can zoom in/out
- [ ] No errors in console

**Step 3: Switch to Test Image**

Click renderer dropdown, select "Test Image"

Verify:
- [ ] Console shows: "Switching renderer from mandelbrot to test_image"
- [ ] Console shows: Workers initialized with "test_image"
- [ ] Checkerboard pattern renders (white/grey squares)
- [ ] Concentric red circles visible
- [ ] Green vertical line at x=0
- [ ] No errors in console

**Step 4: Switch back to Mandelbrot**

Click renderer dropdown, select "Mandelbrot"

Verify:
- [ ] Console shows: "Switching renderer from test_image to mandelbrot"
- [ ] Mandelbrot renders correctly again
- [ ] No errors in console

**Step 5: Test zoom with Test Image**

Select "Test Image", zoom in/out using mouse wheel

Verify:
- [ ] Pattern scales correctly
- [ ] No visual artifacts
- [ ] Console shows BigFloat viewport values at high zoom
- [ ] No errors

**Step 6: Stop server**

Press Ctrl+C

**Step 7: Document test results**

Create test results file:

```bash
cat > docs/test-results/2025-11-19-multiple-renderer-manual-test.md << 'EOF'
# Manual Test Results - Multiple Renderer Support

**Date:** 2025-11-19
**Tester:** Claude Code
**Build:** release

## Test Cases

### TC1: Mandelbrot Initial Render
- Status: PASS/FAIL
- Notes:

### TC2: Switch to Test Image
- Status: PASS/FAIL
- Notes:

### TC3: Test Image Rendering
- Status: PASS/FAIL
- Notes:

### TC4: Switch back to Mandelbrot
- Status: PASS/FAIL
- Notes:

### TC5: Test Image Zoom
- Status: PASS/FAIL
- Notes:

## Issues Found
- None / List issues

## Overall Result
PASS / FAIL
EOF
```

**Step 8: Final commit**

```bash
git add docs/test-results/2025-11-19-multiple-renderer-manual-test.md
git commit -m "test: add manual integration test results for multiple renderer support"
```

---

## Completion Checklist

- [ ] All 13 tasks completed
- [ ] All unit tests pass
- [ ] Clippy shows no warnings
- [ ] Code formatted with rustfmt
- [ ] Manual testing complete
- [ ] Both renderers (Mandelbrot, TestImage) work correctly
- [ ] Renderer switching works without errors
- [ ] Console logs show correct initialization flow
- [ ] All changes committed with descriptive messages

## Notes for Future Work

**Immediate next steps:**
1. Add more colorizers for TestImage renderer
2. Consider making zoom threshold configurable per renderer
3. Add renderer-specific parameters to Initialize message
4. Performance testing: compare TestImage vs Mandelbrot render times

**Known limitations:**
- TestImage BigFloat conversion for circle_distance loses precision (uses to_f64())
- No preloading/warmup of renderers to reduce switch delay
- Worker pool size fixed at hardware_concurrency (could make configurable per renderer)
