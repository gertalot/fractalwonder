# Iteration 3: Manual Web Workers Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add multi-core parallelism using manual Web Workers with atomic work distribution for faster fractal rendering

**Architecture:** Main thread spawns N workers (CPU cores), each loading fractalwonder-compute WASM. Workers compete for tiles via atomic counter in SharedArrayBuffer, compute independently, write results to shared memory. Main thread polls buffer and progressively displays colorized results.

**Tech Stack:** Rust stable, wasm-bindgen (no-modules target), Trunk, web_sys::Worker, js_sys::ArrayBuffer, Atomics API

---

## Prerequisites

**Verify Iterations 1 & 2 Complete:**
- Workspace structure exists (fractalwonder-ui, fractalwonder-compute, fractalwonder-core)
- AsyncProgressiveCanvasRenderer implemented and working
- `trunk serve` runs successfully
- All tests pass

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

---

## Task 1: Add Workspace Dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `fractalwonder-ui/Cargo.toml`
- Modify: `fractalwonder-compute/Cargo.toml`

**Step 1: Add web-sys Worker features to workspace**

In `Cargo.toml` at workspace root, add to `[workspace.dependencies]`:

```toml
web-sys = { version = "0.3", features = [
    "Window",
    "Navigator",
    "HtmlCanvasElement",
    "CanvasRenderingContext2d",
    "ImageData",
    "Worker",
    "WorkerOptions",
    "WorkerType",
    "MessageEvent",
    "DedicatedWorkerGlobalScope",
] }
gloo-utils = "0.2"
```

**Step 2: Add dependencies to fractalwonder-ui**

In `fractalwonder-ui/Cargo.toml`, add to `[dependencies]`:

```toml
web-sys.workspace = true
js-sys.workspace = true
gloo-utils.workspace = true
```

**Step 3: Add dependencies to fractalwonder-compute**

In `fractalwonder-compute/Cargo.toml`, add to `[dependencies]`:

```toml
js-sys.workspace = true
gloo-utils.workspace = true

# Note: NO web-sys with DOM features
# Workers cannot access DOM
```

**Step 4: Verify dependencies**

Run: `cargo check --workspace`
Expected: Compiles successfully, downloads new dependencies

**Step 5: Commit dependency changes**

```bash
git add Cargo.toml Cargo.lock fractalwonder-ui/Cargo.toml fractalwonder-compute/Cargo.toml
git commit -m "feat: add web worker dependencies for multi-core support"
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
    /// Render a viewport
    Render {
        viewport_json: String,  // Serialized Viewport<f64>
        canvas_width: u32,
        canvas_height: u32,
        render_id: u32,
        tile_size: u32,
    },

    /// Terminate worker
    Terminate,
}

/// Message from worker to main thread
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Worker initialized and ready
    Ready,

    /// Single tile completed
    TileComplete {
        tile_index: u32,
    },

    /// All tiles completed
    RenderComplete,

    /// Error occurred
    Error {
        message: String,
    },
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

/// Layout of SharedArrayBuffer for worker-main communication
///
/// Memory layout:
/// - Bytes 0-3: Tile index counter (AtomicU32)
/// - Bytes 4-7: Render ID (AtomicU32) - for cancellation
/// - Bytes 8+: Tile data (8 bytes per pixel: 4 bytes iterations + 4 bytes escaped flag)
pub struct SharedBufferLayout {
    /// Offset in bytes for tile counter
    tile_index_offset: usize,
    /// Offset in bytes for render ID
    render_id_offset: usize,
    /// Offset in bytes where pixel data starts
    data_offset: usize,
    /// Total pixels in canvas
    pub total_pixels: usize,
}

impl SharedBufferLayout {
    const TILE_INDEX_OFFSET: usize = 0;
    const RENDER_ID_OFFSET: usize = 4;
    const DATA_OFFSET: usize = 8;
    const BYTES_PER_PIXEL: usize = 8; // u32 iterations + u32 escaped flag

    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            tile_index_offset: Self::TILE_INDEX_OFFSET,
            render_id_offset: Self::RENDER_ID_OFFSET,
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

    /// Get tile index counter offset
    pub fn tile_index_offset(&self) -> usize {
        self.tile_index_offset
    }

    /// Get render ID offset
    pub fn render_id_offset(&self) -> usize {
        self.render_id_offset
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
Expected: All tests pass (3 new tests)

**Step 4: Commit shared buffer layout**

```bash
git add fractalwonder-compute/src/shared_buffer.rs \
        fractalwonder-compute/src/lib.rs
git commit -m "feat: define SharedArrayBuffer layout for worker communication"
```

---

## Task 4: Update Trunk Configuration for Worker Build

**Files:**
- Modify: `index.html`

**Step 1: Update index.html to build worker WASM**

In `index.html`, replace the existing rust link with TWO links:

```html
<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Fractal Wonder</title>
  <link data-trunk rel="tailwind-css" href="input.css" />

  <!-- Main UI WASM (no-modules target) -->
  <link data-trunk rel="rust"
        data-type="main"
        data-bindgen-target="no-modules"
        data-wasm-opt="z"
        href="./fractalwonder-ui/Cargo.toml" />

  <!-- Worker WASM (no-modules target) -->
  <link data-trunk rel="rust"
        data-type="worker"
        data-bindgen-target="no-modules"
        data-wasm-opt="z"
        href="./fractalwonder-compute/Cargo.toml" />
</head>

<body class="m-0 p-0 overflow-hidden">
  <div id="app"></div>
  <script type="module">
    window.addEventListener('TrunkApplicationStarted', () => {
      window.wasmBindings.hydrate();
    });
  </script>
</body>

</html>
```

**Step 2: Verify Trunk.toml has CORS headers**

Verify `Trunk.toml` contains (should already exist):

```toml
[serve.headers]
Cross-Origin-Opener-Policy = "same-origin"
Cross-Origin-Embedder-Policy = "require-corp"
```

**Step 3: Test trunk build**

Run: `trunk build`
Expected: Builds both UI and worker WASM successfully

Check `dist/` directory for:
- `fractalwonder_ui*.js`
- `fractalwonder_ui*.wasm`
- `fractalwonder_compute*.js`
- `fractalwonder_compute*.wasm`

**Step 4: Commit trunk configuration**

```bash
git add index.html
git commit -m "feat: configure trunk to build worker WASM with no-modules target"
```

---

## Task 5: Add Serde Support to Core Types

**Files:**
- Modify: `fractalwonder-core/Cargo.toml`
- Modify: `fractalwonder-core/src/viewport.rs`
- Modify: `fractalwonder-core/src/points.rs`

**Step 1: Add serde to core dependencies**

In `fractalwonder-core/Cargo.toml`, add:

```toml
[dependencies]
serde.workspace = true
```

**Step 2: Add serde derives to Viewport**

In `fractalwonder-core/src/viewport.rs`, add import and derive:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewport<S> {
    pub visible_bounds: Rect<S>,
}
```

**Step 3: Add serde derives to Rect and Point**

In `fractalwonder-core/src/points.rs`, add import and derives:

```rust
use serde::{Deserialize, Serialize};

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

**Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: Compiles successfully

**Step 5: Commit serialization support**

```bash
git add fractalwonder-core/Cargo.toml \
        fractalwonder-core/src/viewport.rs \
        fractalwonder-core/src/points.rs
git commit -m "feat: add serde support to core types for worker serialization"
```

---

## Task 6: Create Atomic Operations Module

**Files:**
- Create: `fractalwonder-compute/src/atomics.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create atomics.rs with JavaScript bindings**

Create `fractalwonder-compute/src/atomics.rs`:

```rust
use wasm_bindgen::prelude::*;

/// Bindings to JavaScript Atomics API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Atomics, js_name = add)]
    fn atomics_add(
        typedArray: &js_sys::Int32Array,
        index: u32,
        value: i32,
    ) -> i32;

    #[wasm_bindgen(js_namespace = Atomics, js_name = load)]
    fn atomics_load(
        typedArray: &js_sys::Int32Array,
        index: u32,
    ) -> i32;

    #[wasm_bindgen(js_namespace = Atomics, js_name = store)]
    fn atomics_store(
        typedArray: &js_sys::Int32Array,
        index: u32,
        value: i32,
    ) -> i32;
}

/// Atomically fetch and add to u32 value in buffer
pub fn atomic_fetch_add_u32(
    buffer: &js_sys::ArrayBuffer,
    byte_offset: u32,
    value: u32,
) -> u32 {
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_add(&int32_array, index, value as i32) as u32
}

/// Atomically load u32 value from buffer
pub fn atomic_load_u32(
    buffer: &js_sys::ArrayBuffer,
    byte_offset: u32,
) -> u32 {
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_load(&int32_array, index) as u32
}

/// Atomically store u32 value to buffer
pub fn atomic_store_u32(
    buffer: &js_sys::ArrayBuffer,
    byte_offset: u32,
    value: u32,
) -> u32 {
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_store(&int32_array, index, value as i32) as u32
}
```

**Step 2: Add to lib.rs exports**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
#[cfg(target_arch = "wasm32")]
pub mod atomics;
#[cfg(target_arch = "wasm32")]
pub use atomics::{atomic_fetch_add_u32, atomic_load_u32, atomic_store_u32};
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-compute --target wasm32-unknown-unknown`
Expected: Compiles successfully

**Step 4: Commit atomic operations**

```bash
git add fractalwonder-compute/src/atomics.rs \
        fractalwonder-compute/src/lib.rs
git commit -m "feat: add JavaScript Atomics API bindings for worker synchronization"
```

---

## Task 7: Create Worker Entry Point

**Files:**
- Create: `fractalwonder-compute/src/worker.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create worker.rs with entry point**

Create `fractalwonder-compute/src/worker.rs`:

```rust
use crate::{
    atomics::{atomic_fetch_add_u32, atomic_load_u32},
    MandelbrotComputer, PixelRenderer, Renderer, SharedBufferLayout,
    WorkerRequest, WorkerResponse,
};
use fractalwonder_core::{MandelbrotData, PixelRect, Viewport};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

/// Worker initialization - called when worker starts
#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();

    // Send ready message
    let response = WorkerResponse::Ready;
    let message = serde_json::to_string(&response).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
            .dyn_into()
            .expect("Failed to get worker global scope");

        global
            .post_message(&JsValue::from_str(&message))
            .ok();
    }
}

/// Process render request from main thread
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
            render_id,
            tile_size,
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
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                    .dyn_into()
                    .expect("Failed to get worker global scope");
                global.close();
            }
            Ok(())
        }
    }
}

/// Compute all tiles using work-stealing pattern
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

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Worker: Starting render {} with {} tiles",
        render_id, total_tiles
    )));

    // Work-stealing loop
    loop {
        // Atomically get next tile index
        let tile_index = atomic_fetch_add_u32(
            &shared_buffer,
            layout.tile_index_offset() as u32,
            1
        );

        if tile_index >= total_tiles {
            break; // All work done
        }

        // Check if render was cancelled
        let current_render_id = atomic_load_u32(
            &shared_buffer,
            layout.render_id_offset() as u32
        );
        if current_render_id != render_id {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Worker: Render {} cancelled",
                render_id
            )));
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

        // Notify main thread (optional - main polls buffer)
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            let response = WorkerResponse::TileComplete {
                tile_index,
            };
            if let Ok(msg) = serde_json::to_string(&response) {
                let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                    .dyn_into()
                    .expect("Failed to get worker global scope");

                global
                    .post_message(&JsValue::from_str(&msg))
                    .ok();
            }
        }
    }

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

    // Sort by distance from center (closest first)
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

/// Write tile data to shared buffer
fn write_tile_to_buffer(
    view: &Uint8Array,
    layout: &SharedBufferLayout,
    tile: &PixelRect,
    tile_data: &[MandelbrotData],
) {
    let width = tile.x + tile.width; // Canvas width (approximation)

    for local_y in 0..tile.height {
        let canvas_y = tile.y + local_y;
        for local_x in 0..tile.width {
            let canvas_x = tile.x + local_x;
            let pixel_index = (canvas_y * width + canvas_x) as usize;
            let tile_data_index = (local_y * tile.width + local_x) as usize;

            // Encode pixel data
            let pixel = &tile_data[tile_data_index];
            let encoded = SharedBufferLayout::encode_pixel(pixel);

            // Write to buffer
            let offset = layout.pixel_offset(pixel_index);
            for (i, byte) in encoded.iter().enumerate() {
                view.set_index((offset + i) as u32, *byte);
            }
        }
    }
}
```

**Step 2: Add worker module to lib.rs**

In `fractalwonder-compute/src/lib.rs`, add:

```rust
#[cfg(target_arch = "wasm32")]
pub mod worker;
```

**Step 3: Add web-sys DedicatedWorkerGlobalScope feature**

In `fractalwonder-compute/Cargo.toml`, verify web-sys has:

```toml
[dependencies]
web-sys = { workspace = true, features = [
    "console",
    "DedicatedWorkerGlobalScope",
] }
```

If not, add it to workspace in root `Cargo.toml`:

```toml
[workspace.dependencies]
web-sys = { version = "0.3", features = [
    # ... existing features ...
    "console",
    "DedicatedWorkerGlobalScope",
] }
```

**Step 4: Verify it compiles**

Run: `cargo check -p fractalwonder-compute --target wasm32-unknown-unknown`
Expected: Compiles successfully

**Step 5: Commit worker entry point**

```bash
git add fractalwonder-compute/src/worker.rs \
        fractalwonder-compute/src/lib.rs \
        Cargo.toml \
        fractalwonder-compute/Cargo.toml
git commit -m "feat: implement worker entry point with work-stealing parallelism"
```

---

## Task 8: Build and Verify Worker WASM

**Files:**
- None (verification step)

**Step 1: Clean build**

Run: `trunk clean && trunk build`
Expected: Builds successfully

**Step 2: Verify worker files exist**

Check `dist/` directory for:

```
ls -lh dist/ | grep fractalwonder_compute
```

Expected output should show:
- `fractalwonder_compute_bg.wasm` (worker WASM)
- `fractalwonder_compute.js` (worker JS glue)

**Step 3: Check worker file size**

Run: `ls -lh dist/fractalwonder_compute_bg.wasm`
Expected: File exists, reasonable size (~100KB-500KB)

**Step 4: Verify no-modules target**

Open `dist/fractalwonder_compute.js` and verify it does NOT contain ES6 imports (should use global `wasm_bindgen`)

Expected: File starts with something like:
```javascript
let wasm;
```

Not:
```javascript
import * as wasm from './fractalwonder_compute_bg.wasm';
```

**Step 5: Document findings**

Create: `docs/testing/iteration-3-build-verification.md`

```markdown
# Iteration 3: Build Verification

**Date:** [Today's date]

## Verification Steps

- [x] Worker WASM builds successfully
- [x] Worker JS glue generated
- [x] no-modules target confirmed
- [x] File sizes reasonable

## Files Generated

- `dist/fractalwonder_compute_bg.wasm` - [SIZE]
- `dist/fractalwonder_compute.js` - [SIZE]

## Notes

[Any observations]
```

**Step 6: Commit verification docs**

```bash
git add docs/testing/iteration-3-build-verification.md
git commit -m "docs: verify worker WASM builds correctly"
```

---

## Task 9: Manual Browser Test - Worker Loading

**Files:**
- Create: `docs/testing/iteration-3-manual-worker-test.md`

**Step 1: Start dev server**

Run: `trunk serve`

Expected: Server starts on http://localhost:8080

**Step 2: Open browser with DevTools**

1. Open http://localhost:8080
2. Open DevTools (F12)
3. Go to Console tab
4. Go to Network tab

**Step 3: Verify CORS headers**

In Network tab:
1. Click on the HTML document request
2. Go to Headers
3. Verify:
   - `Cross-Origin-Opener-Policy: same-origin`
   - `Cross-Origin-Embedder-Policy: require-corp`

**Step 4: Verify worker files loaded**

In Network tab, verify requests for:
- `fractalwonder_ui_bg.wasm` (main)
- `fractalwonder_compute_bg.wasm` (worker)
- `fractalwonder_compute.js` (worker glue)

**Step 5: Check console for errors**

Expected: No red errors about:
- CORS
- WASM instantiation
- Worker creation

**Step 6: Document test results**

Create `docs/testing/iteration-3-manual-worker-test.md`:

```markdown
# Iteration 3: Manual Worker Load Test

**Date:** [Today's date]

## Test Results

### CORS Headers
- [x] Cross-Origin-Opener-Policy present
- [x] Cross-Origin-Embedder-Policy present

### Files Loaded
- [x] fractalwonder_ui_bg.wasm
- [x] fractalwonder_compute_bg.wasm
- [x] fractalwonder_compute.js

### Console Errors
- [x] No CORS errors
- [x] No WASM errors
- [x] No worker errors

## Notes

[Any observations]
```

**Step 7: Commit test results**

```bash
git add docs/testing/iteration-3-manual-worker-test.md
git commit -m "test: manual verification of worker loading in browser"
```

---

## Task 10: Run Validation Suite

**Files:**
- None (verification only)

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings (may need fixes)

**Step 3: Fix any Clippy warnings**

If warnings appear, fix them and commit:

```bash
git add .
git commit -m "fix: address clippy warnings"
```

**Step 4: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 5: Test WASM build**

Run: `wasm-pack build --target web ./fractalwonder-ui`
Expected: Builds successfully

Run: `wasm-pack build --target web ./fractalwonder-compute`
Expected: Builds successfully

---

## Task 11: Create Minimal Worker Integration Test (Future)

**Note:** This task creates a placeholder for actual worker integration. Full implementation happens in next tasks.

**Files:**
- Create: `fractalwonder-compute/tests/worker_integration.rs`

**Step 1: Create worker integration test skeleton**

Create `fractalwonder-compute/tests/worker_integration.rs`:

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

**Step 3: Commit integration test skeleton**

```bash
git add fractalwonder-compute/tests/worker_integration.rs
git commit -m "test: add integration tests for SharedArrayBuffer"
```

---

## Success Criteria - Phase 1 Complete

At this point, you should have:

- ✅ Worker WASM builds successfully via Trunk
- ✅ Worker files appear in dist/
- ✅ CORS headers configured for SharedArrayBuffer
- ✅ Worker message protocol defined
- ✅ SharedArrayBuffer layout implemented
- ✅ Atomic operations available
- ✅ Worker entry point exists
- ✅ All tests pass
- ✅ No Clippy warnings

**What's NOT done yet:**
- Creating workers from main thread
- Sending messages to workers
- Reading results from SharedArrayBuffer
- Integrating with AsyncProgressiveCanvasRenderer

These will be in separate tasks to keep each task small and testable.

---

## Next Steps

After completing Phase 1, continue with:

1. **Task 12-15:** Worker Pool creation and management (main thread side)
2. **Task 16-18:** Integration with AsyncProgressiveCanvasRenderer
3. **Task 19-20:** Manual browser testing with actual rendering
4. **Task 21:** Final validation and performance testing

Each of these will be separate implementation plan documents.
