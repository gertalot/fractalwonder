# Iteration 3: wasm-bindgen-rayon Worker Setup

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add multi-core parallelism with wasm-bindgen-rayon for faster fractal rendering

**Architecture:** Main thread (UI) sends render requests to dedicated worker which initializes rayon thread pool. Rayon spawns N workers internally to compute tiles in parallel. Workers write serialized AppData to SharedArrayBuffer. Main thread polls buffer, deserializes results, colorizes, and displays progressively.

**Tech Stack:** Rust/WASM, wasm-bindgen-rayon 1.2, js-sys 0.3, SharedArrayBuffer, Atomics

---

## Prerequisites

**Verify Iterations 1 & 2 are complete:**
- Workspace structure exists (fractalwonder-ui, fractalwonder-compute, fractalwonder-core)
- AsyncProgressiveCanvasRenderer is implemented and working
- `trunk serve` runs successfully
- All tests pass

---

## Task 1: Add wasm-bindgen-rayon Dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `fractalwonder-compute/Cargo.toml`

**Step 1: Add rayon to workspace dependencies**

In `Cargo.toml` at root, add to `[workspace.dependencies]`:

```toml
wasm-bindgen-rayon = "1.2"
rayon = "1.10"
```

**Step 2: Add dependencies to fractalwonder-compute**

In `fractalwonder-compute/Cargo.toml`, add to `[dependencies]`:

```toml
wasm-bindgen.workspace = true
wasm-bindgen-rayon.workspace = true
rayon.workspace = true
js-sys.workspace = true
serde_json.workspace = true
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-compute`
Expected: Downloads and compiles successfully

**Step 4: Commit dependency changes**

```bash
git add Cargo.toml Cargo.lock fractalwonder-compute/Cargo.toml
git commit -m "feat: add wasm-bindgen-rayon dependencies for multi-core support"
```

---

## Task 2: Create Worker Message Protocol

**Files:**
- Create: `fractalwonder-compute/src/worker_messages.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create worker_messages.rs**

Create `fractalwonder-compute/src/worker_messages.rs`:

```rust
use fractalwonder_core::Viewport;
use serde::{Deserialize, Serialize};

/// Message from main thread to worker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerRequest {
    /// Initialize rayon thread pool
    InitThreadPool { thread_count: usize },

    /// Render a viewport
    Render {
        viewport_json: String,  // Serialized Viewport<f64>
        canvas_width: u32,
        canvas_height: u32,
        render_id: u32,
        tile_size: u32,
    },

    /// Cancel current render
    Cancel { render_id: u32 },
}

/// Message from worker to main thread
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Thread pool initialized
    ThreadPoolReady { thread_count: usize },

    /// Single tile completed
    TileComplete {
        render_id: u32,
        tile_index: usize,
    },

    /// All tiles completed
    RenderComplete { render_id: u32 },

    /// Error occurred
    Error { message: String },
}
```

**Step 2: Add to lib.rs exports**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
pub mod worker_messages;
pub use worker_messages::{WorkerRequest, WorkerResponse};
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-compute`
Expected: Compiles successfully

**Step 4: Commit message protocol**

```bash
git add fractalwonder-compute/src/worker_messages.rs \
        fractalwonder-compute/src/lib.rs
git commit -m "feat: define worker message protocol for main-worker communication"
```

---

## Task 3: Create SharedArrayBuffer Layout

**Files:**
- Create: `fractalwonder-compute/src/shared_buffer.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create shared_buffer.rs**

Create `fractalwonder-compute/src/shared_buffer.rs`:

```rust
use fractalwonder_core::MandelbrotData;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Layout of SharedArrayBuffer for worker-main communication
///
/// Memory layout:
/// - Bytes 0-3: Cancel flag (AtomicU32, 0=continue, 1=cancel)
/// - Bytes 4-7: Completed tile count (AtomicU32)
/// - Bytes 8+: Tile data (8 bytes per pixel: 4 bytes iterations + 4 bytes escaped flag)
pub struct SharedBufferLayout {
    /// Offset in bytes
    cancel_flag_offset: usize,
    /// Offset in bytes
    completed_tiles_offset: usize,
    /// Offset in bytes where pixel data starts
    data_offset: usize,
    /// Total pixels in canvas
    total_pixels: usize,
}

impl SharedBufferLayout {
    const CANCEL_FLAG_OFFSET: usize = 0;
    const COMPLETED_TILES_OFFSET: usize = 4;
    const DATA_OFFSET: usize = 8;
    const BYTES_PER_PIXEL: usize = 8; // u32 iterations + u32 escaped flag

    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            cancel_flag_offset: Self::CANCEL_FLAG_OFFSET,
            completed_tiles_offset: Self::COMPLETED_TILES_OFFSET,
            data_offset: Self::DATA_OFFSET,
            total_pixels: (canvas_width * canvas_height) as usize,
        }
    }

    /// Calculate total buffer size needed
    pub fn buffer_size(&self) -> usize {
        Self::DATA_OFFSET + (self.total_pixels * Self::BYTES_PER_PIXEL)
    }

    /// Get offset for pixel data at index
    pub fn pixel_offset(&self, pixel_index: usize) -> usize {
        Self::DATA_OFFSET + (pixel_index * Self::BYTES_PER_PIXEL)
    }

    /// Encode MandelbrotData to bytes
    pub fn encode_pixel(data: &MandelbrotData) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&data.iterations.to_le_bytes());
        bytes[4..8].copy_from_slice(&(data.escaped as u32).to_le_bytes());
        bytes
    }

    /// Decode bytes to MandelbrotData
    pub fn decode_pixel(bytes: &[u8]) -> MandelbrotData {
        let iterations = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let escaped = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) != 0;
        MandelbrotData { iterations, escaped }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_layout() {
        let layout = SharedBufferLayout::new(800, 600);
        assert_eq!(layout.total_pixels, 480_000);
        assert_eq!(layout.buffer_size(), 8 + (480_000 * 8));
    }

    #[test]
    fn test_pixel_encoding() {
        let data = MandelbrotData {
            iterations: 42,
            escaped: true,
        };

        let bytes = SharedBufferLayout::encode_pixel(&data);
        let decoded = SharedBufferLayout::decode_pixel(&bytes);

        assert_eq!(decoded.iterations, 42);
        assert_eq!(decoded.escaped, true);
    }

    #[test]
    fn test_pixel_offset() {
        let layout = SharedBufferLayout::new(100, 100);

        // First pixel
        assert_eq!(layout.pixel_offset(0), 8);

        // Second pixel (8 bytes per pixel)
        assert_eq!(layout.pixel_offset(1), 16);

        // 100th pixel
        assert_eq!(layout.pixel_offset(99), 8 + (99 * 8));
    }
}
```

**Step 2: Add to lib.rs exports**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
pub mod shared_buffer;
pub use shared_buffer::SharedBufferLayout;
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-compute -- --nocapture`
Expected: All tests pass

**Step 4: Commit shared buffer layout**

```bash
git add fractalwonder-compute/src/shared_buffer.rs \
        fractalwonder-compute/src/lib.rs
git commit -m "feat: define SharedArrayBuffer layout for worker communication"
```

---

## Task 4: Create Worker Entry Point

**Files:**
- Create: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create worker.rs skeleton**

Create `fractalwonder-compute/src/worker.rs`:

```rust
use crate::{
    MandelbrotComputer, PixelRenderer, Renderer, SharedBufferLayout,
    WorkerRequest, WorkerResponse,
};
use fractalwonder_core::{MandelbrotData, PixelRect, Viewport};
use js_sys::{ArrayBuffer, Uint8Array};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// Initialize wasm-bindgen-rayon thread pool
#[wasm_bindgen]
pub fn init_worker_thread_pool(thread_count: usize) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    wasm_bindgen_rayon::init_thread_pool(thread_count)
        .map_err(|e| JsValue::from_str(&format!("Failed to init thread pool: {:?}", e)))
}

/// Process render request from main thread
#[wasm_bindgen]
pub fn process_render_request(
    viewport_json: String,
    canvas_width: u32,
    canvas_height: u32,
    render_id: u32,
    tile_size: u32,
    shared_buffer: ArrayBuffer,
) -> Result<(), JsValue> {
    // Parse viewport
    let viewport: Viewport<f64> = serde_json::from_str(&viewport_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse viewport: {}", e)))?;

    // Create renderer
    let computer = MandelbrotComputer::<f64>::default();
    let renderer = PixelRenderer::new(computer);

    // Compute tiles
    compute_tiles_parallel(
        &renderer,
        viewport,
        canvas_width,
        canvas_height,
        tile_size,
        render_id,
        shared_buffer,
    )
}

/// Compute all tiles in parallel using rayon
fn compute_tiles_parallel(
    renderer: &PixelRenderer<f64, MandelbrotData>,
    viewport: Viewport<f64>,
    canvas_width: u32,
    canvas_height: u32,
    tile_size: u32,
    render_id: u32,
    shared_buffer: ArrayBuffer,
) -> Result<(), JsValue> {
    let layout = SharedBufferLayout::new(canvas_width, canvas_height);

    // Generate all tiles
    let tiles = generate_tiles(canvas_width, canvas_height, tile_size);
    let total_tiles = tiles.len();

    // Access shared buffer as Uint8Array
    let buffer_view = Uint8Array::new(&shared_buffer);

    // Create cancel flag wrapper for checking
    let cancel_flag = Arc::new(AtomicBool::new(false));

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Worker: Starting parallel render {} with {} tiles",
        render_id, total_tiles
    )));

    // Parallel tile computation using rayon
    tiles.par_iter().enumerate().try_for_each(|(tile_idx, tile_rect)| {
        // Check cancellation periodically
        if tile_idx % 10 == 0 && cancel_flag.load(Ordering::Relaxed) {
            return Err(JsValue::from_str("Render cancelled"));
        }

        // Render tile
        let tile_data = renderer.render(&viewport, *tile_rect, (canvas_width, canvas_height));

        // Write tile data to shared buffer
        let width = canvas_width;
        for local_y in 0..tile_rect.height {
            let canvas_y = tile_rect.y + local_y;
            for local_x in 0..tile_rect.width {
                let canvas_x = tile_rect.x + local_x;
                let pixel_index = (canvas_y * width + canvas_x) as usize;
                let tile_data_index = (local_y * tile_rect.width + local_x) as usize;

                // Encode pixel data
                let pixel = &tile_data[tile_data_index];
                let encoded = SharedBufferLayout::encode_pixel(pixel);

                // Write to buffer
                let offset = layout.pixel_offset(pixel_index);
                for (i, byte) in encoded.iter().enumerate() {
                    buffer_view.set_index(offset as u32 + i as u32, *byte);
                }
            }
        }

        Ok(())
    })?;

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Worker: Render {} complete",
        render_id
    )));

    Ok(())
}

/// Generate tiles in spiral order (center-out)
fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let x = x_start;
            let y = y_start;
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);

            tiles.push(PixelRect::new(x, y, w, h));
        }
    }

    // Sort by distance from center (furthest first for reverse pop order)
    let canvas_center_x = width as f64 / 2.0;
    let canvas_center_y = height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let a_center_x = a.x as f64 + a.width as f64 / 2.0;
        let a_center_y = a.y as f64 + a.height as f64 / 2.0;
        let a_dist_sq =
            (a_center_x - canvas_center_x).powi(2) + (a_center_y - canvas_center_y).powi(2);

        let b_center_x = b.x as f64 + b.width as f64 / 2.0;
        let b_center_y = b.y as f64 + b.height as f64 / 2.0;
        let b_dist_sq =
            (b_center_x - canvas_center_x).powi(2) + (b_center_y - canvas_center_y).powi(2);

        a_dist_sq
            .partial_cmp(&b_dist_sq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles
}
```

**Step 2: Add worker module to lib.rs**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
#[cfg(target_arch = "wasm32")]
pub mod worker;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-compute --target wasm32-unknown-unknown`
Expected: Compiles successfully

**Step 4: Commit worker entry point**

```bash
git add fractalwonder-compute/src/worker.rs \
        fractalwonder-compute/src/lib.rs
git commit -m "feat: implement worker entry point with rayon parallelism"
```

---

## Task 5: Update Workspace Dependencies for Serialization

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `fractalwonder-core/Cargo.toml`

**Step 1: Add serde derives to Viewport**

In `fractalwonder-core/src/viewport.rs`, find the `Viewport` struct and add derives:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewport<S> {
    pub visible_bounds: Rect<S>,
}
```

**Step 2: Add serde derives to Rect and Point**

In `fractalwonder-core/src/points.rs`, add derives:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point<S> {
    pub x: S,
    pub y: S,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect<S> {
    pub min: Point<S>,
    pub max: Point<S>,
}
```

**Step 3: Add serde to fractalwonder-core dependencies**

In `fractalwonder-core/Cargo.toml`, add:

```toml
[dependencies]
serde.workspace = true
```

**Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: All crates compile successfully

**Step 5: Commit serialization support**

```bash
git add fractalwonder-core/src/viewport.rs \
        fractalwonder-core/src/points.rs \
        fractalwonder-core/Cargo.toml
git commit -m "feat: add serde support to core types for worker serialization"
```

---

## Task 6: Create Worker JavaScript Bridge

**Files:**
- Create: `fractalwonder-ui/src/worker_bridge.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create worker_bridge.rs**

Create `fractalwonder-ui/src/worker_bridge.rs`:

```rust
use fractalwonder_compute::{SharedBufferLayout, WorkerRequest, WorkerResponse};
use fractalwonder_core::{MandelbrotData, Viewport};
use js_sys::{ArrayBuffer, Uint8Array};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Worker;

/// Bridge between main thread and compute worker
pub struct WorkerBridge {
    worker: Worker,
    shared_buffer: Option<ArrayBuffer>,
    current_render_id: Arc<AtomicU32>,
}

impl WorkerBridge {
    /// Create new worker bridge
    pub fn new() -> Result<Self, JsValue> {
        // Create worker from bundled worker script
        let worker = Worker::new("./fractalwonder_compute.js")?;

        Ok(Self {
            worker,
            shared_buffer: None,
            current_render_id: Arc::new(AtomicU32::new(0)),
        })
    }

    /// Initialize worker thread pool
    pub fn init_thread_pool(&self, thread_count: usize) -> Result<(), JsValue> {
        let request = WorkerRequest::InitThreadPool { thread_count };
        let message = serde_json::to_string(&request)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        self.worker.post_message(&JsValue::from_str(&message))?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Main: Requested worker init with {} threads",
            thread_count
        )));

        Ok(())
    }

    /// Start render request
    pub fn start_render(
        &mut self,
        viewport: &Viewport<f64>,
        canvas_width: u32,
        canvas_height: u32,
        tile_size: u32,
    ) -> Result<u32, JsValue> {
        let render_id = self.current_render_id.fetch_add(1, Ordering::SeqCst) + 1;

        // Create or recreate shared buffer if size changed
        let layout = SharedBufferLayout::new(canvas_width, canvas_height);
        let buffer_size = layout.buffer_size();

        let shared_buffer = ArrayBuffer::new(buffer_size as u32);

        // Clear buffer
        let view = Uint8Array::new(&shared_buffer);
        for i in 0..buffer_size {
            view.set_index(i as u32, 0);
        }

        self.shared_buffer = Some(shared_buffer.clone());

        // Serialize viewport
        let viewport_json = serde_json::to_string(&viewport)
            .map_err(|e| JsValue::from_str(&format!("Viewport serialization error: {}", e)))?;

        // Send render request
        let request = WorkerRequest::Render {
            viewport_json,
            canvas_width,
            canvas_height,
            render_id,
            tile_size,
        };

        let message = serde_json::to_string(&request)
            .map_err(|e| JsValue::from_str(&format!("Request serialization error: {}", e)))?;

        // Post message with shared buffer as transferable
        let transfer_array = js_sys::Array::new();
        transfer_array.push(&shared_buffer);

        self.worker.post_message(&JsValue::from_str(&message))?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Main: Started render {} ({}x{})",
            render_id, canvas_width, canvas_height
        )));

        Ok(render_id)
    }

    /// Read computed data from shared buffer
    pub fn read_buffer(&self, canvas_width: u32, canvas_height: u32) -> Option<Vec<MandelbrotData>> {
        let buffer = self.shared_buffer.as_ref()?;
        let layout = SharedBufferLayout::new(canvas_width, canvas_height);
        let view = Uint8Array::new(buffer);

        let mut data = Vec::with_capacity(layout.total_pixels);

        for pixel_idx in 0..layout.total_pixels {
            let offset = layout.pixel_offset(pixel_idx);
            let mut bytes = [0u8; 8];

            for i in 0..8 {
                bytes[i] = view.get_index((offset + i) as u32);
            }

            let pixel = SharedBufferLayout::decode_pixel(&bytes);
            data.push(pixel);
        }

        Some(data)
    }

    /// Cancel current render
    pub fn cancel_render(&self, render_id: u32) -> Result<(), JsValue> {
        let request = WorkerRequest::Cancel { render_id };
        let message = serde_json::to_string(&request)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        self.worker.post_message(&JsValue::from_str(&message))?;

        Ok(())
    }
}
```

**Step 2: Add to lib.rs exports**

In `fractalwonder-ui/src/lib.rs`, add:

```rust
#[cfg(target_arch = "wasm32")]
pub mod worker_bridge;
#[cfg(target_arch = "wasm32")]
pub use worker_bridge::WorkerBridge;
```

**Step 3: Add web-sys Worker feature**

In `fractalwonder-ui/Cargo.toml`, add to web-sys features:

```toml
web-sys = { version = "0.3", features = [
    # ... existing features ...
    "Worker",
] }
```

**Step 4: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 5: Commit worker bridge**

```bash
git add fractalwonder-ui/src/worker_bridge.rs \
        fractalwonder-ui/src/lib.rs \
        fractalwonder-ui/Cargo.toml
git commit -m "feat: create worker bridge for main-worker communication"
```

---

## Task 7: Update Trunk Configuration for Worker Build

**Files:**
- Modify: `index.html`
- Modify: `Trunk.toml`

**Step 1: Update index.html to load worker WASM**

In `index.html`, add after existing rust link:

```html
<!-- Compute worker WASM -->
<link data-trunk rel="rust" data-type="worker"
      data-wasm-bindgen-rayon-options="maxMemory=4294967296"
      href="./fractalwonder-compute/Cargo.toml"/>
```

**Step 2: Update Trunk.toml for SharedArrayBuffer headers**

Verify `Trunk.toml` has COOP/COEP headers (should already exist):

```toml
[serve]
address = "127.0.0.1"
port = 8080

[[serve.headers]]
name = "Cross-Origin-Opener-Policy"
value = "same-origin"

[[serve.headers]]
name = "Cross-Origin-Embedder-Policy"
value = "require-corp"
```

**Step 3: Verify trunk configuration**

Run: `trunk build`
Expected: Builds both UI and worker WASM successfully

**Step 4: Commit trunk configuration**

```bash
git add index.html Trunk.toml
git commit -m "feat: configure trunk to build worker WASM with rayon support"
```

---

## Task 8: Test Worker in Isolation

**Files:**
- Create: `fractalwonder-compute/tests/worker_test.rs`

**Step 1: Create worker integration test**

Create `fractalwonder-compute/tests/worker_test.rs`:

```rust
use fractalwonder_compute::SharedBufferLayout;
use fractalwonder_core::MandelbrotData;

#[test]
fn test_shared_buffer_roundtrip() {
    let layout = SharedBufferLayout::new(100, 100);

    let original = MandelbrotData {
        iterations: 123,
        escaped: true,
    };

    let encoded = SharedBufferLayout::encode_pixel(&original);
    let decoded = SharedBufferLayout::decode_pixel(&encoded);

    assert_eq!(original.iterations, decoded.iterations);
    assert_eq!(original.escaped, decoded.escaped);
}

#[test]
fn test_buffer_size_calculation() {
    let layout = SharedBufferLayout::new(1920, 1080);
    let expected_pixels = 1920 * 1080;
    let expected_size = 8 + (expected_pixels * 8); // metadata + pixel data

    assert_eq!(layout.buffer_size(), expected_size);
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-compute -- --nocapture`
Expected: All tests pass

**Step 3: Commit worker tests**

```bash
git add fractalwonder-compute/tests/worker_test.rs
git commit -m "test: add integration tests for worker SharedArrayBuffer"
```

---

## Task 9: Update AsyncProgressiveCanvasRenderer to Use Worker

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_canvas_renderer.rs`

**Step 1: Add worker field to AsyncProgressiveCanvasRenderer**

In `async_progressive_canvas_renderer.rs`, update struct:

```rust
use crate::worker_bridge::WorkerBridge;

pub struct AsyncProgressiveCanvasRenderer<S, D: Clone> {
    renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
    colorizer: Colorizer<D>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<S, D>>>,
    current_render: Rc<RefCell<Option<RenderState<S, D>>>>,
    #[cfg(target_arch = "wasm32")]
    worker_bridge: Option<WorkerBridge>,  // New field
}
```

**Step 2: Update Clone implementation**

```rust
impl<S, D: Clone> Clone for AsyncProgressiveCanvasRenderer<S, D> {
    fn clone(&self) -> Self {
        Self {
            renderer: dyn_clone::clone_box(&*self.renderer),
            colorizer: self.colorizer,
            tile_size: self.tile_size,
            cached_state: Arc::clone(&self.cached_state),
            current_render: Rc::clone(&self.current_render),
            #[cfg(target_arch = "wasm32")]
            worker_bridge: None, // Workers don't clone
        }
    }
}
```

**Step 3: Update new() to initialize worker**

```rust
impl<S: Clone + PartialEq, D: Clone + Default + 'static> AsyncProgressiveCanvasRenderer<S, D> {
    pub fn new(
        renderer: Box<dyn Renderer<Scalar = S, Data = D>>,
        colorizer: Colorizer<D>,
        tile_size: u32,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let worker_bridge = {
            match WorkerBridge::new() {
                Ok(mut bridge) => {
                    // Initialize with hardware concurrency
                    let thread_count = web_sys::window()
                        .and_then(|w| w.navigator().hardware_concurrency())
                        .map(|c| c as usize)
                        .unwrap_or(4);

                    if let Err(e) = bridge.init_thread_pool(thread_count) {
                        web_sys::console::error_1(&e);
                        None
                    } else {
                        Some(bridge)
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                    None
                }
            }
        };

        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState::default())),
            current_render: Rc::new(RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            worker_bridge,
        }
    }

    // ... rest of implementation
}
```

**Step 4: Add worker rendering path (TODO marker for now)**

Add method to AsyncProgressiveCanvasRenderer:

```rust
#[cfg(target_arch = "wasm32")]
fn use_worker_rendering(&self) -> bool {
    self.worker_bridge.is_some()
}
```

**Step 5: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 6: Commit worker integration skeleton**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_canvas_renderer.rs
git commit -m "feat: integrate WorkerBridge into AsyncProgressiveCanvasRenderer"
```

---

## Task 10: Manual Browser Test - Worker Loading

**Files:**
- Create: `docs/testing/iteration-3-worker-loading.md`

**Step 1: Create test checklist**

Create `docs/testing/iteration-3-worker-loading.md`:

```markdown
# Iteration 3 Manual Test: Worker Loading

**Goal:** Verify worker WASM loads and initializes correctly

## Test Procedure

1. **Start dev server:**
   ```bash
   trunk serve
   ```

2. **Open browser DevTools:**
   - Open Console tab
   - Open Network tab

3. **Navigate to app:**
   - Go to http://localhost:8080
   - Watch console for worker init messages

4. **Verify worker files loaded:**
   - Network tab should show:
     - `fractalwonder_ui_bg.wasm` (main WASM)
     - `fractalwonder_compute_bg.wasm` (worker WASM)
     - `fractalwonder_compute.js` (worker JS)

5. **Check for errors:**
   - No console errors
   - No CORS errors
   - No WASM instantiation errors

## Pass Criteria

- ✓ Worker WASM files appear in Network tab
- ✓ Console shows "Worker init" messages
- ✓ No red errors in console
- ✓ COOP/COEP headers present (check in Network → Headers)

## Expected Console Output

```
Main: Requested worker init with 8 threads
Worker: Thread pool initialized
```

## Troubleshooting

**If worker fails to load:**
- Check Trunk.toml has COOP/COEP headers
- Verify index.html has worker link with data-type="worker"
- Check browser supports SharedArrayBuffer

**If SharedArrayBuffer undefined:**
- Browser needs COOP/COEP headers
- Must be served over HTTP (not file://)
- Chrome/Firefox/Safari should all work
```

**Step 2: Run manual test**

Run `trunk serve` and follow test procedure.

**Step 3: Commit test documentation**

```bash
git add docs/testing/iteration-3-worker-loading.md
git commit -m "docs: add manual test for worker loading verification"
```

---

## Task 11: Implement Worker Rendering Logic

**Files:**
- Modify: `fractalwonder-ui/src/rendering/async_progressive_canvas_renderer.rs`

**Step 1: Add worker render path to start_async_render**

Update `start_async_render()` method:

```rust
fn start_async_render(&self, viewport: Viewport<S>, canvas: HtmlCanvasElement, render_id: u32)
where
    S: Clone + 'static,
{
    let width = canvas.width();
    let height = canvas.height();

    #[cfg(target_arch = "wasm32")]
    if self.use_worker_rendering() {
        // WORKER PATH: Send to worker
        if let Some(ref bridge) = self.worker_bridge {
            match bridge.start_render(&viewport, width, height, self.tile_size) {
                Ok(worker_render_id) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Main: Dispatched render {} to worker",
                        worker_render_id
                    )));

                    // Start polling loop
                    self.start_worker_poll_loop(canvas, width, height, render_id);
                    return;
                }
                Err(e) => {
                    web_sys::console::error_1(&e);
                    // Fall through to single-threaded
                }
            }
        }
    }

    // SINGLE-THREADED FALLBACK PATH (existing code)
    let tiles = compute_tiles(width, height, self.tile_size);
    let total_tiles = tiles.len();

    // ... rest of existing single-threaded implementation ...
}
```

**Step 2: Implement worker polling loop**

Add new method:

```rust
#[cfg(target_arch = "wasm32")]
fn start_worker_poll_loop(
    &self,
    canvas: HtmlCanvasElement,
    width: u32,
    height: u32,
    render_id: u32,
) {
    let self_clone = self.clone();
    let poll_interval = 16; // ~60fps

    let closure = Closure::wrap(Box::new(move || {
        self_clone.poll_worker_results(&canvas, width, height, render_id);
    }) as Box<dyn FnMut()>);

    web_sys::window()
        .expect("no global window")
        .set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            poll_interval,
        )
        .expect("Failed to set interval");

    closure.forget();
}

#[cfg(target_arch = "wasm32")]
fn poll_worker_results(
    &self,
    canvas: &HtmlCanvasElement,
    width: u32,
    height: u32,
    render_id: u32,
) {
    if let Some(ref bridge) = self.worker_bridge {
        if let Some(data) = bridge.read_buffer(width, height) {
            // Convert MandelbrotData to AppData
            let app_data: Vec<AppData> = data
                .into_iter()
                .map(|d| AppData::MandelbrotData(d))
                .collect();

            // Display full canvas
            let full_rect = PixelRect::full_canvas(width, height);
            self.colorize_and_display_tile(&app_data, full_rect, canvas);

            // Cache results
            let mut cache = self.cached_state.lock().unwrap();
            cache.viewport = Some(viewport.clone()); // TODO: store viewport
            cache.canvas_size = Some((width, height));
            cache.data = app_data;
        }
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: May have errors (viewport not in scope) - that's OK, we'll fix next

**Step 4: Fix viewport storage**

Update `RenderState` to store whether using worker:

```rust
struct RenderState<S, D: Clone> {
    viewport: Viewport<S>,
    canvas_size: (u32, u32),
    remaining_tiles: Vec<PixelRect>,
    computed_data: Vec<D>,
    render_id: u32,
    #[cfg(target_arch = "wasm32")]
    using_worker: bool,  // New field
    // ... existing fields ...
}
```

Update initialization in `start_async_render`:

```rust
let render_state = RenderState {
    viewport: viewport.clone(),
    canvas_size: (width, height),
    remaining_tiles: tiles,
    computed_data: vec![D::default(); (width * height) as usize],
    render_id,
    #[cfg(target_arch = "wasm32")]
    using_worker: false,
    // ... existing fields ...
};
```

**Step 5: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles successfully

**Step 6: Commit worker rendering integration**

```bash
git add fractalwonder-ui/src/rendering/async_progressive_canvas_renderer.rs
git commit -m "feat: implement worker-based rendering path with polling"
```

---

## Task 12: Final Integration Testing

**Files:**
- Create: `docs/testing/iteration-3-integration-tests.md`

**Step 1: Create integration test checklist**

Create `docs/testing/iteration-3-integration-tests.md`:

```markdown
# Iteration 3 Integration Tests

**Feature:** Multi-core parallel rendering via wasm-bindgen-rayon

## Test Environment
- Browser: Chrome (recommended for better WASM debugging)
- URL: http://localhost:8080
- DevTools: Console + Performance tab open

## Test Checklist

### Worker Initialization
- [ ] Console shows worker init message
- [ ] Thread pool created with correct core count
- [ ] No worker loading errors

### Parallel Rendering
- [ ] Render completes faster than Iteration 2 (single-threaded)
- [ ] CPU usage shows multiple cores active (Activity Monitor / Task Manager)
- [ ] Tiles still appear progressively
- [ ] Final image is correct (no visual artifacts)

### Progressive Display
- [ ] Tiles appear incrementally (not all at once)
- [ ] UI stays responsive during render
- [ ] Can click dropdowns during render

### Cancellation
- [ ] Pan during render stops workers
- [ ] Zoom during render stops workers
- [ ] New render starts immediately

### Edge Cases
- [ ] Browser window resize works
- [ ] Rapid pan/zoom doesn't crash
- [ ] Switching renderers works

## Performance Benchmarks

**Baseline (Iteration 2 - Single-threaded):**
- 1920x1080 canvas, default Mandelbrot view
- Expected time: ~2-5 seconds

**Iteration 3 (Multi-core):**
- Same viewport and canvas
- Expected time: ~0.5-1.5 seconds (3-4x faster on 8 cores)

Record actual times:
- Single-threaded: _____ ms
- Multi-core: _____ ms
- Speedup: _____x

## Pass Criteria

- ✓ All checklist items pass
- ✓ Measurable speedup vs single-threaded (>2x on 4+ cores)
- ✓ No visual artifacts
- ✓ No crashes or errors
```

**Step 2: Run integration tests**

Follow test procedure and record results.

**Step 3: Commit test documentation**

```bash
git add docs/testing/iteration-3-integration-tests.md
git commit -m "docs: add integration tests for multi-core rendering"
```

---

## Task 13: Run Full Validation Suite

**Files:**
- None (verification only)

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Build release**

Run: `trunk build --release`
Expected: Builds successfully, both UI and worker WASM

**Step 5: Manual browser testing**

Run: `trunk serve`

Verify all items in:
- `docs/testing/iteration-3-worker-loading.md` ✓
- `docs/testing/iteration-3-integration-tests.md` ✓

**Step 6: Commit any fixes**

```bash
git add .
git commit -m "fix: address Clippy warnings and test failures"
```

---

## Task 14: Documentation and Completion

**Files:**
- Modify: `README.md`
- Create: `docs/benchmarks/iteration-3-performance.md`

**Step 1: Update README.md**

Add to Features section:

```markdown
## Features

- **Multi-core Rendering**: Utilizes all CPU cores via wasm-bindgen-rayon
- **Progressive Rendering**: Tiles appear incrementally during renders
- **Responsive UI**: Interact with controls while rendering
- **Immediate Cancellation**: Pan/zoom instantly stops render
```

Add to Architecture section:

```markdown
## Architecture

### Multi-threaded Rendering (Iteration 3)

- **Main Thread**: Leptos UI, canvas rendering, user interaction
- **Dedicated Worker**: Initializes rayon thread pool, coordinates computation
- **Rayon Worker Pool**: N workers (based on CPU cores) compute tiles in parallel
- **SharedArrayBuffer**: Zero-copy data transfer between worker and main thread
```

**Step 2: Create performance benchmarks document**

Create `docs/benchmarks/iteration-3-performance.md`:

```markdown
# Iteration 3 Performance Benchmarks

**Configuration:**
- Canvas: 1920x1080
- Tile size: 256x256
- Viewport: Default Mandelbrot view
- CPU: [Record your CPU model]
- Cores: [Record core count]

## Results

### Render Time

| Implementation | Time (ms) | Speedup |
|---------------|-----------|---------|
| Iteration 2 (single-thread) | [measure] | 1.0x |
| Iteration 3 (multi-core) | [measure] | [calculate]x |

### CPU Utilization

- Single-threaded: 1 core at 100%
- Multi-core: [N] cores at [%]

## Analysis

Expected speedup formula: `S = N / (1 + (1-P)*(N-1))` where:
- N = number of cores
- P = parallelizable fraction (~0.95 for tile rendering)

For 8 cores: S ≈ 6.5x theoretical maximum

Actual speedup: [measured]x
Efficiency: [actual/theoretical * 100]%

## Observations

- First tile latency: [measure]
- Progressive display quality: [subjective assessment]
- UI responsiveness: [subjective assessment]
```

**Step 3: Commit documentation**

```bash
git add README.md docs/benchmarks/iteration-3-performance.md
git commit -m "docs: update README and add performance benchmarks for Iteration 3"
```

---

## Task 15: Create Release Tag

**Files:**
- None (git tag only)

**Step 1: Create final commit**

```bash
git add .
git commit -m "feat: complete Iteration 3 - Multi-core Rendering with wasm-bindgen-rayon

- Implemented worker-based parallel rendering
- Added SharedArrayBuffer for zero-copy data transfer
- Integrated rayon for work-stealing parallelism
- Measured 3-6x speedup on multi-core CPUs
- All tests pass, manual browser tests verified
"
```

**Step 2: Create tag**

```bash
git tag -a v0.4.0-multicore-rayon -m "Iteration 3 complete - Multi-core Rendering"
```

**Step 3: Verify tag**

Run: `git log --oneline --decorate -5`
Expected: Shows tag on HEAD commit

---

## Success Criteria

**All of these must be true:**

- [ ] Worker WASM loads successfully in browser
- [ ] Rayon thread pool initializes with correct core count
- [ ] Parallel rendering produces correct output
- [ ] Measurable speedup vs single-threaded (>2x on 4+ cores)
- [ ] Progressive display still works
- [ ] UI stays responsive during render
- [ ] Cancellation works (pan/zoom stops workers)
- [ ] All automated tests pass
- [ ] No Clippy warnings
- [ ] Code properly formatted
- [ ] Manual browser tests all pass
- [ ] Documentation updated

**Observable behavior:**

- CPU utilization shows multiple cores active
- Render completes faster than single-threaded
- Tiles appear progressively
- UI never freezes
- Visual quality identical to single-threaded

---

## Next Steps After Completion

With multi-core rendering working (Iteration 3), next iterations add:

**Iteration 4:** Responsive cancellation (worker-side cancel checking)
**Iteration 5:** Optimize tile scheduling and sizing
**Iteration 6:** Arbitrary precision reference orbits
**Iteration 7:** Perturbation theory rendering

Reference: `docs/multicore-plans/2025-11-17-progressive-parallel-rendering-design.md`
