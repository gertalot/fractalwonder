# Iteration 3: Manual Web Workers for Parallel Rendering

**Date:** 2025-11-17
**Status:** Design Phase
**Approach:** Manual Web Workers (NOT wasm-bindgen-rayon)

---

## Executive Summary

After thorough research, we determined that **Trunk + wasm-bindgen-rayon is not a proven, documented approach**. Instead, we will use **manual Web Workers** with atomic work distribution, which is:

- ✅ Proven for fractal rendering (multiple working examples)
- ✅ Compatible with Trunk + Leptos (confirmed)
- ✅ Simpler to debug and understand
- ✅ No nightly Rust or complex build configuration
- ✅ Lower risk, faster to ship

**Key Reference Projects:**
- [sgasse/wasm_worker_interaction](https://github.com/sgasse/wasm_worker_interaction) - Rust web worker examples
- [ScottLogic Multi-threaded WASM](https://blog.scottlogic.com/2019/07/15/multithreaded-webassembly.html) - Fractal rendering
- [larsch/webfractals](https://github.com/larsch/webfractals) - Progressive fractal rendering

---

## Architecture Overview

### Thread Model

```
Main Thread (Browser UI Thread)
├─ Leptos UI (fractalwonder-ui WASM)
├─ Canvas rendering (colorization)
├─ Creates SharedArrayBuffer
├─ Spawns N Web Workers (N = navigator.hardwareConcurrency)
└─ Polls SharedArrayBuffer for results

Worker Threads (N workers, e.g., 8 on 8-core machine)
├─ Each loads fractalwonder-compute WASM
├─ Each shares the same memory via SharedArrayBuffer
├─ Compete for work via atomic counter
├─ Compute tiles and write results to shared memory
└─ No coordination between workers (fully independent)
```

### Key Insight: Work-Stealing Pattern

Instead of pre-assigning tiles to workers (static partitioning), we use a **shared atomic counter**:

1. SharedArrayBuffer contains:
   - Atomic counter at offset 0 (next pixel/tile index to compute)
   - Tile data starting at offset 8

2. Each worker atomically increments the counter to get its next work item

3. Workers run until counter >= total work items

This automatically balances work across cores without manual coordination.

---

## Data Structures

### SharedArrayBuffer Layout

```
Bytes 0-3:   Global tile index (AtomicU32)
Bytes 4-7:   Render ID (AtomicU32) - for cancellation
Bytes 8+:    Serialized tile data
```

**Per-pixel data (8 bytes):**
```
Bytes 0-3:   iterations (u32)
Bytes 4-7:   escaped flag (u32, 0=false, 1=true)
```

### Rust Types

```rust
// Main thread (fractalwonder-ui)
pub struct WorkerPool {
    workers: Vec<web_sys::Worker>,
    shared_buffer: js_sys::ArrayBuffer,
    current_render_id: Arc<AtomicU32>,
}

// Shared between main and worker
pub struct SharedBufferLayout {
    tile_index_offset: usize,      // 0
    render_id_offset: usize,        // 4
    data_offset: usize,             // 8
    bytes_per_pixel: usize,         // 8
    total_pixels: usize,
}

// Worker thread (fractalwonder-compute)
pub struct WorkerConfig {
    viewport: Viewport<f64>,
    canvas_width: u32,
    canvas_height: u32,
    tile_size: u32,
    max_iterations: u32,
}
```

---

## Communication Protocol

### Main Thread → Worker (postMessage)

```rust
#[derive(Serialize, Deserialize)]
pub enum WorkerRequest {
    Render {
        viewport_json: String,      // Serialized Viewport<f64>
        canvas_width: u32,
        canvas_height: u32,
        tile_size: u32,
        render_id: u32,
    },
    Terminate,
}
```

### Worker → Main Thread (postMessage)

```rust
#[derive(Serialize, Deserialize)]
pub enum WorkerResponse {
    Ready,                          // Worker initialized
    TileComplete { tile_index: u32 },
    RenderComplete,
    Error { message: String },
}
```

### Shared Memory (no messages needed)

Workers **do not send pixel data via postMessage**. They write directly to SharedArrayBuffer. Main thread polls buffer for completed tiles.

---

## Implementation Details

### 1. Build Configuration

**Trunk Configuration (index.html):**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Fractal Wonder</title>
  <link data-trunk rel="tailwind-css" href="input.css" />

  <!-- Main UI WASM (--target no-modules) -->
  <link data-trunk rel="rust"
        data-type="main"
        data-bindgen-target="no-modules"
        data-wasm-opt="z"
        href="./fractalwonder-ui/Cargo.toml" />

  <!-- Worker WASM (--target no-modules) -->
  <link data-trunk rel="rust"
        data-type="worker"
        data-bindgen-target="no-modules"
        data-wasm-opt="z"
        href="./fractalwonder-compute/Cargo.toml" />
</head>
<body class="m-0 p-0 overflow-hidden">
  <div id="app"></div>
</body>
</html>
```

**Key attribute: `data-bindgen-target="no-modules"`**

This uses `wasm-bindgen --target no-modules`, which:
- Works in all modern browsers
- Compatible with Web Workers
- No ES6 module issues

**Trunk.toml (already correct):**

```toml
[serve.headers]
Cross-Origin-Opener-Policy = "same-origin"
Cross-Origin-Embedder-Policy = "require-corp"
```

These headers enable SharedArrayBuffer.

**NO `.cargo/config.toml` needed** (no atomics flags required for manual workers)

**NO `rust-toolchain.toml` needed** (stable Rust works)

### 2. Workspace Dependencies

**Root Cargo.toml additions:**

```toml
[workspace.dependencies]
# ... existing dependencies ...

# For worker communication
gloo-utils = "0.2"           # For typed arrays
js-sys.workspace = true
web-sys = { version = "0.3", features = [
    "Worker",
    "WorkerOptions",
    "WorkerType",
    "MessageEvent",
] }
```

**fractalwonder-ui/Cargo.toml:**

```toml
[dependencies]
# ... existing dependencies ...
web-sys = { workspace = true, features = [
    "Worker",
    "WorkerOptions",
    "WorkerType",
    "MessageEvent",
] }
js-sys.workspace = true
gloo-utils.workspace = true
```

**fractalwonder-compute/Cargo.toml:**

```toml
[dependencies]
# ... existing dependencies ...
js-sys.workspace = true
gloo-utils.workspace = true

# NOTE: NO web-sys with DOM features
# Workers cannot access DOM
```

### 3. Worker Creation (Main Thread)

```rust
// fractalwonder-ui/src/workers/worker_pool.rs

use web_sys::{Worker, WorkerOptions, WorkerType};
use wasm_bindgen::prelude::*;
use js_sys::ArrayBuffer;

pub struct WorkerPool {
    workers: Vec<Worker>,
    shared_buffer: Option<ArrayBuffer>,
    current_render_id: Arc<AtomicU32>,
}

impl WorkerPool {
    pub fn new() -> Result<Self, JsValue> {
        let worker_count = web_sys::window()
            .and_then(|w| w.navigator().hardware_concurrency())
            .map(|c| c as usize)
            .unwrap_or(4);

        let mut workers = Vec::new();

        for i in 0..worker_count {
            // Create worker with module type
            let mut options = WorkerOptions::new();
            options.type_(WorkerType::Module);

            // Worker script path (Trunk generates this)
            let worker = Worker::new_with_options(
                "./fractalwonder_compute.js",
                &options
            )?;

            // Set up message handler
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                // Handle worker responses
                Self::handle_worker_message(e);
            }) as Box<dyn FnMut(_)>);

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            workers.push(worker);
        }

        Ok(Self {
            workers,
            shared_buffer: None,
            current_render_id: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn start_render(
        &mut self,
        viewport: &Viewport<f64>,
        canvas_width: u32,
        canvas_height: u32,
        tile_size: u32,
    ) -> Result<u32, JsValue> {
        let render_id = self.current_render_id.fetch_add(1, Ordering::SeqCst) + 1;

        // Create SharedArrayBuffer
        let layout = SharedBufferLayout::new(canvas_width, canvas_height);
        let buffer_size = layout.buffer_size();

        let shared_buffer = ArrayBuffer::new(buffer_size as u32);
        self.shared_buffer = Some(shared_buffer.clone());

        // Initialize buffer (zero out)
        let view = js_sys::Uint8Array::new(&shared_buffer);
        for i in 0..buffer_size {
            view.set_index(i as u32, 0);
        }

        // Serialize viewport
        let viewport_json = serde_json::to_string(viewport)
            .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;

        // Create request
        let request = WorkerRequest::Render {
            viewport_json,
            canvas_width,
            canvas_height,
            tile_size,
            render_id,
        };

        let message = serde_json::to_string(&request)
            .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;

        // Send to all workers
        for worker in &self.workers {
            // Create transfer object (share memory, don't copy)
            let transfer = js_sys::Array::new();
            transfer.push(&shared_buffer);

            worker.post_message_with_transfer(
                &JsValue::from_str(&message),
                &transfer
            )?;
        }

        Ok(render_id)
    }
}
```

### 4. Worker Implementation (Worker Thread)

```rust
// fractalwonder-compute/src/worker.rs

use wasm_bindgen::prelude::*;
use js_sys::{ArrayBuffer, Uint8Array};
use std::sync::atomic::{AtomicU32, Ordering};

#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();

    // Send ready message
    let response = WorkerResponse::Ready;
    let message = serde_json::to_string(&response).unwrap();

    web_sys::self_()
        .post_message(&JsValue::from_str(&message))
        .ok();
}

#[wasm_bindgen]
pub fn process_render_request(
    message_json: String,
    shared_buffer: ArrayBuffer,
) -> Result<(), JsValue> {
    let request: WorkerRequest = serde_json::from_str(&message_json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    match request {
        WorkerRequest::Render {
            viewport_json,
            canvas_width,
            canvas_height,
            tile_size,
            render_id,
        } => {
            let viewport: Viewport<f64> = serde_json::from_str(&viewport_json)
                .map_err(|e| JsValue::from_str(&format!("Parse viewport: {}", e)))?;

            compute_tiles(
                viewport,
                canvas_width,
                canvas_height,
                tile_size,
                render_id,
                shared_buffer,
            )
        }
        WorkerRequest::Terminate => {
            web_sys::self_().close();
            Ok(())
        }
    }
}

fn compute_tiles(
    viewport: Viewport<f64>,
    width: u32,
    height: u32,
    tile_size: u32,
    render_id: u32,
    shared_buffer: ArrayBuffer,
) -> Result<(), JsValue> {
    let layout = SharedBufferLayout::new(width, height);
    let view = Uint8Array::new(&shared_buffer);

    // Generate all tiles
    let tiles = generate_tiles(width, height, tile_size);
    let total_tiles = tiles.len() as u32;

    // Create renderer
    let computer = MandelbrotComputer::<f64>::default();
    let renderer = PixelRenderer::new(computer);

    // Work-stealing loop
    loop {
        // Atomically get next tile index
        let tile_index = atomic_fetch_add(&view, 0, 1);

        if tile_index >= total_tiles {
            break; // All work done
        }

        // Check if render was cancelled
        let current_render_id = atomic_load(&view, 4);
        if current_render_id != render_id {
            break; // Cancelled
        }

        let tile = &tiles[tile_index as usize];

        // Render tile
        let tile_data = renderer.render(
            &viewport,
            *tile,
            (width, height)
        );

        // Write to shared buffer
        write_tile_to_buffer(&view, &layout, tile, &tile_data);

        // Notify main thread
        let response = WorkerResponse::TileComplete {
            tile_index: tile_index,
        };
        if let Ok(msg) = serde_json::to_string(&response) {
            web_sys::self_()
                .post_message(&JsValue::from_str(&msg))
                .ok();
        }
    }

    Ok(())
}

// Atomic operations on Uint8Array
fn atomic_fetch_add(view: &Uint8Array, offset: u32, value: u32) -> u32 {
    // Read current value
    let bytes = [
        view.get_index(offset),
        view.get_index(offset + 1),
        view.get_index(offset + 2),
        view.get_index(offset + 3),
    ];
    let current = u32::from_le_bytes(bytes);

    // Write incremented value
    let new_value = current + value;
    let new_bytes = new_value.to_le_bytes();
    view.set_index(offset, new_bytes[0]);
    view.set_index(offset + 1, new_bytes[1]);
    view.set_index(offset + 2, new_bytes[2]);
    view.set_index(offset + 3, new_bytes[3]);

    current
}
```

**CRITICAL NOTE:** The above `atomic_fetch_add` is **NOT truly atomic**. This is a simplification. For true atomics, we need `Atomics.add()` from JavaScript. See next section.

### 5. True Atomic Operations

We need to call JavaScript's `Atomics.add()` from Rust:

```rust
// fractalwonder-compute/src/atomics.rs

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Atomics, js_name = add)]
    pub fn atomics_add(
        typedArray: &js_sys::Int32Array,
        index: u32,
        value: i32,
    ) -> i32;

    #[wasm_bindgen(js_namespace = Atomics, js_name = load)]
    pub fn atomics_load(
        typedArray: &js_sys::Int32Array,
        index: u32,
    ) -> i32;
}

pub fn atomic_fetch_add_u32(
    buffer: &js_sys::ArrayBuffer,
    byte_offset: u32,
    value: u32,
) -> u32 {
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_add(&int32_array, index, value as i32) as u32
}
```

### 6. Main Thread: Progressive Rendering

```rust
// fractalwonder-ui/src/rendering/worker_canvas_renderer.rs

impl WorkerCanvasRenderer {
    fn start_progressive_poll(&self, canvas: HtmlCanvasElement) {
        let self_clone = self.clone();

        let closure = Closure::wrap(Box::new(move || {
            self_clone.poll_and_render(&canvas);
        }) as Box<dyn FnMut()>);

        web_sys::window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .ok();

        closure.forget();
    }

    fn poll_and_render(&self, canvas: &HtmlCanvasElement) {
        if let Some(buffer) = &self.worker_pool.shared_buffer {
            let layout = SharedBufferLayout::new(
                canvas.width(),
                canvas.height(),
            );

            let view = js_sys::Uint8Array::new(buffer);

            // Read all pixel data
            let mut pixel_data = Vec::new();
            for pixel_idx in 0..layout.total_pixels {
                let offset = layout.pixel_offset(pixel_idx);
                let bytes = [
                    view.get_index((offset + 0) as u32),
                    view.get_index((offset + 1) as u32),
                    view.get_index((offset + 2) as u32),
                    view.get_index((offset + 3) as u32),
                    view.get_index((offset + 4) as u32),
                    view.get_index((offset + 5) as u32),
                    view.get_index((offset + 6) as u32),
                    view.get_index((offset + 7) as u32),
                ];

                let data = SharedBufferLayout::decode_pixel(&bytes);
                pixel_data.push(data);
            }

            // Colorize and display
            self.colorize_and_display(&pixel_data, canvas);
        }

        // Continue polling
        self.start_progressive_poll(canvas.clone());
    }
}
```

---

## Testing Strategy

### Phase 1: Minimal Proof of Concept

1. **Single worker, single tile**
   - Verify worker creation
   - Verify SharedArrayBuffer creation
   - Verify postMessage communication
   - Verify tile computation
   - Verify result retrieval

2. **Build verification**
   - Run `trunk build`
   - Verify both WASMs generated
   - Verify worker script exists
   - Verify no-modules target used

### Phase 2: Multi-Worker Test

1. **4 workers, 16 tiles**
   - Verify atomic counter works
   - Verify no tile computed twice
   - Verify all tiles completed
   - Verify performance improvement

### Phase 3: Full Integration

1. **Full canvas render**
   - Large canvas (1920x1080)
   - Many tiles (hundreds)
   - Progressive display
   - Cancellation testing

---

## Success Criteria

- ✅ Multiple workers spawn successfully
- ✅ All tiles computed exactly once
- ✅ Results appear in SharedArrayBuffer
- ✅ Progressive display visible
- ✅ Measurable speedup (>2x on 4+ cores)
- ✅ UI stays responsive during render
- ✅ Trunk build succeeds
- ✅ No console errors

---

## Risk Mitigation

### Known Risks

1. **Trunk worker path:** Worker script path might be wrong
   - **Mitigation:** Test worker creation immediately
   - **Fallback:** Manually inspect dist/ for correct path

2. **SharedArrayBuffer not truly atomic without Atomics API**
   - **Mitigation:** Use JavaScript Atomics via wasm_bindgen extern
   - **Fallback:** Mutex-based coordination if needed

3. **no-modules target compatibility**
   - **Mitigation:** Reference working examples
   - **Fallback:** Try different wasm-bindgen targets

### Debugging Strategy

1. **Console logging at every step**
2. **Test in isolation** (worker creation, buffer creation, etc.)
3. **Compare to working examples**
4. **Browser DevTools** (check Network tab for worker loading)

---

## Next Steps

1. **Create minimal test case** (separate from main app)
2. **Verify Trunk builds worker correctly**
3. **Test worker communication**
4. **Test SharedArrayBuffer**
5. **Test atomic operations**
6. **Integrate into AsyncProgressiveCanvasRenderer**

---

## References

- [sgasse/wasm_worker_interaction](https://github.com/sgasse/wasm_worker_interaction)
- [ScottLogic: Faster Fractals with Multi-Threaded WebAssembly](https://blog.scottlogic.com/2019/07/15/multithreaded-webassembly.html)
- [larsch/webfractals](https://github.com/larsch/webfractals)
- [MDN: Web Workers API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API)
- [MDN: SharedArrayBuffer](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer)
- [MDN: Atomics](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics)
