# Gradient Editor Bug Fix Session

## Context

A gradient editor component was implemented in `/fractalwonder-ui/src/components/gradient_editor.rs` but has multiple serious bugs that need fixing. This document provides all context needed to debug and fix these issues.

## Current Bugs (User-Reported)

1. **Color stop boxes don't move when dragging** - The squares representing color stops should visually follow the mouse during drag, but they don't move.

2. **Clicking gradient creates midpoint instead of color stop** - Clicking on the gradient bar should add a new color stop (square), but appears to create a midpoint (diamond) instead.

3. **Garbage text in zoom controls** - Instead of just showing "Zoom: 2.0x", the UI shows: `= 10.0 on : click = move | _ | zoom.update(|z| *z = (*z * 1.2).min(10.0)) >` - This is Leptos view macro code leaking as visible text.

4. **Midpoint diamonds don't move when dragging** - The diamond shapes should visually follow the mouse during drag.

5. **Midpoint dragging creates hard edges** - When dragging midpoints, something goes wrong with the gradient interpolation, creating hard color transitions instead of smooth blending.

## Architecture Overview

**Tech Stack:**
- Rust compiled to WASM
- Leptos 0.6 (reactive UI framework)
- HTML5 Canvas for gradient rendering
- OKLAB color space for perceptually uniform gradients

**Key Files:**
- `fractalwonder-ui/src/components/gradient_editor.rs` - Main component (480 lines)
- `fractalwonder-ui/src/components/palette_editor.rs` - Parent component that hosts GradientEditor
- `fractalwonder-ui/src/rendering/colorizers/gradient.rs` - Gradient data structure with `to_preview_lut()`

**Data Structures:**
```rust
// In gradient.rs
pub struct ColorStop {
    pub position: f64,  // 0.0 to 1.0
    pub color: [u8; 3], // RGB
}

pub struct Gradient {
    pub stops: Vec<ColorStop>,
    pub midpoints: Vec<f64>,  // One between each pair of stops, 0.0-1.0 (0.5 = centered)
}
```

## How the Component Should Work

### Color Stops (Squares)
- Rendered as 12x12px squares above the gradient bar
- Positioned at `left: {position * 100}%`
- **Dragging**: On mousedown, track which stop is being dragged. On mousemove, update that stop's position. On mouseup, sort stops and call `on_change`.
- **Visual feedback**: The square should move in real-time during drag.

### Midpoints (Diamonds)
- Rendered as 10x10px diamonds (rotated squares) between each pair of color stops
- Position formula: `left_stop_pos + (right_stop_pos - left_stop_pos) * midpoint_value`
- Midpoint value ranges 0.05-0.95 (clamped to prevent extremes)
- **Dragging**: Similar to stops, but updates `gradient.midpoints[index]` instead

### Canvas Gradient Bar
- 320x32px canvas rendered with OKLAB interpolation via `gradient.to_preview_lut(width)`
- Clicking should add a NEW COLOR STOP (not a midpoint) at that position
- The new stop's color should be sampled from the gradient at that position

### Zoom Controls
- Two buttons: ZoomOutIcon (-) and ZoomInIcon (+)
- Text between icons shows "Zoom: X.Xx" only when zoom > 1.0
- The text showing code instead of the zoom value is a Leptos view macro parsing issue

## Suspected Root Causes

### Bug 1 & 4 (Stops/midpoints don't move visually):
The `<For>` loop captures `position` and `display_pos` at render time. When the gradient signal updates, the loop doesn't re-render because the key doesn't change. The position values are stale.

**Possible fix**: Make position reactive by reading from the signal inside the style closure, not from captured variables.

### Bug 2 (Click creates midpoint):
The click handler `handle_bar_click` adds to `grad.stops`, but the visual result suggests stops aren't being added or rendered. Check if the midpoint count is being incorrectly updated or if there's a rendering issue.

### Bug 3 (Garbage text):
This is a Leptos view macro parsing issue. The button attributes around lines 248-253 may have incorrect syntax. Check for:
- Missing braces around closures
- Incorrect attribute syntax
- Comments inside view! macro that break parsing

### Bug 5 (Hard edges from midpoint drag):
The midpoint value calculation or the gradient interpolation might be wrong. Check:
- `apply_midpoint_bias()` in gradient.rs
- Whether midpoint values are being set correctly (0.0-1.0 range, with 0.5 = linear)

## Reference: Leptos Patterns

**Reactive For loop with dynamic styles:**
```rust
<For
    each=move || some_signal.get().into_iter().enumerate().collect::<Vec<_>>()
    key=|(i, _)| *i
    children=move |(index, _)| {
        // Read position from signal INSIDE the closure for reactivity
        let position = move || {
            some_signal.get()
                .and_then(|data| data.get(index))
                .map(|item| item.position)
                .unwrap_or(0.0)
        };

        view! {
            <div style=move || format!("left: {}%", position() * 100.0) />
        }
    }
/>
```

**Button with event handler:**
```rust
<button
    class="..."
    prop:disabled=move || some_condition()
    on:click=move |_| do_something()
>
    <Icon />
</button>
```

## Testing Commands

```bash
# Run in project root
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check -p fractalwonder-ui
cargo test --workspace

# Visual testing: trunk serve should already be running on localhost:8080
# Open palette editor to test gradient editor
```

## Files to Read

1. `fractalwonder-ui/src/components/gradient_editor.rs` - The buggy component
2. `fractalwonder-ui/src/rendering/colorizers/gradient.rs` - Gradient struct and `apply_midpoint_bias()`
3. `docs/ux-palette-editor/src/components/GradientEditor.tsx` - Reference TypeScript prototype that works correctly

## Expected Behavior (from prototype)

The TypeScript prototype at `docs/ux-palette-editor/src/components/GradientEditor.tsx` shows the correct behavior:
- Color stops are draggable squares that move in real-time
- Midpoints are draggable diamonds that move in real-time
- Clicking the gradient bar adds a new color stop with sampled color
- Zoom controls show clean "Zoom: X.Xx" text
- Midpoints adjust blend smoothly without hard edges

## Task

Debug and fix all 5 issues listed above. The fixes should:
1. Make color stop squares move visually during drag
2. Make clicking the gradient bar add color stops (not midpoints)
3. Fix the zoom text to only show "Zoom: X.Xx" when zoomed
4. Make midpoint diamonds move visually during drag
5. Fix midpoint interpolation to produce smooth gradients

After fixing, run all quality checks and commit with descriptive message.
