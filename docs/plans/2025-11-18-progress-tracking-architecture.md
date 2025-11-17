# Progress Tracking Architecture

## Summary

This document defines how render progress data flows from worker pool to UI components using Leptos signals. The architecture leverages existing infrastructure and idiomatic Leptos patterns.

## Progress Data Structure

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub render_id: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    pub fn new(total_tiles: u32, render_id: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            render_id,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}
```

**Type properties:**
- `Copy`: Zero-cost signal updates
- `PartialEq`: Leptos skips re-renders when unchanged
- `render_id`: Prevents stale progress from cancelled renders
- `elapsed_ms`: Time since render started, updated progressively
- `is_complete`: Whether all tiles finished (replaces derived method)

## Where Progress Information Originates

### Render Start (message_worker_pool.rs)

```rust
pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    progress_signal: RwSignal<RenderProgress>,
    render_start_time: Cell<Option<f64>>,  // Track start time
    // ...
}

pub fn start_render(&self, viewport: &Viewport<BigFloat>, ...) {
    let total_tiles = pending_tiles.len() as u32;
    let render_id = self.render_id.fetch_add(1, Ordering::SeqCst);

    // Record start time
    let start_time = window().performance().unwrap().now();
    self.render_start_time.set(Some(start_time));

    // Initialize progress: 0 of N tiles, 0ms elapsed
    self.progress_signal.set(RenderProgress::new(total_tiles, render_id));

    // Distribute tiles to workers...
}
```

### Tile Completion (message_worker_pool.rs)

```rust
fn handle_worker_message(&self, msg: WorkerToMain) {
    match msg {
        WorkerToMain::TileComplete { render_id, .. } => {
            if render_id != self.render_id.load(Ordering::SeqCst) {
                return; // Ignore stale renders
            }

            // Calculate elapsed time
            let elapsed_ms = if let Some(start) = self.render_start_time.get() {
                window().performance().unwrap().now() - start
            } else {
                0.0
            };

            // Update progress with tile count and elapsed time
            self.progress_signal.update(|p| {
                if p.render_id == render_id {
                    p.completed_tiles += 1;
                    p.elapsed_ms = elapsed_ms;
                    p.is_complete = p.completed_tiles >= p.total_tiles;
                }
            });

            // Invoke tile completion callback...
        }
    }
}
```

## Signal Ownership and Sharing

### Renderer Owns the Signal

```rust
// message_parallel_renderer.rs
pub struct MessageParallelRenderer {
    worker_pool: MessageWorkerPool,
    progress: RwSignal<RenderProgress>,
    // ...
}

impl MessageParallelRenderer {
    pub fn new(...) -> Self {
        let progress = RwSignal::new(RenderProgress::new(0, 0));

        let worker_pool = MessageWorkerPool::new(
            ...,
            progress, // Pass signal to worker pool
        );

        Self { worker_pool, progress, ... }
    }

    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }
}
```

### Worker Pool Updates the Signal

```rust
// message_worker_pool.rs
pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    progress_signal: RwSignal<RenderProgress>,
    // ...
}
```

**Key insight:** `RwSignal<T>` is `Copy`, so passing it to worker pool is zero-cost.

## How App Reads Progress

### Direct Signal Access

```rust
// app.rs
let canvas_renderer: RwSignal<CanvasRendererHolder> = create_rw_signal(...);

let render_progress = create_memo(move |_| {
    canvas_renderer.with(|cr| cr.progress().get())
});

view! {
    <ProgressIndicator progress=render_progress />
}
```

### Context API (for deep component trees)

```rust
// app.rs
let progress_signal = canvas_renderer.with_untracked(|cr| cr.progress());
provide_context(progress_signal);

// child_component.rs
let progress = use_context::<RwSignal<RenderProgress>>()
    .expect("progress signal provided");

view! {
    <div>{move || format!("{:.1}%", progress.get().percentage())}</div>
}
```

## Data Flow

```
App Component
    │
    ├─> canvas_renderer.with(|cr| cr.progress())  [READ]
    │
    └─> provide_context(progress_signal)
            │
            └─> Available to all child components
                    │
                    ↓
            UI components re-render automatically


MessageParallelRenderer [OWNS SIGNAL]
    │
    └─> progress: RwSignal<RenderProgress>
            │
            └─> Passed to MessageWorkerPool
                    │
                    ↓
            MessageWorkerPool [UPDATES SIGNAL]
                │
                ├─> start_render()
                │       └─> progress_signal.set(...)  [RESET]
                │
                └─> handle_worker_message()
                        └─> progress_signal.update(|p| p.completed_tiles += 1)  [INCREMENT]
                                │
                                ↓ (Leptos reactive propagation)
                        UI automatically re-renders
```

## Thread Safety

- `RwSignal::update()` is atomic
- Safe to call from message handlers
- `render_id` comparison prevents race conditions with cancelled renders

## Why This Approach

1. **Idiomatic Leptos:** Signals are first-class reactive primitives
2. **Zero ceremony:** No callbacks needed in App component
3. **Automatic reactivity:** UI updates when signal changes
4. **Type-safe:** Compiler enforces correct usage
5. **Consistent:** Matches existing patterns (viewport, render_time_ms)
6. **Context API:** Deep components access progress without prop drilling
7. **Single source of truth:** Renderer owns the signal

## Integration with App Component

The `RenderProgress` signal replaces the current broken `render_time_ms` signal:

### Current (Broken)

```rust
// app.rs
let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

create_effect(move |_| {
    let vp = viewport.get();
    let mut info = info_provider.info(&vp);
    info.render_time_ms = render_time_ms.get();  // Always None or ~0ms
    set_renderer_info.set(info);
});

// interactive_canvas.rs - WRONG: measures function call, not async work
let start = window().performance().unwrap().now();
canvas_renderer.with(|cr| cr.render(&vp, canvas));
let elapsed = window().performance().unwrap().now() - start;
set_render_time_ms.set(Some(elapsed));  // ~0ms
```

### Proposed (Correct)

```rust
// app.rs
let render_progress = create_memo(move |_| {
    canvas_renderer.with(|cr| cr.progress().get())
});

create_effect(move |_| {
    let vp = viewport.get();
    let mut info = info_provider.info(&vp);
    info.render_time_ms = Some(render_progress.get().elapsed_ms);  // Accurate!
    set_renderer_info.set(info);
});

// interactive_canvas.rs - Remove timing code, let worker pool handle it
canvas_renderer.with(|cr| cr.render(&vp, canvas));  // No timing wrapper
```

### Migration Path

1. Add `progress: RwSignal<RenderProgress>` to `MessageParallelRenderer`
2. Pass progress signal to `MessageWorkerPool`
3. Worker pool updates `elapsed_ms` on each tile completion
4. Remove `set_render_time_ms` prop from `InteractiveCanvas`
5. Update App to read `progress.elapsed_ms` instead of `render_time_ms` signal
6. Remove broken timing code from `InteractiveCanvas`

## Files Modified

- `fractalwonder-core/src/rendering/mod.rs` - Add RenderProgress struct
- `fractalwonder-ui/src/workers/message_worker_pool.rs` - Add render_start_time field, update progress signal with elapsed_ms
- `fractalwonder-ui/src/rendering/message_parallel_renderer.rs` - Own progress signal, pass to pool, handle recolorization timing
- `fractalwonder-ui/src/components/interactive_canvas.rs` - Remove broken timing code, remove set_render_time_ms prop
- `fractalwonder-ui/src/app.rs` - Replace render_time_ms signal with progress signal, update RendererInfo effect

## Elapsed Time Tracking

### Why Track Time in Worker Pool

The current `InteractiveCanvas` component measures time incorrectly:

```rust
// WRONG: Measures only time for render() call, which returns immediately
let start = window().performance().unwrap().now();
canvas_renderer.with(|cr| cr.render(&vp, canvas));  // Returns instantly
let elapsed = window().performance().unwrap().now() - start;  // ~0ms
```

**Problem:** `render()` returns immediately after calling `start_render()`. Workers compute tiles asynchronously.

**Solution:** Worker pool tracks time from `start_render()` call until each tile completes.

### Recolorization Handling

When recolorizing from cache (viewport unchanged, colorizer changed):

```rust
// In message_parallel_renderer.rs
fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
    if cache.viewport == Some(viewport) && cache.canvas_size == Some((width, height)) {
        // Recolorize from cache - measure this operation
        let start = window().performance().unwrap().now();
        let _ = self.recolorize_from_cache(render_id, canvas);
        let elapsed = window().performance().unwrap().now() - start;

        // Update progress for instant completion
        self.progress.set(RenderProgress {
            completed_tiles: total_tiles,
            total_tiles,
            render_id,
            elapsed_ms: elapsed,
            is_complete: true,
        });
    } else {
        // Normal tile-based rendering
        self.worker_pool.start_render(...);  // Pool tracks time
    }
}
```

### Benefits

1. **Accurate:** Measures actual parallel render time, not just function call overhead
2. **Progressive:** Updates continuously as tiles complete
3. **Replaces broken implementation:** Fixes current `render_time_ms` signal that always shows ~0ms
4. **Handles both paths:** Works for cache hits (recolorization) and cache misses (recompute)

## Example: Derived State

Components can derive additional state from progress:

```rust
// Is rendering currently active?
let is_rendering = create_memo(move |_| {
    let progress = render_progress.get();
    !progress.is_complete
});

// Time remaining estimate
let estimated_ms_remaining = create_memo(move |_| {
    let progress = render_progress.get();

    if progress.completed_tiles == 0 || progress.is_complete {
        return None;
    }

    let ms_per_tile = progress.elapsed_ms / progress.completed_tiles as f64;
    let remaining_tiles = progress.total_tiles - progress.completed_tiles;
    Some(ms_per_tile * remaining_tiles as f64)
});

// Format elapsed time for display
let formatted_time = create_memo(move |_| {
    let elapsed = render_progress.get().elapsed_ms;
    if elapsed < 1000.0 {
        format!("{:.0}ms", elapsed)
    } else {
        format!("{:.1}s", elapsed / 1000.0)
    }
});
```

## Edge Cases

**Fast renders:** Progress signal updates but UI may not render if completion is immediate.

**Cancelled renders:** New `render_id` causes old progress updates to be ignored via comparison check.

**Worker errors:** Progress increments only on successful tile completion (existing retry logic unchanged).
