# Multiple Renderer Support Design

**Date:** 2025-11-19
**Status:** Design Complete

## Problem

Workers hardcode `AdaptiveMandelbrotRenderer::new(1e10)` in worker.rs line 12. The UI can select different renderers (test_image, mandelbrot), but workers cannot execute them because the renderer is compiled into the WASM at build time.

## Goals

1. Support runtime selection of renderers (TestImage, Mandelbrot)
2. Make TestImageComputer work with BigFloat (like MandelbrotComputer)
3. Workers recreate when renderer changes (clean switching, no caching)
4. Single WASM file with all renderers

## Solution Overview

Add a protocol for workers to receive renderer selection at startup:
- Workers signal when ready with `Ready` message
- Main thread responds with `Initialize { renderer_id }` message
- Workers create the specified renderer using factory function
- Workers then request work as normal

## Design Details

### 1. Message Protocol Changes

**Add Ready Message** (workers signal initialization complete):

```rust
// In fractalwonder-compute/src/messages.rs
pub enum WorkerToMain {
    Ready,  // NEW - worker initialized and ready for commands
    RequestWork { render_id: Option<u32> },
    TileComplete { render_id: u32, tile: PixelRect, data: Vec<AppData>, compute_time_ms: f64 },
    Error { render_id: Option<u32>, tile: Option<PixelRect>, error: String },
}
```

**Add Initialize Message** (main tells workers which renderer):

```rust
pub enum MainToWorker {
    Initialize { renderer_id: String },  // NEW - tells worker which renderer to create
    RenderTile { render_id: u32, viewport_json: String, tile: PixelRect, canvas_width: u32, canvas_height: u32 },
    NoWork,
    Terminate,
}
```

### 2. Worker Initialization Flow

**Current flow:**
1. Worker loads WASM → immediately sends `RequestWork { render_id: None }`
2. Main responds with RenderTile or NoWork

**New flow:**
1. Worker loads WASM → sends `Ready`
2. Main responds with `Initialize { renderer_id }`
3. Worker creates renderer → sends `RequestWork { render_id: None }`
4. Main responds with RenderTile or NoWork

**Implementation in worker.rs:**

```rust
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    // No renderer yet - wait for Initialize
    let renderer: Rc<RefCell<Option<Box<dyn Renderer<Scalar=BigFloat, Data=AppData>>>>>
        = Rc::new(RefCell::new(None));

    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        handle_worker_message(&renderer, e.data())
    }));

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready - wait for Initialize
    send_message(&WorkerToMain::Ready);
}

fn handle_worker_message(
    renderer: &Rc<RefCell<Option<Box<dyn Renderer<...>>>>>,
    data: JsValue,
) -> Result<(), JsValue> {
    let msg: MainToWorker = serde_json::from_str(&msg_str)?;

    match msg {
        MainToWorker::Initialize { renderer_id } => {
            let new_renderer = create_renderer(&renderer_id)?;
            *renderer.borrow_mut() = Some(new_renderer);
            send_message(&WorkerToMain::RequestWork { render_id: None });
        }
        MainToWorker::RenderTile { ... } => {
            let r = renderer.borrow();
            let r = r.as_ref().expect("Renderer not initialized");
            handle_render_tile(r, ...)?;
        }
        // ... other cases
    }
    Ok(())
}
```

### 3. Renderer Factory Function

**Create renderers based on ID:**

```rust
// In fractalwonder-compute/src/worker.rs or new renderer_factory.rs
fn create_renderer(renderer_id: &str) -> Result<Box<dyn Renderer<Scalar=BigFloat, Data=AppData>>, JsValue> {
    match renderer_id {
        "test_image" => {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(
                pixel_renderer,
                |d| AppData::TestImageData(*d)
            );
            Ok(Box::new(app_renderer))
        }
        "mandelbrot" => {
            Ok(Box::new(AdaptiveMandelbrotRenderer::new(1e10)))
        }
        _ => Err(JsValue::from_str(&format!("Unknown renderer: {}", renderer_id))),
    }
}
```

**Alternative: Use RenderConfig**

Add factory to RenderConfig for better maintainability:

```rust
// In fractalwonder-compute/src/render_config.rs
pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: &'static [ColorScheme],
    pub default_color_scheme_id: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
    pub create_renderer: fn() -> Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>,  // NEW
}

pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        // ... existing fields
        create_renderer: || {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            Box::new(AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d)))
        },
    },
    RenderConfig {
        id: "mandelbrot",
        // ... existing fields
        create_renderer: || Box::new(AdaptiveMandelbrotRenderer::new(1e10)),
    },
];

// Worker uses config
fn create_renderer(renderer_id: &str) -> Result<Box<dyn Renderer<Scalar=BigFloat, Data=AppData>>, JsValue> {
    get_config(renderer_id)
        .map(|config| (config.create_renderer)())
        .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))
}
```

### 4. Make TestImageComputer Generic

**Follow MandelbrotComputer pattern:**

```rust
// In fractalwonder-compute/src/computers/test_image.rs
pub struct TestImageComputer<T> {
    checkerboard_size: T,
    circle_radius_step: T,
    circle_line_thickness: T,
}

impl<T> TestImageComputer<T>
where
    T: Clone + num_traits::FromPrimitive,
{
    pub fn new() -> Self {
        Self {
            checkerboard_size: T::from_f64(5.0).unwrap(),
            circle_radius_step: T::from_f64(10.0).unwrap(),
            circle_line_thickness: T::from_f64(0.1).unwrap(),
        }
    }
}

impl<T> ImagePointComputer for TestImageComputer<T>
where
    T: Clone + num_traits::Float,  // Provides sqrt, floor, abs, round, arithmetic
{
    type Scalar = T;
    type Data = TestImageData;

    fn natural_bounds(&self) -> Rect<T> {
        Rect::new(
            Point::new(T::from_f64(-50.0).unwrap(), T::from_f64(-50.0).unwrap()),
            Point::new(T::from_f64(50.0).unwrap(), T::from_f64(50.0).unwrap())
        )
    }

    fn compute(&self, coord: Point<T>, _viewport: &Viewport<T>) -> TestImageData {
        let x = coord.x().clone();
        let y = coord.y().clone();

        // Circle distance calculation
        let distance = (x.clone() * x.clone() + y.clone() * y.clone()).sqrt();
        let nearest_ring = (distance.clone() / self.circle_radius_step.clone()).round();
        let ring_distance = (distance - nearest_ring.clone() * self.circle_radius_step.clone()).abs();

        let circle_distance = if ring_distance < self.circle_line_thickness.clone() / T::from_f64(2.0).unwrap()
            && nearest_ring > T::zero() {
            ring_distance
        } else {
            ring_distance + T::one()
        };

        // Vertical line check
        if x.abs() < self.circle_line_thickness.clone() {
            return TestImageData::new(false, 0.0);
        }

        // Checkerboard pattern
        let square_x = (x / self.checkerboard_size.clone()).floor().to_i32().unwrap();
        let square_y = (y / self.checkerboard_size.clone()).floor().to_i32().unwrap();
        let is_light = (square_x + square_y) % 2 == 0;

        TestImageData::new(is_light, circle_distance.to_f64().unwrap())
    }
}

// Update RendererInfo to work with f64 for UI display
impl RendererInfo for TestImageComputer<f64> {
    type Scalar = f64;
    // ... existing implementation
}
```

### 5. Main Thread Changes

**Update RenderWorkerPool:**

```rust
// In fractalwonder-ui/src/workers/render_worker_pool.rs
pub struct RenderWorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,  // NEW - which renderer workers use
    pending_tiles: VecDeque<TileRequest>,
    // ... existing fields
}

impl RenderWorkerPool {
    pub fn new(
        on_tile_complete: F,
        progress_signal: RwSignal<RenderProgress>,
        renderer_id: String,  // NEW
    ) -> Result<Rc<RefCell<Self>>, JsValue> {
        // ... create pool with renderer_id
    }

    fn handle_worker_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => {  // NEW
                let msg = MainToWorker::Initialize {
                    renderer_id: self.renderer_id.clone(),
                };
                self.send_message_to_worker(worker_id, &msg);
            }

            WorkerToMain::RequestWork { render_id } => {
                // Existing logic unchanged
            }
            // ... other cases
        }
    }

    pub fn switch_renderer(&mut self, new_renderer_id: String) {
        self.renderer_id = new_renderer_id;

        // Terminate all workers
        for worker in &self.workers {
            worker.terminate();
        }

        // Recreate workers (they'll send Ready → receive Initialize with new renderer_id)
        self.workers = create_workers(self.workers.len(), self.self_ref.upgrade().unwrap())?;
    }

    fn send_message_to_worker(&self, worker_id: usize, msg: &MainToWorker) {
        let msg_json = serde_json::to_string(msg).expect("Failed to serialize message");
        self.workers[worker_id]
            .post_message(&JsValue::from_str(&msg_json))
            .expect("Failed to post message");
    }
}
```

**Update ParallelCanvasRenderer:**

Pass renderer_id to worker pool constructor. When renderer changes in UI, call `switch_renderer()` on the pool.

## Implementation Order

1. Add `Ready` message to `WorkerToMain` enum
2. Add `Initialize` message to `MainToWorker` enum
3. Make `TestImageComputer<T>` generic over scalar type
4. Add `create_renderer` to `RenderConfig` (or standalone factory)
5. Update `worker.rs` to use Ready/Initialize protocol
6. Update `RenderWorkerPool` to track `renderer_id` and handle Ready
7. Add `switch_renderer()` method to pool
8. Wire up UI renderer selection to call `switch_renderer()`
9. Test switching between test_image and mandelbrot
10. Update tests for generic `TestImageComputer<T>`

## Trade-offs

**Chosen approach:** Worker recreation on renderer switch
- Pro: Simple, clean state - no stale renderer issues
- Pro: Reuses existing termination/recreation logic
- Con: Brief pause when switching (acceptable per requirements)

**Rejected approach:** Keep workers alive, send new renderer in messages
- Pro: No recreation overhead
- Con: Complex state management (what if worker is mid-tile?)
- Con: Memory overhead (multiple renderers in WASM)

## Testing Strategy

1. Unit tests for generic `TestImageComputer<f64>` and `TestImageComputer<BigFloat>`
2. Integration test for worker protocol (mock main/worker message exchange)
3. Manual testing: switch between renderers in UI, verify:
   - TestImage renders correctly (checkerboard + circles)
   - Mandelbrot renders correctly at low and high zoom
   - No visual artifacts during switch
   - Console shows no errors
4. Performance: verify TestImage renders at similar speed to Mandelbrot at low zoom

## Future Enhancements

- Add more renderers (Julia set, Burning Ship, etc.)
- Make zoom threshold configurable per renderer in RenderConfig
- Support renderer-specific parameters in Initialize message
- Add renderer warmup/preload option to reduce switch delay
