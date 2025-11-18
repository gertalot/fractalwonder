# Message-Based Parallel Rendering Architecture

**Date:** 2025-11-17
**Status:** Design Approved
**Replaces:** SharedArrayBuffer-based parallel rendering with atomic counters

---

## Problem Statement

The current parallel rendering system has critical issues:

1. **Race conditions**: SharedArrayBuffer with atomic counters causes memory ordering issues - black rectangles appear because main thread reads pixel data before workers' non-atomic writes become visible
2. **Brittle timing hack**: "Extra polling frames" workaround depends on hardware-specific cache coherency timing
3. **Wrong renderer**: Workers use `MandelbrotComputer::<f64>` instead of `AdaptiveMandelbrotRenderer`, causing blocky rendering at deep zoom levels
4. **Complex synchronization**: Atomic operations, memory fences, and polling loops add unnecessary complexity

---

## Solution Overview

Replace SharedArrayBuffer with **message-based worker coordination** using a request/response pattern. Workers pull work when ready (not pushed), eliminating all SharedArrayBuffer complexity while enabling clean cancellation, progressive rendering, and data caching.

---

## Architecture

### High-Level Components

**1. Worker Pool (Main Thread)**
- Creates N workers based on hardware concurrency
- Maintains queue of pending tiles
- Tracks current `render_id` for cancellation
- Routes work requests to available workers
- Caches computed fractal data for recolorization

**2. Workers (Compute Threads)**
- Create `AdaptiveMandelbrotRenderer` once at startup (fixes precision bug)
- Send `RequestWork` on startup and after completing each tile
- Compute fractal data (iterations + escaped flag), not RGBA pixels
- Return tile data via transferable arrays (zero-copy)
- Reused across renders until renderer type changes

**3. Message Protocol**
- Worker→Main: `RequestWork`, `TileComplete`, `Error`
- Main→Worker: `RenderTile`, `NoWork`, `Terminate`
- Metadata: JSON (viewport, tile coords, render_id)
- Pixel data: Transferable `Uint32Array` (zero-copy transfer)

### Data Flow

```
User pans/zooms → Main increments render_id → Main generates tiles →
Worker requests work → Main sends tile → Worker computes →
Worker sends result + requests next → Main stores in cache →
Main colorizes & draws → Repeat until all tiles complete
```

---

## Message Protocol

### Worker → Main Messages

```rust
enum WorkerToMain {
    /// Worker requests work assignment
    RequestWork {
        render_id: Option<u32>
        // None = worker just started, will accept any work
        // Some(id) = finished work for this render, wants more from same render
    },

    /// Worker completed a tile
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<MandelbrotData>,  // Transferred, not copied
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

### Main → Worker Messages

```rust
enum MainToWorker {
    /// Assign tile to render
    RenderTile {
        render_id: u32,
        viewport_json: String,  // Serialized Viewport<BigFloat>
        tile: PixelRect,
        canvas_width: u32,
        canvas_height: u32,
    },

    /// No work available (render complete or queue empty)
    NoWork,

    /// Terminate worker
    Terminate,
}
```

### Transfer Strategy

- **Metadata**: JSON serialized (small, readable, debuggable)
- **Pixel data**: Transferable `Uint32Array` (zero-copy, browser-native)
- **Size per tile**: ~131KB (16,384 pixels × 8 bytes)
- **Benefit**: Browser handles transfer efficiently without copying memory

### Error Handling

1. Worker sends `Error` message with render_id, tile, and error string
2. Main thread logs error to browser console
3. Tile re-queued at end of queue (retry once with different worker)
4. If second worker also fails: skip tile, appears black on canvas

### Worker Lifecycle

- **On creation**: Worker sends `RequestWork { render_id: None }` immediately
- **After each tile**: Worker sends result AND `RequestWork { render_id: Some(id) }`
- **On renderer switch**: Main terminates all workers, creates new pool
- **On idle**: Worker waits for next `RenderTile` message (stays alive)

---

## Worker Implementation

### Initialization

```rust
#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();

    // Create adaptive renderer ONCE at startup (fixes precision bug!)
    let renderer = AdaptiveMandelbrotRenderer::new(1e10);

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
        match parse_message(e.data()) {
            Ok(MainToWorker::RenderTile { ... }) => { /* handle */ }
            Ok(MainToWorker::NoWork) => { /* go idle */ }
            Ok(MainToWorker::Terminate) => { global.close(); }
            Err(e) => { log_error(e); }
        }
    }));

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // NO "Ready" message - just request work immediately
    request_work(None);
}
```

### Tile Rendering

```rust
RenderTile { render_id, viewport_json, tile, canvas_width, canvas_height } => {
    let start_time = now();

    // Parse BigFloat viewport
    let viewport: Viewport<BigFloat> = serde_json::from_str(&viewport_json)?;

    // Render tile (may take 0-60+ seconds at extreme zoom)
    let tile_data = renderer.render(&viewport, tile, (canvas_width, canvas_height));

    let compute_time_ms = now() - start_time;

    // Send result with transferable array (zero-copy)
    send_tile_complete(render_id, tile, tile_data, compute_time_ms);

    // Immediately request next work from same render
    request_work(Some(render_id));
}
```

### Key Points

- Renderer created once at startup, reused for all tiles and renders
- No cancellation during tile computation (between tiles only)
- Transferable array for pixel data ensures zero-copy transfer
- Timing data included for performance monitoring
- Error handling wraps computation, reports failures to main thread

---

## WorkerPool Implementation

### Structure

```rust
pub struct WorkerPool {
    workers: Vec<Worker>,
    pending_tiles: VecDeque<TileRequest>,
    current_render_id: u32,
    current_viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
}
```

### Initialization

- Detects hardware concurrency (`navigator.hardwareConcurrency`)
- Creates N workers with message handlers
- Workers automatically request work on startup
- Main thread waits for `RequestWork` messages (no separate "ready" phase)

### Message Handling

```rust
RequestWork { render_id } => {
    // Worker wants work
    if render_id matches current_render_id (or is None):
        send next tile from queue
    else:
        send NoWork (stale render, worker should go idle)
}

TileComplete { render_id, tile, data, compute_time_ms } => {
    if render_id matches current_render_id:
        call on_tile_complete callback (stores in cache + draws)
    else:
        ignore stale result, log discard
}

Error { render_id, tile, error } => {
    log error to console with full context
    re-queue tile at end of queue (retry once)
}
```

### Starting a Render

```rust
pub fn start_render(&mut self, viewport: Viewport<BigFloat>, width: u32, height: u32, tile_size: u32) {
    // Increment render_id (automatically cancels previous render)
    self.current_render_id += 1;

    // Generate tiles (center-first ordering)
    self.pending_tiles = generate_tiles(width, height, tile_size);

    // Store viewport and canvas size
    self.current_viewport = viewport;
    self.canvas_size = (width, height);

    // That's it! Workers request work when ready
}
```

### Tile Generation

- **Fixed grid**: 128×128 pixel tiles (adjustable)
- **Ordering**: Sorted by distance from canvas center (center renders first for better UX)
- **Storage**: `VecDeque` for efficient `pop_front()` during work distribution
- **Future**: Replace with adaptive quadtree when implementing dynamic subdivision

---

## Main Thread Integration

### Structure

```rust
struct CachedState {
    viewport: Option<Viewport<BigFloat>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<MandelbrotData>,  // Full canvas in raster order
    render_id: AtomicU32,
}

pub struct ParallelCanvasRenderer {
    worker_pool: Rc<RefCell<WorkerPool>>,
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
}
```

### Rendering Decision Logic

```rust
fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
    let mut cache = self.cached_state.lock().unwrap();
    let render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

    // Convert f64 viewport to BigFloat (UI uses f64, workers need BigFloat)
    let viewport_bf = convert_to_bigfloat(viewport);

    let width = canvas.width();
    let height = canvas.height();

    if cache.viewport == Some(viewport_bf) && cache.canvas_size == Some((width, height)) {
        // Same viewport → recolorize from cache (fast, no computation)
        drop(cache);
        self.recolorize_from_cache(render_id, canvas);
    } else {
        // Viewport changed → recompute fractal data
        cache.data.clear();
        cache.data.resize((width * height) as usize, MandelbrotData::default());
        cache.viewport = Some(viewport_bf.clone());
        cache.canvas_size = Some((width, height));
        drop(cache);

        // Start worker computation
        self.worker_pool.borrow_mut().start_render(viewport_bf, width, height, self.tile_size);
    }
}
```

### Tile Completion Callback

```rust
let on_tile_complete = move |tile_result: TileResult| {
    let mut cache = cached_state.lock().unwrap();

    // Store tile data in cache at correct raster positions
    for local_y in 0..tile_result.tile.height {
        for local_x in 0..tile_result.tile.width {
            let canvas_x = tile_result.tile.x + local_x;
            let canvas_y = tile_result.tile.y + local_y;
            let cache_idx = (canvas_y * width + canvas_x) as usize;
            let tile_idx = (local_y * tile_result.tile.width + local_x) as usize;
            cache.data[cache_idx] = tile_result.data[tile_idx];
        }
    }

    drop(cache);

    // Colorize and draw tile immediately (progressive rendering)
    draw_tile(canvas, &tile_result, &colorizer);
};
```

### Recolorization from Cache

```rust
fn recolorize_from_cache(&self, render_id: u32, canvas: &HtmlCanvasElement) {
    let cache = self.cached_state.lock().unwrap();

    // Check if render was cancelled
    if cache.render_id.load(Ordering::SeqCst) != render_id {
        return; // Cancelled during recolorization
    }

    // Colorize entire cached data array
    let colors: Vec<u8> = cache.data
        .iter()
        .flat_map(|data| {
            let app_data = AppData::MandelbrotData(*data);
            let (r, g, b, a) = (self.colorizer)(&app_data);
            [r, g, b, a]
        })
        .collect();

    // Draw to canvas in one operation
    let image_data = ImageData::new_with_u8_clamped_array(
        Clamped(&colors),
        canvas.width()
    ).unwrap();

    context.put_image_data(&image_data, 0.0, 0.0).unwrap();
}
```

### Progressive Tile Drawing

```rust
fn draw_tile(canvas: &HtmlCanvasElement, tile_result: &TileResult, colorizer: &Colorizer<AppData>) {
    // Colorize tile data
    let colors: Vec<u8> = tile_result.data
        .iter()
        .flat_map(|data| {
            let app_data = AppData::MandelbrotData(*data);
            let (r, g, b, a) = colorizer(&app_data);
            [r, g, b, a]
        })
        .collect();

    // Create ImageData for this tile
    let image_data = ImageData::new_with_u8_clamped_array(
        Clamped(&colors),
        tile_result.tile.width
    ).unwrap();

    // Draw at tile position (progressive rendering - user sees tiles appear)
    let context = canvas.get_context("2d").unwrap().unwrap()
        .dyn_into::<CanvasRenderingContext2d>().unwrap();

    context.put_image_data(
        &image_data,
        tile_result.tile.x as f64,
        tile_result.tile.y as f64
    ).unwrap();
}
```

---

## Key Design Decisions

### Cancellation Strategy

**Decision**: Increment `render_id`, workers finish current tile, ignore stale results

**Rationale**:
- Simple: No explicit cancel messages, no coordination needed
- Automatic: Workers naturally transition to new work when they request next tile
- Trade-off: Workers finish current tile before switching (0-60+ seconds at extreme zoom)
- Acceptable: Tiles are usually fast; slow tiles are rare edge cases at extreme zoom
- UI remains responsive: New render starts immediately, old results simply ignored

**Alternative considered**: Send explicit `CancelRender` message with mid-tile cancellation checks
- Rejected: Adds complexity to renderer interface, requires modifying renderer trait
- Would need: Renderer to accept cancellation token, check frequently during computation
- Benefit: Stop expensive tiles faster
- Cost: Pollutes clean renderer interface, harder to swap renderers

### Data Transfer Method

**Decision**: Transferable `Uint32Array` for pixel data, JSON for metadata

**Rationale**:
- Zero-copy: Transferable arrays move ownership, no memory copying
- Browser-native: Designed exactly for this use case
- Fast: Much faster than JSON serialization for large binary data
- Debuggable: Can still inspect metadata (JSON) in console, binary data transferred efficiently
- Small metadata: Viewport, coords, render_id easily fit in JSON

**Alternative considered**: Pure JSON serialization
- Rejected: JSON stringify/parse overhead on ~131KB per tile is wasteful
- Would work: But slower and uses more memory

**Alternative considered**: MessagePack binary format
- Rejected: Extra dependency, less debuggable, minimal benefit over hybrid approach

### Renderer Creation Strategy

**Decision**: Create renderer once at worker startup, reuse for all renders

**Rationale**:
- Efficient: No repeated allocation overhead
- Safe: `AdaptiveMandelbrotRenderer` is stateless, safe to reuse
- Simple: No per-render or per-tile setup

**Renderer switching**: When user switches renderer type, main thread terminates all workers and creates new pool
- Trade-off: ~100ms overhead on renderer switch
- Acceptable: Renderer switches are infrequent user actions
- Simple: Avoids complex renderer configuration protocol

### Cache Management

**Decision**: Store full canvas data (`Vec<MandelbrotData>`) in raster order

**Rationale**:
- Enables instant recolorization when user changes color scheme
- Fractal computation is expensive (seconds to minutes)
- Coloring is cheap (milliseconds)
- Better UX: Color scheme changes feel instant
- Memory cost: Acceptable (8 bytes per pixel)

**Inspired by**: `TilingCanvasRenderer` implementation which demonstrates this pattern cleanly

### Error Handling

**Decision**: Retry failed tiles once with different worker, then skip

**Rationale**:
- Robust: Transient errors (FP edge cases) might succeed on retry
- Simple: No complex retry logic, just re-queue once
- Safe: Won't infinite loop on permanently broken tiles
- Visible: Failed tiles appear black (user can report issues)

---

## Implementation Benefits

### Over Current System

1. **No race conditions**: Messages provide explicit synchronization, no memory ordering issues
2. **No timing hacks**: No "extra polling frames" workaround needed
3. **Fixes precision bug**: Workers use `AdaptiveMandelbrotRenderer` instead of fixed f64
4. **Simpler code**: Event-driven message handling, no atomic operations
5. **Better UX**: Progressive rendering, instant recolorization, center-first tile ordering
6. **Debuggable**: Can inspect message flow in browser console

### Performance Characteristics

- **Natural load balancing**: Work-stealing pattern (fast workers get more tiles automatically)
- **Progressive rendering**: Tiles appear as completed, user sees results immediately
- **Efficient recolorization**: Cached data avoids recomputation when only color scheme changes
- **Zero-copy transfer**: Transferable arrays move ownership without memory copying
- **Timing data**: Per-tile compute times logged for performance monitoring

### Future Extensions

1. **Adaptive quadtree**: Main thread can dynamically subdivide complex tiles and add to queue
2. **Distributed rendering**: Same message protocol works over WebSocket (workers can be remote servers)
3. **Multiple renderers**: Easy to extend when Julia set, Burning Ship, etc. are added
4. **Progress reporting**: Can add total/completed tile counts to UI
5. **Work prioritization**: Can reorder pending tiles based on user interest (viewport changes)

---

## Current Limitations

1. **No mid-tile cancellation**: Workers must finish current tile before switching renders
   - Impact: Expensive tiles at extreme zoom (60+ seconds) delay render switching
   - Mitigation: Rare edge case, UI remains responsive (new render starts immediately)

2. **Fixed tile grid**: Not yet adaptive to fractal complexity
   - Impact: Equal work per tile even if some regions are simpler
   - Future: Replace with quadtree subdivision

3. **Renderer switching overhead**: Must recreate all workers (~100ms)
   - Impact: Brief delay when user switches between Mandelbrot/Julia/etc
   - Acceptable: Infrequent user action

4. **Memory usage**: Stores full canvas data for caching
   - Impact: 8 bytes per pixel (e.g., 7.8MB for 1920×1080 canvas)
   - Acceptable: Modern devices have plenty of RAM, enables instant recolorization

---

## Migration Path

1. Keep existing `ParallelCanvasRenderer` as fallback during development
2. Implement new `MessageBasedParallelRenderer` alongside
3. Test thoroughly with various zoom levels and canvas sizes
4. Switch default renderer in `App.rs`
5. Remove old `SharedArrayBuffer`-based implementation
6. Update `compute-worker.js` to load new worker entry point

---

## Success Criteria

- ✅ No black rectangles at any zoom level
- ✅ Smooth progressive rendering (tiles appear center-first)
- ✅ Instant recolorization when changing color schemes
- ✅ Clean cancellation (pan/zoom immediately starts new render)
- ✅ Correct precision at extreme zoom (no blockiness)
- ✅ Performance comparable to or better than current system
- ✅ No memory leaks over extended use
- ✅ Clean error messages for worker failures
