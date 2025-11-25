# Worker Infrastructure Design

## Overview

Move fractal computation from main thread to Web Workers for parallel rendering. Workers load separate WASM instances of the compute module and communicate via JSON message passing.

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| WASM loading | Separate JS loader | Workers can't share main thread's WASM instance |
| Message types location | `fractalwonder-core` | Both compute and UI need these types |
| Work distribution | Pull-based | Workers request work when idle; better load balancing for variable tile compute times |
| Cancellation | Terminate & recreate | Immediate cancellation; avoids waiting for slow deep-zoom tiles |

## Message Protocol

### MainToWorker (main thread → worker)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    Initialize { renderer_id: String },
    RenderTile {
        render_id: u32,
        viewport_json: String,  // JSON-serialized Viewport (preserves BigFloat precision)
        tile: PixelRect,        // For placing result on canvas
    },
    NoWork,
    Terminate,
}
```

### WorkerToMain (worker → main thread)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    Ready,
    RequestWork { render_id: Option<u32> },
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    },
    Error { message: String },
}
```

## Worker Lifecycle

```
┌─────────────┐     Ready      ┌─────────────┐
│   Created   │ ─────────────► │   Main      │
│   (JS load) │                │   Thread    │
└─────────────┘                └──────┬──────┘
                                      │
                               Initialize{renderer_id}
                                      │
                                      ▼
┌─────────────┐  RequestWork   ┌─────────────┐
│   Worker    │ ◄───────────── │   Worker    │
│   Idle      │                │ Initialized │
└──────┬──────┘                └─────────────┘
       │
       │ RenderTile{...}
       ▼
┌─────────────┐  TileComplete  ┌─────────────┐
│   Worker    │ ─────────────► │   Main      │
│  Computing  │                │   Thread    │
└──────┬──────┘                └─────────────┘
       │
       │ RequestWork
       ▼
   (back to Idle or next tile)
```

## Component Structure

### fractalwonder-core/src/messages.rs

Message enums shared by compute and UI crates.

### fractalwonder-compute/src/worker.rs

Worker entry point:

```rust
#[wasm_bindgen]
pub fn init_message_worker() {
    // Set up onmessage handler
    // On Initialize → create renderer, send RequestWork
    // On RenderTile → compute, send TileComplete, send RequestWork
    // On NoWork → idle
    // On Terminate → close
}
```

### message-compute-worker.js

JS loader copied to dist/:

```javascript
import init, { init_message_worker } from './fractalwonder-compute.js';

async function run() {
    await init();
    init_message_worker();
}

run();
```

### fractalwonder-ui/src/workers/worker_pool.rs

Manages worker lifecycle and work distribution:

```rust
pub struct WorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    current_viewport: Viewport,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    progress: RwSignal<RenderProgress>,
}

impl WorkerPool {
    pub fn new(...) -> Result<Rc<RefCell<Self>>, JsValue>;
    pub fn start_render(&mut self, viewport, canvas_size, tiles);
    pub fn cancel(&mut self);  // Terminate and recreate workers
    pub fn switch_renderer(&mut self, renderer_id: &str);
}
```

### fractalwonder-ui/src/rendering/parallel_renderer.rs

Replaces AsyncProgressiveRenderer:

```rust
pub struct ParallelRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
}

impl ParallelRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue>;
    pub fn progress(&self) -> RwSignal<RenderProgress>;
    pub fn cancel(&self);
    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement);
}
```

## File Changes

### New Files

| File | Location | Purpose |
|------|----------|---------|
| `messages.rs` | core | Message enums |
| `worker.rs` | compute | Worker entry point |
| `message-compute-worker.js` | root | JS loader |
| `worker_pool.rs` | ui/workers | Worker management |
| `parallel_renderer.rs` | ui/rendering | Parallel rendering orchestration |

### Modified Files

| File | Change |
|------|--------|
| `fractalwonder-core/src/compute_data.rs` | Add `Serialize, Deserialize` derives |
| `fractalwonder-core/src/lib.rs` | Export message types |
| `fractalwonder-compute/Cargo.toml` | Add `crate-type = ["cdylib"]`, wasm-bindgen |
| `index.html` | Trunk directive to copy worker JS |
| `interactive_canvas.rs` | Use `ParallelRenderer` |

### Deletable After Migration

- `async_progressive_renderer.rs`

## Existing Code Reused

- `tiles.rs` - tile generation, center-out ordering
- `colorizers/` - all colorization logic
- `canvas_utils.rs` - drawing utilities
- `RenderProgress` - progress tracking
- `Renderer` trait - unchanged, workers use same interface

## Data Flow

```
User interaction
       │
       ▼
ParallelRenderer.render()
       │
       ├─► cancel() existing render
       │
       ├─► generate_tiles() (center-out)
       │
       └─► worker_pool.start_render()
                  │
                  ▼
           Queue tiles, wake workers
                  │
    ┌─────────────┼─────────────┐
    ▼             ▼             ▼
 Worker 1     Worker 2     Worker N
    │             │             │
    └──────┬──────┴──────┬──────┘
           │             │
     TileComplete   TileComplete
           │             │
           ▼             ▼
    on_tile_complete callback
           │
           ├─► colorize(data)
           │
           └─► draw_pixels_to_canvas()
```
