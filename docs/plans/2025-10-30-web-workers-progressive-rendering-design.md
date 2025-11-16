# Web Workers for Progressive Rendering - Design Document

**Date:** 2025-10-30
**Last Updated:** 2025-11-16
**Status:** Refined (Added loading state integration)
**Goal:** Enable true progressive rendering using Web Workers for parallel tile computation while maintaining UI responsiveness and preserving existing architecture abstractions.

**Changelog:**
- 2025-11-16: Added callback-based worker readiness protocol with UI loading states to avoid main thread blocking
- 2025-11-16: Added SharedArrayBuffer architecture for perturbation theory support (hybrid approach with Transferables for tile results)

## Problem Statement

The current `TilingCanvasRenderer` computes all tiles synchronously in a single JavaScript/WASM call. While it has tiling infrastructure, the browser cannot repaint between tiles because WASM doesn't yield control back to the event loop. This results in:

- No progressive rendering visible to user (entire image appears at once)
- UI freezing during computation (especially at high zoom with BigFloat)
- CPU underutilization (only one core used despite multi-core availability)
- Cannot interrupt renders cleanly during computation

## Requirements

### Functional Requirements

1. **Progressive rendering:** User sees tiles appear one-by-one as they complete
2. **UI responsiveness:** Main thread remains responsive during computation (< 16ms per frame)
3. **Maximum CPU utilization:** Use all available CPU cores (`navigator.hardwareConcurrency`)
4. **Immediate cancellation:** User can zoom/pan mid-render without lag
5. **Preserve abstractions:** Maintain `CanvasRenderer` trait, pluggable renderers, instant recolorization
6. **WASM in workers:** Workers can load WASM modules that contain pure computation and do NOT use "hydrate" or need
   any DOM dependencies (which won't work in workers)
7. **Simple is better:** prefer simple and straightforward architecture and build process for worker script. If it
   feels complicated, STOP because we need to reconsider the plan.

### Non-Functional Requirements

- Memory overhead acceptable (< 100 MB for worker pool)
- Minimal code duplication between main thread and workers
- Type-safe across worker boundary
- Robust error handling for worker failures

## Design Overview

### Architecture Choice: Dedicated Worker Renderer

**Selected approach:** Create `WorkerPoolCanvasRenderer` that manages a pool of Web Workers. Each worker runs dedicated computation code (only `MandelbrotComputer` or `TestImageComputer` level). Main thread handles UI concerns: queue management, caching, colorization, and canvas rendering.

**Key architectural principle:** Clean separation of concerns

- **Workers:** Pure computation (viewport → pixel data)
- **Main thread:** UI/coordination (colorization, canvas, cache, queue)

## Detailed Design

### 1. Worker Architecture & Lifecycle

#### Worker Pool Structure

```rust
pub struct WorkerPoolCanvasRenderer<S, D: Clone> {
    workers: Vec<WorkerHandle>,
    tile_queue: Arc<Mutex<VecDeque<PixelRect>>>,
    cache: Arc<Mutex<CachedState<S, D>>>,  // Same as TilingCanvasRenderer
    colorizer: Colorizer<D>,
    renderer_type: RendererType,
    current_render_id: Arc<AtomicU32>,
    in_flight: Arc<Mutex<HashMap<WorkerId, (PixelRect, u32)>>>,
    workers_ready: Arc<AtomicU32>,  // Track how many workers are ready
    on_ready_callback: Option<Box<dyn Fn()>>,  // Callback when all workers ready
    pending_render: Option<(Viewport<S>, HtmlCanvasElement)>,  // Queued render request

    // Perturbation theory support (SharedArrayBuffer)
    shared_reference_orbit: Option<SharedArrayBuffer>,  // Shared across all workers
    reference_orbit_ready: Arc<AtomicBool>,  // Signals when reference orbit is ready
}
```

#### Worker Count

- Spawn `navigator.hardwareConcurrency` workers (one per CPU core)
- Maximizes parallel computation
- Each worker is independent, loads full WASM module as ES6 module

#### Worker Module Type: ES6 Modules

**IMPORTANT:** Workers must be created as ES6 module workers, not classic workers:

- Use `new Worker(url, { type: 'module' })` in JavaScript
- Enables `import` statements in worker code
- Allows dynamic WASM imports without `importScripts`
- Required for modern WASM loading patterns

#### Worker Lifecycle

1. **Initialization:**

   - Main thread spawns N workers as ES6 modules
   - Each worker dynamically imports WASM module
   - Worker receives `Init` message with `RendererType` (Mandelbrot or TestImage)
   - Worker constructs appropriate renderer instance
   - Worker sends `Ready` response back to main thread
   - Main thread increments `workers_ready` counter
   - **When all workers ready:** Main thread invokes `on_ready_callback` (signals UI to hide loading indicator)
   - **If render requested before ready:** Request queued in `pending_render`, executed when workers ready

2. **Computation Loop:**

   - Worker waits for `ComputeTile` message
   - Computes tile using renderer
   - Serializes data to bytes
   - Sends `TileComplete` back to main thread
   - Repeats until cancelled or no more tiles

3. **Termination:**
   - On renderer swap: main thread terminates all workers
   - Workers cleaned up by browser

#### Message Protocol

```rust
// Main → Worker
enum WorkerMessage {
    Init {
        renderer_type: RendererType,
        config: RenderConfig,
    },
    ComputeTile {
        render_id: u32,
        viewport: Viewport<S>,
        rect: PixelRect,
        canvas_size: (u32, u32),
    },
    Cancel {
        render_id: u32,
    },
}

// Worker → Main
enum WorkerResponse {
    Ready,
    TileComplete {
        render_id: u32,
        rect: PixelRect,
        data: Vec<u8>,  // Serialized D (MandelbrotData or TestImageData)
    },
    Error {
        message: String,
    },
}
```

### 2. Work Queue & Load Balancing

#### Queue Strategy: Pull-Based Work Distribution with Readiness Gate

**Algorithm:**

1. Main thread generates all tiles at render start: `compute_tiles_circular(width, height, tile_size, center)`
2. Tiles added to shared queue in circular order: `VecDeque<PixelRect>`
3. **Wait for all workers to signal Ready** before dispatching any tiles
4. Initial dispatch: Once all workers ready, send one tile to each worker
5. On `TileComplete` receipt:
   - Process tile (cache, colorize, display)
   - If queue not empty: send next tile to this worker
   - Else: worker becomes idle

**Worker Readiness Protocol:**

```rust
impl WorkerPoolCanvasRenderer {
    pub fn on_ready(&mut self, callback: impl Fn() + 'static) {
        self.on_ready_callback = Some(Box::new(callback));
    }

    fn handle_worker_ready(&self) {
        let ready_count = self.workers_ready.fetch_add(1, Ordering::SeqCst) + 1;
        if ready_count == self.workers.len() as u32 {
            // All workers ready!
            if let Some(callback) = &self.on_ready_callback {
                callback();  // Signal UI to hide loading indicator
            }

            // Execute any pending render
            if let Some((viewport, canvas)) = self.pending_render.take() {
                self.start_worker_computation(&viewport, &canvas);
            }
        }
    }

    pub fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
        if !self.all_workers_ready() {
            // Workers still initializing - queue this render
            self.pending_render = Some((viewport.clone(), canvas.clone()));
            return;
        }

        // Workers ready - dispatch tiles immediately
        self.start_worker_computation(viewport, canvas);
    }

    fn all_workers_ready(&self) -> bool {
        self.workers_ready.load(Ordering::SeqCst) == self.workers.len() as u32
    }
}
```

**Self-balancing properties:**

- Fast tiles (solid areas): worker quickly requests more work
- Slow tiles (fractal details): worker takes longer, others continue
- No worker idles while work remains
- Automatically adapts to varying tile complexity

**Progressive rendering guarantee:** Each `TileComplete` triggers immediate `colorize() + putImageData()`. User sees tiles appear as soon as any worker completes one, starting from center and radiating outward (circular rendering pattern).

#### Circular Tile Ordering

**Goal:** Render tiles starting from the center of the viewport and working outward in a circular/spiral pattern.

**Rationale:**

- Users typically focus on the center of the viewport
- Progressive rendering becomes more visually meaningful when central content appears first
- Provides immediate feedback on the area of interest while periphery fills in

**Algorithm:**

```rust
/// Compute tiles in circular order starting from center
fn compute_tiles_circular(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let center_x = width / 2;
    let center_y = height / 2;

    // Generate all tiles
    let mut tiles = Vec::new();
    for y in (0..height).step_by(tile_size as usize) {
        for x in (0..width).step_by(tile_size as usize) {
            let tile_width = tile_size.min(width - x);
            let tile_height = tile_size.min(height - y);
            tiles.push(PixelRect::new(x, y, tile_width, tile_height));
        }
    }

    // Sort by distance from center (closest first)
    tiles.sort_by_key(|tile| {
        let tile_center_x = tile.x + tile.width / 2;
        let tile_center_y = tile.y + tile.height / 2;
        let dx = (tile_center_x as i32 - center_x as i32).abs();
        let dy = (tile_center_y as i32 - center_y as i32).abs();
        // Use squared distance to avoid sqrt (faster, same ordering)
        (dx * dx + dy * dy) as u32
    });

    tiles
}
```

**Visual pattern:**

```
Render order (1 = first, higher numbers = later):
[5][4][3][4][5]
[4][2][1][2][4]
[3][1][C][1][3]  <- C = center tile (rendered first)
[4][2][1][2][4]
[5][4][3][4][5]
```

### 3. Data Serialization & Transfer - Hybrid Approach

#### Architecture: SharedArrayBuffer + Transferables

**Two types of data flow:**

1. **Reference orbit (large, shared, read-only)** → SharedArrayBuffer
2. **Tile results (small, one-way, write-once)** → Transferable postMessage

#### Reference Orbit Sharing (Perturbation Theory)

**Challenge:** All workers need access to the same reference orbit (~2.5 MB at deep zoom).

**Solution:** SharedArrayBuffer for zero-copy sharing

```rust
// Main thread: Compute reference orbit into shared buffer
fn compute_and_share_reference_orbit(&self, viewport: &Viewport<S>) {
    // Signal not ready
    self.reference_orbit_ready.store(false, Ordering::SeqCst);

    // Compute reference orbit (BigFloat, expensive)
    let orbit_data = compute_reference_orbit(viewport);

    // Create SharedArrayBuffer and copy data
    let shared_buffer = SharedArrayBuffer::new(orbit_data.len() * size_of::<f64>());
    copy_to_shared_buffer(&orbit_data, &shared_buffer);

    self.shared_reference_orbit = Some(shared_buffer.clone());

    // Share reference with all workers (just the buffer reference, not data!)
    for worker in &self.workers {
        worker.post_message(WorkerMessage::SetReferenceOrbit {
            buffer: shared_buffer.clone(),  // ~8 bytes, not 2.5 MB!
        });
    }

    // Signal ready
    self.reference_orbit_ready.store(true, Ordering::SeqCst);
}

// Worker: Wait for reference, then compute tiles
fn on_compute_tile(&self, msg: ComputeTile) {
    // Wait for reference orbit to be ready
    while !self.reference_orbit_ready.load(Ordering::Acquire) {
        // Could use futex or sleep here instead of busy wait
    }

    // Compute tile using shared reference orbit (reads shared memory)
    let tile_data = self.renderer.render_with_perturbation(
        &msg.viewport,
        msg.rect,
        &self.shared_reference_orbit,  // Read-only access
    );

    // Send result via Transferable (as before)
    send_tile_result(tile_data, msg.rect, msg.render_id);
}
```

**Benefits:**
- Memory: 2.5 MB × 1 (shared) vs 2.5 MB × 8 (copies) = **8× memory savings**
- Transfer: ~8 bytes (reference) vs 20 MB (data) = **Instant sharing**
- Read-only: No locks needed, no race conditions

#### Tile Result Transfer (Progressive Rendering)

**Challenge:** Workers send tile results to main thread frequently.

**Solution:** Transferable postMessage for zero-copy one-way transfer

```rust
// Worker side
let data: Vec<D> = renderer.render(viewport, rect, canvas_size);
let bytes = bincode::serialize(&data).unwrap();
post_message(WorkerResponse::TileComplete { render_id, rect, data: bytes });

// Main thread side
let bytes: Vec<u8> = response.data;
let data: Vec<D> = bincode::deserialize(&bytes).unwrap();
// Store in cache, colorize, display
```

**Transfer optimization:** Use `Transferable` for zero-copy transfer of `ArrayBuffer`. Worker loses access after transfer (acceptable - tile is done).

**Type safety:**

- Both main thread and worker compiled with same Rust types
- Serialization format guaranteed to match
- Type mismatch = compile error, not runtime error

#### Summary: Hybrid Data Transfer Strategy

| Data Type | Method | Why | Size | Cost |
|-----------|--------|-----|------|------|
| Reference orbit | SharedArrayBuffer | Large, shared, read-only | ~2.5 MB | One-time copy, shared across workers |
| Tile results | Transferable postMessage | Small, one-way, write-once | ~80 KB | Zero-copy ownership transfer |
| Control messages | Regular postMessage | Tiny, infrequent | <1 KB | Negligible |

**Philosophy:** Use the right tool for each data flow pattern

### 4. Cache Management & Recolorization

#### Cache Structure (Unchanged)

```rust
struct CachedState<S, D: Clone> {
    viewport: Option<Viewport<S>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<D>,  // Raster-order pixel data
    render_id: AtomicU32,
}
```

#### Cache Behavior

**During computation:**

```rust
fn on_tile_complete(&self, render_id: u32, rect: PixelRect, data: Vec<D>) {
    let mut cache = self.cache.lock().unwrap();

    // Check render_id (discard stale tiles)
    if cache.render_id.load(Ordering::SeqCst) != render_id {
        return;
    }

    // Store tile in cache at raster positions (same as TilingCanvasRenderer)
    for local_y in 0..rect.height {
        for local_x in 0..rect.width {
            let cache_idx = ((rect.y + local_y) * width + (rect.x + local_x)) as usize;
            cache.data[cache_idx] = data[tile_idx].clone();
        }
    }

    // Colorize and display immediately (progressive!)
    self.colorize_and_display_tile(&data, rect, canvas);
}
```

**Recolorization (instant, no workers):**

```rust
fn set_colorizer(&mut self, colorizer: Colorizer<D>) {
    self.colorizer = colorizer;

    let cache = self.cache.lock().unwrap();
    let full_rect = PixelRect::full_canvas(width, height);

    // Reuse ALL cached data, just change colors
    self.colorize_and_display_tile(&cache.data, full_rect, canvas);
    // No workers involved, instant!
}
```

**Cache invalidation:**

```rust
fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement) {
    let mut cache = self.cache.lock().unwrap();

    if cache.viewport == Some(viewport) && cache.canvas_size == Some((width, height)) {
        // Same viewport → recolorize from cache (instant)
        self.recolorize_from_cache(canvas);
    } else {
        // Viewport changed → recompute with workers
        cache.data.clear();
        cache.data.resize((width * height) as usize, D::default());
        self.start_worker_computation(viewport, canvas);
    }
}
```

**Key insight:** Cache logic identical to `TilingCanvasRenderer`. Only difference: WHO fills cache (workers vs synchronous loop).

### 5. Cancellation & Responsiveness

#### Cancellation Protocol: Render ID

**Mechanism:**

- Every render has unique `render_id` (monotonically increasing `AtomicU32`)
- When user zooms/pans: increment `render_id`, notify workers
- Workers check `render_id` before sending results
- Main thread discards tiles with old `render_id`

**Implementation:**

```rust
fn cancel_render(&self) {
    // Increment render_id (atomic)
    let new_id = self.cache.lock().unwrap()
        .render_id.fetch_add(1, Ordering::SeqCst) + 1;

    // Clear work queue
    self.tile_queue.lock().unwrap().clear();

    // Notify all workers
    for worker in &self.workers {
        worker.post_message(&WorkerMessage::Cancel { render_id: new_id });
    }
}
```

**Worker side:**

```rust
fn worker_loop(renderer: Box<dyn Renderer>) {
    let mut current_render_id = 0;

    loop {
        match receive_message() {
            WorkerMessage::ComputeTile { render_id, viewport, rect, canvas_size } => {
                current_render_id = render_id;
                let data = renderer.render(&viewport, rect, canvas_size);

                // Check before sending (avoid wasted postMessage)
                if current_render_id == render_id {
                    post_message(WorkerResponse::TileComplete { render_id, rect, data });
                }
            }

            WorkerMessage::Cancel { render_id } => {
                current_render_id = render_id;
                // Current work is stale, won't send it
            }
        }
    }
}
```

#### Responsiveness Guarantees

**Main thread per-tile work:**

- Receive message: ~0.1 ms
- Deserialize data: ~0.5 ms
- Colorize 128×128 tile: ~1-2 ms
- putImageData: ~1 ms
- **Total: ~3 ms** (well under 16 ms frame budget)

**User interaction flow:**

1. User zooms/pans
2. Leptos effect detects viewport change
3. `cancel_render()` called (< 1 ms)
4. `render()` called with new viewport
5. Workers drop old work, start new tiles
6. Main thread remains responsive throughout

### 6. Renderer Swapping & Type Safety

#### Pluggability

**Interface preservation:**

```rust
impl<S, D> CanvasRenderer for WorkerPoolCanvasRenderer<S, D> {
    type Scalar = S;
    type Data = D;

    fn render(&self, viewport: &Viewport<S>, canvas: &HtmlCanvasElement);
    fn set_renderer(&mut self, renderer_type: RendererType);
    fn set_colorizer(&mut self, colorizer: Colorizer<D>);
    fn cancel_render(&self);
    fn natural_bounds(&self) -> Rect<S>;
}
```

**Usage (identical to current):**

```rust
// Swap to Mandelbrot
canvas_renderer.update(|cr| {
    cr.set_renderer(RendererType::Mandelbrot);
});

// Swap to TestImage
canvas_renderer.update(|cr| {
    cr.set_renderer(RendererType::TestImage);
});

// Change colors (instant)
canvas_renderer.update(|cr| {
    cr.set_colorizer(new_colorizer);
});
```

#### Renderer Type Handling

```rust
enum RendererType {
    Mandelbrot,
    TestImage,
}

impl WorkerPoolCanvasRenderer {
    fn set_renderer(&mut self, renderer_type: RendererType) {
        // Terminate all existing workers
        for worker in &self.workers {
            worker.terminate();
        }

        // Clear cache (different renderer = different data type)
        self.cache.lock().unwrap().data.clear();

        // Spawn new workers with new renderer type
        self.workers = spawn_workers(renderer_type, get_hardware_concurrency());
        self.renderer_type = renderer_type;
    }
}
```

**Worker construction:**

```rust
#[wasm_bindgen]
pub fn worker_main() {
    let msg = receive_init_message();

    let renderer: Box<dyn Renderer> = match msg.renderer_type {
        RendererType::Mandelbrot => {
            let computer = MandelbrotComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            Box::new(pixel_renderer)
        }
        RendererType::TestImage => {
            let computer = TestImageComputer::new();
            let pixel_renderer = PixelRenderer::new(computer);
            Box::new(pixel_renderer)
        }
    };

    worker_loop(renderer);
}
```

### 7. Error Handling

#### Worker Failures

```rust
fn setup_worker_error_handler(&self, worker: &Worker, worker_id: usize) {
    let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
        console::error(&format!("Worker {} failed: {}", worker_id, e.message()));
        // Strategy: Continue with remaining workers (degraded performance)
        // Alternative: Respawn failed worker or fail entire render
    }));

    worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
}
```

#### Serialization Failures

```rust
// Worker: wrap serialize in Result
match bincode::serialize(&data) {
    Ok(bytes) => post_message(TileComplete { bytes }),
    Err(e) => post_message(Error { message: e.to_string() }),
}

// Main: handle deserialize failure gracefully
match bincode::deserialize::<D>(&bytes) {
    Ok(data) => { /* cache and display */ }
    Err(e) => {
        console::error(&format!("Deserialize failed: {}", e));
        // Skip this tile, continue render
    }
}
```

#### Race Conditions

- Tile completes after cancellation → `render_id` check discards it (expected behavior)
- Multiple rapid cancellations → `render_id` keeps incrementing, all old work ignored
- Worker in middle of tile when cancelled → completes tile but doesn't send (wasted work, unavoidable)

#### Browser Compatibility

```rust
fn get_hardware_concurrency() -> usize {
    let nav = web_sys::window().unwrap().navigator();
    let cores = nav.hardware_concurrency();
    if cores > 0 { cores as usize } else { 4 } // Fallback
}
```

## Implementation Plan

### File Structure

```
src/rendering/
├── worker_pool_canvas_renderer.rs   (NEW: main thread coordinator)
├── worker/
│   ├── mod.rs                       (NEW: worker entry point)
│   ├── messages.rs                  (NEW: message types)
│   └── worker.js                    (NEW: worker bootstrap script)
├── tiling_canvas_renderer.rs        (DEPRECATED: keep for comparison)
├── canvas_renderer.rs               (UNCHANGED: trait definition)
├── renderer_trait.rs                (UNCHANGED)
└── ...
```

### Integration Points

**In `app.rs`:**

```rust
// Signal for worker pool readiness
let (workers_ready, set_workers_ready) = create_signal(false);

// Create WorkerPoolCanvasRenderer with readiness callback
let canvas_renderer = {
    let mut renderer = WorkerPoolCanvasRenderer::new(
        RendererType::Mandelbrot,
        colorizer,
        128
    );

    // Register callback to signal UI when workers are ready
    renderer.on_ready(move || {
        set_workers_ready.set(true);
    });

    create_rw_signal(renderer)
};

// UI: Show loading indicator while workers initialize
view! {
    <div class="relative w-screen h-screen">
        <Show
            when=move || workers_ready.get()
            fallback=|| view! {
                <LoadingSpinner message="Initializing workers..." />
            }
        >
            <InteractiveCanvas
                canvas_renderer=canvas_renderer
                viewport=viewport
                // ... rest of props
            />
        </Show>
    </div>
}

// Handle renderer swaps with loading state
let on_renderer_select = move |new_renderer_id: String| {
    set_workers_ready.set(false);  // Show loading indicator

    canvas_renderer.update(|cr| {
        cr.set_renderer(new_renderer_type);

        // Re-register callback for new workers
        cr.on_ready(move || {
            set_workers_ready.set(true);  // Hide loading indicator
        });
    });
};
```

### Build Configuration

**Trunk.toml:**

- Add worker build target
- Ensure COOP/COEP headers present (already configured)
- **CRITICAL: Disable file hashing for WASM files** to allow workers to load WASM modules with predictable names
  ```toml
  [[hooks]]
  stage = "post_build"
  command = "sh"
  command_arguments = ["-c", "cd dist && for f in *.wasm; do cp $f ${f%-*}.wasm 2>/dev/null || true; done"]
  ```
  Or use Trunk's asset configuration to disable hashing:
  ```toml
  [build]
  filehash = false
  ```

**Cargo.toml:**

- Add dependencies: `bincode`, `serde` (likely already present)

## Performance Characteristics

### Expected Improvements

**Current (single-threaded):**

- 1 core at 100%
- UI blocked during computation
- No progressive rendering visible

**With workers (8-core machine):**

- 8 cores at ~100% each
- UI responsive (main thread mostly idle)
- Progressive rendering: tiles appear ~8× faster (assuming load balanced)

### Memory Overhead

Per worker:

- WASM module: ~2-5 MB (depends on precision code)
- Renderer state: minimal (mostly stack)
- One tile at a time: ~16-64 KB (128×128 × sizeof(D))

Total for 8 workers: ~40-80 MB (acceptable on modern machines)

### Bottlenecks

**Potential:**

- Message passing overhead (mitigated by processing one tile per message)
- Serialization overhead (bincode is fast, ~1 ms per tile)
- Main thread colorization (batching possible if needed)

**Unlikely:**

- Worker starvation (work queue prevents this)
- Cache contention (tiles write to different indices)

## Success Criteria

1. ✅ User sees tiles appear progressively (not all at once)
2. ✅ UI remains responsive during render (can zoom/pan without lag)
3. ✅ All CPU cores utilized (~100% on each core during render)
4. ✅ Cancellation immediate (< 100 ms from user action to workers stopping)
5. ✅ Recolorization instant (< 100 ms, no recomputation)
6. ✅ Renderer swapping works (Mandelbrot ↔ TestImage)
7. ✅ No regressions in image quality or accuracy

## Risks & Mitigations

| Risk                                  | Mitigation                                                         |
| ------------------------------------- | ------------------------------------------------------------------ |
| Worker spawn/initialization slow      | Spawn workers once at app start, reuse across renders              |
| Serialization overhead too high       | Profile and optimize; consider alternative serialization if needed |
| Main thread colorization bottleneck   | Batch colorization if needed, or move to workers (complex)         |
| Browser worker limits                 | Check `hardwareConcurrency`, cap at reasonable max (e.g., 16)      |
| WASM module size too large per worker | Investigate code splitting, but likely acceptable (<5 MB/worker)   |

## Future Enhancements

1. **Adaptive batching:** Send multiple tiles per message if tiles are very small
2. **Priority queue:** Render center tiles first (user focus area)
3. **Incremental precision:** Render low-precision first, refine progressively
4. **OffscreenCanvas:** Move canvas operations to worker (experimental API)
5. **Shared memory:** Use SharedArrayBuffer for coordinate data (requires careful synchronization)

## Conclusion

This design enables true progressive rendering with maximum CPU utilization while preserving the existing clean architecture. The `CanvasRenderer` abstraction remains intact, allowing seamless integration with the current codebase. Main thread responsiveness is guaranteed through careful work budgeting and the render ID cancellation protocol.

**Progressive rendering guarantee:** ✅ YES - Each worker sends results immediately upon tile completion. Main thread processes and displays tiles as they arrive. User sees tiles appearing one-by-one in real-time.
