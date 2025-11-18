# Progress Indicators Design

## Overview

Add visual progress indicators to show rendering status: a circular pie chart indicator in the lower-left corner when UI is hidden, and a linear progress bar next to the color menu when UI is visible.

## Requirements

**Circular Progress Indicator (Lower Left):**
- 24×24px pie chart, clockwise fill from 12 o'clock
- Position: 20px from bottom-left corner
- Style: `bg-black/30`, `backdrop-blur-sm`, `rounded-full`, `rgb(224,224,224)` fill
- Gentle shimmer animation (moving gradient sweep)
- Visibility: Show after 1 second of rendering, fade out when UI fades in

**Linear Progress Bar (Next to Color Menu):**
- Max 100px width, dynamic sizing
- Position: Right of color menu dropdown, vertically centered in UI panel
- Style: `bg-black/30`, `backdrop-blur-sm`, `rounded-lg`, `rgb(224,224,224)` fill
- Gentle shimmer animation (moving gradient sweep)
- Visibility: Show immediately when rendering while UI is visible

## Architecture

### Progress Data Model

```rust
pub struct RenderProgress {
    pub completed_tiles: usize,
    pub total_tiles: Option<usize>,
    pub is_rendering: bool,
    pub render_id: u32,
}
```

**Rationale:**
- `total_tiles` is `Option` to support future adaptive quadtree tiling where total is unknown upfront
- `render_id` allows detecting when a new render starts (progress resets)
- Immutable snapshot prevents threading issues

### Observer Pattern Implementation

**MessageWorkerPool changes:**
- Add fields: `completed_tiles: usize`, `total_tiles: Option<usize>`, `on_progress: Option<Rc<dyn Fn(RenderProgress)>>`
- Constructor accepts optional progress callback
- `start_render()`: Reset progress, call callback with initial state
- `handle_worker_message(TileComplete)`: Increment completed, call callback
- When all tiles complete: Set `is_rendering=false`, call callback

**Why observer pattern:**
- Pool owns tile state (source of truth)
- Callback crosses abstraction boundary (renderer → app)
- No trait changes needed
- Optional = no performance cost if unused

### Leptos Signal Integration

**app.rs changes:**

Add signals:
```rust
let (tiles_completed, set_tiles_completed) = create_signal(0usize);
let (tiles_total, set_tiles_total) = create_signal(None::<usize>);
let (is_rendering, set_is_rendering) = create_signal(false);
let (render_start_time, set_render_start_time) = create_signal(None::<f64>);
```

Progress callback:
```rust
let progress_callback = move |progress: RenderProgress| {
    set_tiles_completed.set(progress.completed_tiles);
    set_tiles_total.set(progress.total_tiles);
    set_is_rendering.set(progress.is_rendering);

    if progress.completed_tiles == 0 && progress.is_rendering {
        set_render_start_time.set(Some(window().performance().now()));
    } else if !progress.is_rendering {
        set_render_start_time.set(None);
    }
};
```

Pass to renderer:
```rust
MessageParallelRenderer::new(colorizer, tile_size, Some(Rc::new(progress_callback)))
```

**Data flow:**
```
MessageWorkerPool (tracks progress, calls callback)
  ↓
MessageParallelRenderer (passes callback through)
  ↓
app.rs (updates Leptos signals)
  ↓
UI components (display progress indicators)
```

## Components

### CircularProgress Component

**Location:** `fractalwonder-ui/src/components/circular_progress.rs`

**Props:**
- `completed: ReadSignal<usize>`
- `total: ReadSignal<Option<usize>>`
- `is_rendering: ReadSignal<bool>`
- `elapsed_ms: ReadSignal<Option<f64>>`

**Logic:**
```rust
let progress_percent = create_memo(move |_| {
    total.get().map(|t| {
        if t > 0 { (completed.get() as f64 / t as f64 * 100.0) as u32 }
        else { 0 }
    }).unwrap_or(0)
});

let should_show = create_memo(move |_| {
    is_rendering.get() && elapsed_ms.get().map(|ms| ms > 1000.0).unwrap_or(false)
});
```

**Rendering:**
- SVG pie chart: 24×24px circle with conic gradient or arc path
- Fill clockwise from top (12 o'clock position)
- Position: `fixed bottom-5 left-5`
- Style: `bg-black/30 backdrop-blur-sm rounded-full`
- Foreground: `rgb(224,224,224)`
- Visibility: Fades opposite to UI panel (300ms transition)

### LinearProgress Component

**Location:** `fractalwonder-ui/src/components/linear_progress.rs`

**Props:**
- `completed: ReadSignal<usize>`
- `total: ReadSignal<Option<usize>>`
- `is_rendering: ReadSignal<bool>`

**Logic:**
```rust
let progress_percent = create_memo(move |_| {
    total.get().map(|t| {
        if t > 0 { (completed.get() as f64 / t as f64 * 100.0) as u32 }
        else { 0 }
    }).unwrap_or(0)
});

let should_show = create_memo(move |_| {
    is_rendering.get()
});
```

**Rendering:**
- Horizontal bar container: `max-w-[100px] h-2 rounded-lg`
- Background: `bg-black/30 backdrop-blur-sm`
- Fill: `bg-[rgb(224,224,224)]` width based on percentage
- Position: Right of color menu in UI panel, vertically centered using flexbox
- Visibility: Shows/hides with UI panel (no delay)

### Shimmer Animation

**CSS keyframes:**
```css
@keyframes shimmer {
  0% { background-position: -100% 0; }
  100% { background-position: 200% 0; }
}
```

**Applied to fill elements:**
```rust
class="bg-linear-to-r from-[rgb(224,224,224)] via-white to-[rgb(224,224,224)]
       bg-size-[200%_100%] animate-shimmer"
```

**Configuration:**
- Duration: 3s
- Easing: linear
- Iteration: infinite

## Edge Cases

**Fast renders (<1 second):**
- Circular indicator never appears
- Linear bar shows briefly if UI visible

**UI toggling during render:**
- Circular shows after 1s when UI hidden
- Linear shows when UI visible
- Both can briefly coexist during transitions

**Unknown total tiles (adaptive tiling):**
- `total_tiles = None`
- Show indeterminate state (shimmer continues)
- Components handle `None` gracefully

**Stale render detection:**
- `render_id` changes when new render starts
- Progress resets to 0

**Worker errors/retries:**
- Progress increments only on successful completion
- Failed tiles don't increment counter
- Pool already handles retry logic

## Testing Strategy

**Unit tests:**
- Progress calculation with various tile counts
- Edge cases (zero tiles, None total)
- Visibility logic with timing thresholds

**Integration tests:**
- Verify callback fires on tile completion
- Verify signals update correctly
- Verify render_id increments on new render

**Browser tests:**
- Visual verification of animations
- Timing verification (1-second delay)
- UI toggle during render
- Fast render (no indicator shown)

## Files Modified

- `fractalwonder-ui/src/workers/message_worker_pool.rs` - Add progress tracking
- `fractalwonder-ui/src/rendering/message_parallel_renderer.rs` - Pass callback through
- `fractalwonder-ui/src/app.rs` - Add signals and callback
- `fractalwonder-ui/src/components/circular_progress.rs` - New component
- `fractalwonder-ui/src/components/linear_progress.rs` - New component
- `fractalwonder-ui/src/components/mod.rs` - Export new components
- `fractalwonder-ui/src/components/ui.rs` - Integrate linear progress bar
