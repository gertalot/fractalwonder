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
}

impl RenderProgress {
    pub fn new(total_tiles: u32, render_id: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            render_id,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }

    pub fn is_complete(&self) -> bool {
        self.completed_tiles >= self.total_tiles
    }
}
```

**Type properties:**
- `Copy`: Zero-cost signal updates
- `PartialEq`: Leptos skips re-renders when unchanged
- `render_id`: Prevents stale progress from cancelled renders

## Where Progress Information Originates

### Render Start (message_worker_pool.rs)

```rust
pub fn start_render(&self, viewport: &Viewport<BigFloat>, ...) {
    let total_tiles = pending_tiles.len() as u32;
    let render_id = self.render_id.fetch_add(1, Ordering::SeqCst);

    // Initialize progress: 0 of N tiles
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

            // Increment completed count
            self.progress_signal.update(|p| {
                if p.render_id == render_id {
                    p.completed_tiles += 1;
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

## Files Modified

- `fractalwonder-core/src/rendering/mod.rs` - Add RenderProgress struct
- `fractalwonder-ui/src/workers/message_worker_pool.rs` - Accept and update progress signal
- `fractalwonder-ui/src/rendering/message_parallel_renderer.rs` - Own progress signal, pass to pool
- `fractalwonder-ui/src/app.rs` - Read progress signal, provide via context (optional)

## Example: Derived State

Components can derive additional state from progress:

```rust
// Is rendering currently active?
let is_rendering = create_memo(move |_| {
    let progress = render_progress.get();
    !progress.is_complete() && progress.total_tiles > 0
});

// Time remaining estimate (requires additional tracking)
let estimated_ms_remaining = create_memo(move |_| {
    let progress = render_progress.get();
    let elapsed = elapsed_ms.get();

    if progress.completed_tiles == 0 {
        return None;
    }

    let ms_per_tile = elapsed / progress.completed_tiles as f64;
    let remaining_tiles = progress.total_tiles - progress.completed_tiles;
    Some(ms_per_tile * remaining_tiles as f64)
});
```

## Edge Cases

**Fast renders:** Progress signal updates but UI may not render if completion is immediate.

**Cancelled renders:** New `render_id` causes old progress updates to be ignored via comparison check.

**Worker errors:** Progress increments only on successful tile completion (existing retry logic unchanged).
