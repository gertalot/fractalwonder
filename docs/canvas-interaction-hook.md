# Canvas Interaction Hook

## Overview

`use_canvas_interaction` is a generic Leptos hook for adding real-time pan/zoom interaction to HTML canvas elements. It's designed for applications where full canvas re-renders are expensive (seconds to hours), such as:

- High-precision fractal rendering
- Large geographic maps
- Complex scientific visualizations
- Game worlds with procedural generation

## How It Works

1. **Interaction Start**: User drags (pointerdown) or zooms (wheel)
2. **ImageData Capture**: Hook snapshots current canvas ImageData
3. **Real-time Preview**: Animation loop applies pixel transformations (translate/scale) to the snapshot
4. **Debounced End**: After 1.5s of inactivity, fires callback with final transformation
5. **Consumer Re-render**: Callback converts pixel transform to domain coordinates and triggers full re-render

## API

### `use_canvas_interaction(canvas_ref, on_interaction_end)`

**Parameters:**
- `canvas_ref: NodeRef<leptos::html::Canvas>` - Reference to canvas element
- `on_interaction_end: Fn(TransformResult)` - Callback fired when interaction ends

**Returns:** `InteractionHandle` with:
- `is_interacting: Signal<bool>` - Reactive interaction state
- Event handlers: `on_pointer_down`, `on_pointer_move`, `on_pointer_up`, `on_wheel`
- `reset: Fn()` - Clear all interaction state

### `TransformResult`

```rust
pub struct TransformResult {
    pub offset_x: f64,        // Pixels dragged horizontally
    pub offset_y: f64,        // Pixels dragged vertically
    pub zoom_factor: f64,     // Cumulative zoom (1.0 = no zoom)
    pub matrix: [[f64; 3]; 3], // 2D affine transformation matrix
}
```

## Usage Example

```rust
use fractalwonder::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};

#[component]
pub fn MyCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    let handle = use_canvas_interaction(
        canvas_ref,
        move |result: TransformResult| {
            // Convert pixel coordinates to your domain
            let domain_offset = pixel_to_domain(result.offset_x, result.offset_y);

            // Trigger expensive re-render
            render_full_quality(domain_offset, result.zoom_factor);
        },
    );

    view! {
        <canvas
            node_ref=canvas_ref
            width="800"
            height="600"
            on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
            on:pointermove=move |ev| (handle.on_pointer_move)(ev)
            on:pointerup=move |ev| (handle.on_pointer_up)(ev)
            on:wheel=move |ev| (handle.on_wheel)(ev)
            style="cursor: grab; touch-action: none;"
        />
        <Show when=move || handle.is_interacting.get()>
            <div class="interaction-indicator">"Navigating..."</div>
        </Show>
    }
}
```

## Implementation Details

- **Coordinate System**: Works purely in pixel space (no domain knowledge)
- **ImageData Memory**: Stores snapshot only during active interaction
- **Animation Loop**: Uses requestAnimationFrame for 60fps preview
- **Zoom Center**: Zoom is centered on current pointer position
- **Timeout**: 1.5s debounce, restarts on any new pointer/wheel event
- **State Management**: Leptos signals for reactive state, StoredValue for internal bookkeeping

## Testing

Unit tests for matrix math:
```bash
cargo test build_transform_matrix
```

Browser integration tests:
```bash
cargo test --target wasm32-unknown-unknown
```

Manual testing:
```bash
trunk serve
# Navigate to http://localhost:8080
# Test drag, zoom, verify console logs
```

## Architecture Notes

This hook is **framework-agnostic in design** but **Leptos-specific in implementation**. All transformation logic operates in pixel space with no assumptions about the underlying data (fractals, maps, etc.), making it suitable for any canvas-based application where re-renders are expensive.
